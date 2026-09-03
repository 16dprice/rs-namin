use std::path::PathBuf;

use macroquad::color::hsl_to_rgb;
use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::clock::{self, Clock};
use crate::debug::{DebugOverlay, SnapView};
use crate::doc::{ObjectSpec, SceneDoc};
use crate::editor::EditorState;
use crate::input::{InputProvider, MacroquadInput, UiGatedInput};
use crate::registry::{SceneEntry, SceneSource};
use crate::render_util::{self, OffscreenRenderer};
use crate::scene::Scene;
use crate::scene::value::AnimValue;
use crate::ui::{self, TransportState, UiRequest};

/// How many frames a transient status message stays in the app bar.
const STATUS_FRAMES: u32 = 240;

/// How many frames the "Esc exits" reminder lingers after entering preview.
const PREVIEW_HINT_FRAMES: u32 = 180;

/// Thickness of the rainbow ring outlining the preview's video frame.
const PREVIEW_BORDER: f32 = 4.0;

/// An in-progress viewport drag of a document object.
struct ViewportDrag {
    object_index: usize,
    /// Initial values of the dragged properties (position, or start+end).
    initial: Vec<(&'static str, Vec3)>,
    /// World-space hit point on the drag plane when the drag began.
    initial_hit: Vec2,
    /// The z of the plane the drag moves in.
    plane_z: f32,
    resume_after: bool,
}

/// The interactive viewer as an app mode: orbit controls, debug overlay,
/// egui chrome (app bar, transport, HUD, inspector). One scene per instance;
/// opening another scene from the library constructs a fresh `ViewerMode`.
pub struct ViewerMode {
    entry: &'static SceneEntry,
    scene: Scene,
    timeline: Timeline,
    camera: Camera,
    initial_camera: Camera,
    clock: Clock,
    debug_overlay: DebugOverlay,
    orbit: OrbitController,
    transport: TransportState,
    /// Snapshot rendered last frame, awaiting readback (draw calls flush on
    /// next_frame, so the texture is only readable one frame later).
    pending_snapshot: Option<(OffscreenRenderer, PathBuf)>,
    /// Transient app-bar message and its remaining frame count.
    status: Option<(String, u32)>,
    /// Present when the scene is a document: palette + inspector editing.
    editor: Option<EditorState>,
    viewport_drag: Option<ViewportDrag>,
    /// Chrome-free playback preview: the scene renders through the export
    /// pipeline, letterboxed to the window, with every panel and overlay
    /// hidden. The orbit camera is untouched, so leaving preview restores
    /// the editing view.
    preview: bool,
    /// Offscreen target for preview frames; recreated when the window
    /// letterbox size changes.
    preview_renderer: Option<OffscreenRenderer>,
    preview_hint_frames: u32,
}

impl ViewerMode {
    /// Build the entry's scene and set up playback. Must be called inside the
    /// macroquad window (scene builders may load textures).
    pub fn new(entry: &'static SceneEntry) -> Self {
        let (scene, timeline, camera) = entry.build_or_error_scene();

        // Documents open with the editor attached (doc = source of truth).
        // A doc that fails to parse has nothing to edit; it shows the error
        // scene without an editor.
        let editor = match entry.source {
            SceneSource::Doc(path) => SceneDoc::load(path).ok().map(|doc| EditorState::new(doc, path)),
            SceneSource::Builtin(_) => None,
        };

        let mut clock = Clock::new(timeline.duration(), 60.0);
        clock.loop_mode = clock::LoopMode::Loop;
        clock.play();

        Self {
            entry,
            initial_camera: camera.clone(),
            orbit: OrbitController::from_camera(&camera),
            scene,
            timeline,
            camera,
            clock,
            debug_overlay: DebugOverlay::new(),
            transport: TransportState::default(),
            pending_snapshot: None,
            status: None,
            editor,
            viewport_drag: None,
            // RS_NAMIN_PREVIEW=1 opens straight into preview — lets agents
            // frame-dump the chrome-free render without scripted input.
            preview: std::env::var("RS_NAMIN_PREVIEW").is_ok(),
            preview_renderer: None,
            preview_hint_frames: PREVIEW_HINT_FRAMES,
        }
    }

