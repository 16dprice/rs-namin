use std::path::PathBuf;

use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::clock::{self, Clock};
use crate::debug::{DebugOverlay, SnapView};
use crate::doc::SceneDoc;
use crate::editor::EditorState;
use crate::input::{InputProvider, MacroquadInput, UiGatedInput};
use crate::registry::{SceneEntry, SceneSource};
use crate::render_util::{self, OffscreenRenderer};
use crate::scene::Scene;
use crate::ui::{self, TransportState, UiRequest};

/// How many frames a transient status message stays in the app bar.
const STATUS_FRAMES: u32 = 240;

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

        match snap {
            SnapView::Front => self.orbit.snap_front(),
            SnapView::Right => self.orbit.snap_right(),
            SnapView::Top => self.orbit.snap_top(),
            SnapView::None => {}
        }

        self.clock.tick(get_frame_time());

        if self.debug_overlay.camera_follow_timeline {
            self.camera = self.initial_camera.clone();
            self.timeline.apply(self.clock.current_time, &mut self.scene, &mut self.camera);
        } else {
            self.timeline.apply_scene_only(self.clock.current_time, &mut self.scene);
        }

        // 3D scene pass
        set_camera(&self.camera.to_macroquad());
        self.debug_overlay.draw_world(&self.orbit, &self.scene);
        self.scene.draw_world();

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