    pub fn scene_name(&self) -> &'static str {
        self.entry.name
    }

    /// Run one viewer frame (input, playback, all render passes, UI).
    /// Returns any app-level navigation request.
    pub fn frame(&mut self) -> UiRequest {
        // Finish last frame's snapshot: its draw calls have flushed, so the
        // offscreen texture is now readable.
        if let Some((renderer, path)) = self.pending_snapshot.take() {
            self.save_snapshot(&renderer, &path);
        }

        if self.preview {
            return self.preview_frame();
        }

        clear_background(BLACK);

        // egui input+layout pass first: it decides whether it captures this
        // frame's pointer/keyboard, and scene input is gated accordingly.
        let status_text = self.status.as_ref().map(|(message, _)| message.clone());
        let ui_response = ui::viewer_layout(ui::ViewerUi {
            overlay: &mut self.debug_overlay,
            transport: &mut self.transport,
            clock: &mut self.clock,
            scene: &self.scene,
            camera: &self.camera,
            timeline: &self.timeline,
            scene_name: self.entry.name,
            status: status_text.as_deref(),
            editor: self.editor.as_mut(),
        });

        // Rebuild from the document if the editor changed it this frame.
        if self.editor.as_ref().is_some_and(|e| e.rebuild_needed) {
            self.editor.as_mut().unwrap().rebuild_needed = false;
            self.rebuild_from_doc();
        }
        // A rename moved the .ron file: re-resolve the registry entry so the
        // app bar, export, and library all agree on the new name.
        if self.editor.as_ref().is_some_and(|e| e.renamed) {
            let editor = self.editor.as_mut().unwrap();
            editor.renamed = false;
            let name = crate::editor::scene_stem(editor.path).to_string();
            crate::registry::rescan();
            if let Some(entry) = crate::registry::find(&name) {
                self.entry = entry;
            }
        }
        let mut request = ui_response.request;
        if ui_response.export {
            request = UiRequest::OpenExport(self.entry);
        }
        let raw_input = MacroquadInput;
        let input = UiGatedInput::new(&raw_input, ui_response.capture.pointer, ui_response.capture.keyboard);

        let snap = self.debug_overlay.handle_input(&mut self.clock, &input);

        if matches!(request, UiRequest::None) && input.is_key_pressed(KeyCode::Escape) {
            request = UiRequest::OpenLibrary;
        }

        if ui_response.preview || (matches!(request, UiRequest::None) && input.is_key_pressed(KeyCode::P)) {
            self.preview = true;
            self.preview_hint_frames = PREVIEW_HINT_FRAMES;
        }

        match snap {
            SnapView::Front => self.orbit.snap_front(),
            SnapView::Right => self.orbit.snap_right(),
            SnapView::Top => self.orbit.snap_top(),
            SnapView::None => {}
        }

        self.viewport_interact(&input, ui_response.capture.pointer_over_ui);

        self.clock.tick(get_frame_time());

        if self.debug_overlay.camera_follow_timeline {
            self.camera = self.initial_camera.clone();
            self.timeline.apply(self.clock.current_time, &mut self.scene, &mut self.camera);
        } else {
            self.timeline
                .apply_scene_only(self.clock.current_time, &mut self.scene, &self.camera);
        }

        // 3D scene pass
        set_camera(&self.camera.to_macroquad());
        self.debug_overlay.draw_world(&self.orbit, &self.scene);
        self.scene.draw_world();
        self.draw_selection_highlight();

        // Screen-space scene pass: design-space coordinates, so screen-space
        // objects match their size/position in exports (WYSIWYG).
        set_camera(&render_util::screen_space_camera(None));
        self.scene.draw_screen();

        // Debug overlay uses real window pixels.
        set_default_camera();
        self.debug_overlay.draw(&self.camera, &input);

        // Snapshot: render the scene (no UI) into an offscreen target now;
        // readback happens at the start of the next frame.
        if ui_response.snapshot {
            self.begin_snapshot();
            set_default_camera();
        }

        // egui paint pass — last, so the UI draws on top of everything.
        ui::draw();

        // Orbit controller runs last so it doesn't consume input before UI.
        if self.debug_overlay.camera_follow_timeline {
            self.orbit = OrbitController::from_camera(&self.camera);
        } else {
            self.orbit.update(&mut self.camera, &input);
        }

        self.debug_overlay.record_camera(&self.camera, self.clock.current_time);

        if let Some((_, frames_left)) = &mut self.status {
            *frames_left -= 1;
            if *frames_left == 0 {
                self.status = None;
            }
        }

        request
    }

    /// One frame of chrome-free preview: exactly what export renders — the
    /// document camera driven by camera tracks, the offscreen two-pass
    /// renderer — letterboxed to the window. Transport keys stay live;
    /// Esc or P returns to the editor with its orbit view intact.
    fn preview_frame(&mut self) -> UiRequest {
        let input = MacroquadInput;
        if input.is_key_pressed(KeyCode::Escape) || input.is_key_pressed(KeyCode::P) {
            self.preview = false;
            return UiRequest::None;
        }
        crate::debug::transport_keys(&mut self.clock, &self.debug_overlay.keybindings, &input);
        self.clock.tick(get_frame_time());

        let mut camera = self.initial_camera.clone();
        self.timeline.apply(self.clock.current_time, &mut self.scene, &mut camera);

        clear_background(BLACK);
        let fit = render_util::fit_size(
            vec2(screen_width() - 2.0 * PREVIEW_BORDER, screen_height() - 2.0 * PREVIEW_BORDER),
            render_util::DESIGN_WIDTH / render_util::DESIGN_HEIGHT,
        );
        let (w, h) = (fit.x.round().max(1.0) as u32, fit.y.round().max(1.0) as u32);
        if self.preview_renderer.as_ref().is_none_or(|r| r.width != w || r.height != h) {
            self.preview_renderer = Some(OffscreenRenderer::new(w, h));
        }
        let renderer = self.preview_renderer.as_ref().unwrap();
        renderer.render_frame(&self.scene, &camera);
        set_default_camera();
        let dest = Rect::new((screen_width() - fit.x) / 2.0, (screen_height() - fit.y) / 2.0, fit.x, fit.y);
        draw_texture_ex(
            &renderer.target.texture,
            dest.x,
            dest.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dest.w, dest.h)),
                flip_y: true,
                ..Default::default()
            },
        );

        // Rainbow ring marking the video frame's true extent — the scene
        // background is black like the window, so the frame edge would be
        // invisible without it. Wall-clock phase, so it keeps moving while
        // playback is paused.
        let phase = (get_time() * 0.2).fract() as f32;
        for (chunk, pos) in border_chunks(dest, PREVIEW_BORDER, 24.0) {
            let color = hsl_to_rgb((pos + phase).fract(), 1.0, 0.5);
            draw_rectangle(chunk.x, chunk.y, chunk.w, chunk.h, color);
        }

        if self.preview_hint_frames > 0 {
            self.preview_hint_frames -= 1;
            let alpha = self.preview_hint_frames as f32 / PREVIEW_HINT_FRAMES as f32;
            let text = "Esc exits preview";
            let dims = measure_text(text, None, 24, 1.0);
            draw_text(
                text,
                (screen_width() - dims.width) / 2.0,
                screen_height() - 16.0,
                24.0,
                Color::new(1.0, 1.0, 1.0, alpha),
            );
        }
        UiRequest::None
    }

    /// Viewport editing for document scenes: left-click selects (hit-testing
    /// world-object AABBs), left-drag translates the object on its z-plane.
    /// Dragging pauses the clock (edits land at a fixed playhead) and writes
    /// through `EditorState::auto_key` — keyframe when tracked, initial
    /// override otherwise.
    fn viewport_interact(&mut self, input: &dyn InputProvider, pointer_over_ui: bool) {
        let Some(editor) = &mut self.editor else { return };
        // A failed build means the visible scene may not match the doc's
        // object list — indices would lie, so don't interact.
        if editor.error.is_some() {
            self.viewport_drag = None;
            return;
        }

        let screen = vec2(input.screen_width(), input.screen_height());
        let mouse = input.mouse_position();

        // Selection only responds to clean viewport clicks: a press anywhere
        // over egui chrome (panels, popups — even non-interactive panel
        // space, which the `pointer` gate lets through) must never select or
        // deselect. Drag continuation below stays gated on `pointer` only,
        // so an in-progress drag survives crossing a panel.
        if !pointer_over_ui && input.is_mouse_button_pressed(MouseButton::Left) {
            let (origin, dir) = self.camera.screen_ray(mouse, screen);
            let mut best: Option<(usize, f32)> = None;
            for (index, (id, object)) in self.scene.iter().enumerate() {
                // Hidden objects (appear_at in the future) don't steal
                // clicks; select them in the palette, or scrub past their
                // appear time to manipulate them here.
                if object.is_screen_space() || !self.scene.is_visible(id) {
                    continue;
                }
                if let Some(t) = object.bounding_box().ray_intersect(origin, dir)
                    && best.is_none_or(|(_, bt)| t < bt)
                {
                    best = Some((index, t));
                }
            }
            editor.select(best.map(|(index, _)| index));

            if let Some((index, _)) = best
                && let Some(initial) = draggable_properties(&editor.doc.objects[index].object, &self.scene, index)
            {
                // A property bound at the current time is overwritten by its
                // source every frame; dragging would silently fight the
                // binding, so block it. Bindings windowed elsewhere on the
                // timeline don't get in the way.
                let object_id = &editor.doc.objects[index].id;
                let time = self.clock.current_time;
                if let Some(binding_index) = initial.iter().find_map(|(prop, _)| {
                    editor
                        .bindings_for(object_id, prop)
                        .into_iter()
                        .find(|&i| editor.doc.bindings[i].active_at(time))
                }) {
                    let binding = &editor.doc.bindings[binding_index];
                    self.status = Some((
                        format!(
                            "{}.{} is bound to {}.{} — adjust the offset in the inspector",
                            binding.target, binding.property, binding.source, binding.source_property
                        ),
                        STATUS_FRAMES,
                    ));
                    return;
                }
                let plane_z = initial[0].1.z;
                if let Some(hit) = ray_plane_z(origin, dir, plane_z) {
                    self.viewport_drag = Some(ViewportDrag {
                        object_index: index,
                        initial,
                        initial_hit: hit,
                        plane_z,
                        resume_after: self.clock.playback_state == crate::clock::PlaybackState::Playing,
                    });
                    self.clock.pause();
                }
            }
        }

        if let Some(drag) = &self.viewport_drag {
            if input.is_mouse_button_down(MouseButton::Left) {
                let (origin, dir) = self.camera.screen_ray(mouse, screen);
                if let Some(hit) = ray_plane_z(origin, dir, drag.plane_z) {
                    let delta = hit - drag.initial_hit;
                    let time = self.clock.current_time;
                    let object_index = drag.object_index;
                    let updates: Vec<(&'static str, Vec3)> = drag
                        .initial
                        .iter()
                        .map(|(prop, start)| (*prop, *start + vec3(delta.x, delta.y, 0.0)))
                        .collect();
                    for (prop, value) in updates {
                        editor.auto_key(object_index, prop, AnimValue::Vec3(value), time);
                    }
                }
            } else {
                if drag.resume_after {
                    self.clock.play();
                }
                self.viewport_drag = None;
            }
        }
    }

    fn draw_selection_highlight(&self) {
        let Some(editor) = &self.editor else { return };
        let Some(selected) = editor.selected else { return };
        if editor.error.is_some() {
            return;
        }
        if let Some((_, object)) = self.scene.iter().nth(selected)
            && !object.is_screen_space()
        {
            let bb = object.bounding_box();
            crate::debug::draw_aabb(bb.min, bb.max, Color::new(1.0, 0.9, 0.2, 0.9));
        }
    }

    /// Rebuild scene/timeline from the edited document. The current (orbit)
    /// camera is kept; only the initial camera — used by follow mode, export,
    /// and snapshots of the doc — is replaced.
    fn rebuild_from_doc(&mut self) {
        let Some(editor) = &self.editor else { return };
        match editor.doc.build() {
            Ok((scene, timeline, camera)) => {
                self.scene = scene;
                self.timeline = timeline;
                self.initial_camera = camera;
                self.clock.duration = self.timeline.duration();
                self.clock.current_time = self.clock.current_time.min(self.clock.duration);
                self.editor.as_mut().unwrap().error = None;
            }
            Err(error) => {
                self.editor.as_mut().unwrap().error = Some(error);
            }
        }
    }

    fn begin_snapshot(&mut self) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        std::fs::create_dir_all("snapshots").ok();
        let path = PathBuf::from(format!(
            "snapshots/{}_t{:.2}s_{}.png",
            self.entry.name, self.clock.current_time, timestamp
        ));

        let renderer = OffscreenRenderer::new(render_util::DESIGN_WIDTH as u32, render_util::DESIGN_HEIGHT as u32);
        renderer.render_frame(&self.scene, &self.camera);
        self.pending_snapshot = Some((renderer, path));
    }

    fn save_snapshot(&mut self, renderer: &OffscreenRenderer, path: &PathBuf) {
        let mut rgba = Vec::new();
        renderer.read_rgba(&mut rgba);
        match image::save_buffer(path, &rgba, renderer.width, renderer.height, image::ColorType::Rgba8) {
            Ok(()) => {
                self.status = Some((format!("Saved {}", path.display()), STATUS_FRAMES));
                eprintln!("Snapshot saved: {}", path.display());
            }
            Err(e) => {
                self.status = Some((format!("Snapshot failed: {e}"), STATUS_FRAMES));
                eprintln!("Snapshot failed: {e}");
            }
        }
    }
}

/// Border ring around `rect`: chunk rectangles of roughly `chunk_len` px,
/// each paired with its fractional position (0..1) along the perimeter,
/// walking clockwise from the top-left corner. The top and bottom strips
/// extend `thickness` past the rect on both sides to cover the corners.
fn border_chunks(rect: Rect, thickness: f32, chunk_len: f32) -> Vec<(Rect, f32)> {
    let t = thickness;
    let horiz = rect.w + 2.0 * t;
    let total = 2.0 * horiz + 2.0 * rect.h;
    if t <= 0.0 || total <= 0.0 || chunk_len <= 0.0 {
        return Vec::new();
    }
    let left = rect.x - t;
    let right = rect.x + rect.w;
    let top = rect.y - t;
    let bottom = rect.y + rect.h;

    let subdivide = |len: f32| {
        let count = (len / chunk_len).ceil().max(1.0) as usize;
        let step = len / count as f32;
        (0..count).map(move |i| (i as f32 * step, step))
    };

    let mut chunks = Vec::new();
    let mut walked = 0.0;
    for (start, len) in subdivide(horiz) {
        chunks.push((Rect::new(left + start, top, len, t), (walked + start + len / 2.0) / total));
    }
    walked += horiz;
    for (start, len) in subdivide(rect.h) {
        chunks.push((Rect::new(right, rect.y + start, t, len), (walked + start + len / 2.0) / total));
    }
    walked += rect.h;
    for (start, len) in subdivide(horiz) {
        chunks.push((
            Rect::new(right + t - start - len, bottom, len, t),
            (walked + start + len / 2.0) / total,
        ));
    }
    walked += horiz;
    for (start, len) in subdivide(rect.h) {
        chunks.push((Rect::new(left, bottom - start - len, t, len), (walked + start + len / 2.0) / total));
    }
    chunks
}

/// Intersect a ray with the plane z = `plane_z`; returns the XY hit point.
fn ray_plane_z(origin: Vec3, dir: Vec3, plane_z: f32) -> Option<Vec2> {
    if dir.z.abs() < 1e-6 {
        return None;
    }
    let t = (plane_z - origin.z) / dir.z;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    Some(vec2(hit.x, hit.y))
}

/// The translatable Vec3 properties of a doc object, with their current
/// runtime values: `position` for most types, `start`+`end` for Line/Arrow.
/// Screen-space Text is not viewport-draggable.
fn draggable_properties(spec: &ObjectSpec, scene: &Scene, index: usize) -> Option<Vec<(&'static str, Vec3)>> {
    let (_, object) = scene.iter().nth(index)?;
    let get_vec3 = |prop: &str| match object.get(prop) {
        Some(AnimValue::Vec3(v)) => Some(v),
        _ => None,
    };
    match spec {
        ObjectSpec::Text { .. } => None,
        ObjectSpec::Line { .. } | ObjectSpec::Arrow { .. } => Some(vec![("start", get_vec3("start")?), ("end", get_vec3("end")?)]),
        _ => Some(vec![("position", get_vec3("position")?)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The torus builder is GL-free, so ViewerMode constructs headless.
    #[test]
    fn viewer_starts_outside_preview() {
        let viewer = ViewerMode::new(crate::registry::find("torus").unwrap());
        assert!(!viewer.preview);
        assert!(viewer.preview_renderer.is_none());
    }

    #[test]
    fn border_chunks_ring_bounds_and_walk_order() {
        let rect = Rect::new(100.0, 50.0, 640.0, 360.0);
        let chunks = border_chunks(rect, 4.0, 24.0);

        // The ring's bounding box is the rect expanded by the thickness.
        let min_x = chunks.iter().map(|(r, _)| r.x).fold(f32::MAX, f32::min);
        let min_y = chunks.iter().map(|(r, _)| r.y).fold(f32::MAX, f32::min);
        let max_x = chunks.iter().map(|(r, _)| r.x + r.w).fold(f32::MIN, f32::max);
        let max_y = chunks.iter().map(|(r, _)| r.y + r.h).fold(f32::MIN, f32::max);
        assert_eq!((min_x, min_y, max_x, max_y), (96.0, 46.0, 744.0, 414.0));

        // Perimeter positions walk 0..1 strictly forward (continuous rainbow).
        let positions: Vec<f32> = chunks.iter().map(|(_, p)| *p).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]));
        assert!(positions.iter().all(|p| (0.0..1.0).contains(p)));
    }

    #[test]
    fn border_chunks_tile_each_strip_without_gaps() {
        let rect = Rect::new(100.0, 50.0, 640.0, 360.0);
        let chunks = border_chunks(rect, 4.0, 24.0);

        let strip_len = |pred: &dyn Fn(&Rect) -> bool, horizontal: bool| -> f32 {
            chunks
                .iter()
                .filter(|(r, _)| pred(r))
                .map(|(r, _)| if horizontal { r.w } else { r.h })
                .sum()
        };
        // Top/bottom strips cover width + both corners; sides cover the height.
        assert!((strip_len(&|r| r.y < 50.0, true) - 648.0).abs() < 1e-3);
        assert!((strip_len(&|r| r.y >= 410.0, true) - 648.0).abs() < 1e-3);
        assert!((strip_len(&|r| r.x < 100.0 && r.y >= 50.0 && r.y < 410.0, false) - 360.0).abs() < 1e-3);
        assert!((strip_len(&|r| r.x >= 740.0 && r.y >= 50.0 && r.y < 410.0, false) - 360.0).abs() < 1e-3);
    }

    #[test]
    fn border_chunks_zero_thickness_is_empty() {
        assert!(border_chunks(Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 24.0).is_empty());
    }
}
