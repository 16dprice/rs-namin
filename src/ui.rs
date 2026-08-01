//! egui-based application UI.
//!
//! Frame protocol: each mode calls its `*_layout` function once per frame
//! BEFORE scene input handling (it runs egui's input+layout pass and reports
//! what egui captured), wraps the frame's `InputProvider` in `UiGatedInput`
//! with the returned flags, then calls [`draw`] AFTER all macroquad drawing
//! so the UI paints on top. See docs/module_layout.md.

use std::fmt::Write;

use egui_macroquad::egui;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::clock::{Clock, LoopMode, PlaybackState};
use crate::debug::DebugOverlay;
use crate::editor::{self, EditorState};
use crate::export::{ExportForm, ExportPhase, ExportUiEvent, RESOLUTION_PRESETS, recommended_bitrate};
use crate::registry::{self, SceneEntry, SceneKind};
use crate::scene::Scene;
use crate::scene::value::AnimValue;

/// Which input domains egui captured this frame. Feed into
/// [`crate::input::UiGatedInput`] so scene controls ignore captured input.
pub struct UiCapture {
    pub pointer: bool,
    pub keyboard: bool,
    /// The pointer is over any egui panel/window/popup this frame. Broader
    /// than `pointer`: egui's `wants_pointer_input` goes *false* on the
    /// frame a press lands on non-interactive panel space (its `!any_down`
    /// clause), which is exactly when a click must NOT fall through to
    /// scene selection (e.g. clicking panel background to dismiss a color
    /// popup used to deselect the object). Gate selection clicks on this;
    /// keep drags gated on `pointer` so an orbit that crosses a panel
    /// doesn't stall.
    pub pointer_over_ui: bool,
}

/// App-level navigation requested by this frame's UI.
#[derive(Clone, Copy)]
pub enum UiRequest {
    None,
    OpenLibrary,
    OpenScene(&'static SceneEntry),
    OpenExport(&'static SceneEntry),
    /// Create a new scene document and open it in the viewer/editor.
    NewScene,
    /// Delete a scene document's file (Doc entries only, after confirm).
    DeleteScene(&'static SceneEntry),
}

/// Persistent transport-bar state across frames.
#[derive(Default)]
pub struct TransportState {
    resume_after_scrub: bool,
}

/// Apply one frame of scrub-slider interaction to the clock: pause while
/// dragging, seek to the dragged time, and resume playback afterwards only
/// if it was playing when the drag started. Pure logic, kept separate from
/// egui so it stays unit-testable.
pub fn apply_scrub(state: &mut TransportState, clock: &mut Clock, drag_started: bool, scrub_to: Option<f32>, drag_stopped: bool) {
    if drag_started {
        state.resume_after_scrub = clock.playback_state == PlaybackState::Playing;
        clock.pause();
    }
    if let Some(t) = scrub_to {
        clock.scrub(t);
    }
    if drag_stopped {
        if state.resume_after_scrub {
            clock.play();
        }
        state.resume_after_scrub = false;
    }
}

/// Everything the viewer-mode UI reads and mutates for one frame.
pub struct ViewerUi<'a> {
    pub overlay: &'a mut DebugOverlay,
    pub transport: &'a mut TransportState,
    pub clock: &'a mut Clock,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub timeline: &'a Timeline,
    pub scene_name: &'a str,
    /// Transient message shown in the app bar (e.g. "Saved snapshots/…").
    pub status: Option<&'a str>,
    /// Present when the open scene is an editable document.
    pub editor: Option<&'a mut EditorState>,
}

pub struct ViewerUiResponse {
    pub capture: UiCapture,
    pub request: UiRequest,
    /// The Snapshot button was clicked this frame.
    pub snapshot: bool,
    /// The Export button was clicked this frame.
    pub export: bool,
}

/// Run the egui input+layout pass for the viewer mode: app bar, transport
/// bar (F2), camera status strip (F1), and value inspector (F3).
pub fn viewer_layout(args: ViewerUi) -> ViewerUiResponse {
    let ViewerUi {
        overlay,
        transport,
        clock,
        scene,
        camera,
        timeline,
        scene_name,
        status,
        mut editor,
    } = args;

    let mut response = ViewerUiResponse {
        capture: UiCapture {
            pointer: false,
            keyboard: false,
            pointer_over_ui: false,
        },
        request: UiRequest::None,
        snapshot: false,
        export: false,
    };

    egui_macroquad::ui(|ctx| {
        egui::TopBottomPanel::top("app_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("< Library").clicked() {
                    response.request = UiRequest::OpenLibrary;
                }
                ui.separator();
                ui.strong(scene_name);
                ui.separator();
                if ui
                    .button("Snapshot")
                    .on_hover_text("Save the current frame as a PNG (scene only, no UI)")
                    .clicked()
                {
                    response.snapshot = true;
                }
                if ui.button("Export...").on_hover_text("Render this scene to MP4").clicked() {
                    response.export = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match status {
                    Some(message) => {
                        ui.weak(message);
                    }
                    None => {
                        ui.weak("Esc: library · F1: HUD · F2: transport · F3: inspector");
                    }
                });
            });
        });

        // Camera strip first so it sits at the very bottom; the transport
        // stacks above it, leaving the transport's top edge bordering the
        // viewport — that edge is its resize handle, and a panel docked
        // flush against it would sit in the hover zone and steal the drag.
        if overlay.hud_visible {
            camera_panel(ctx, overlay, scene, camera);
        }
        if overlay.transport_visible {
            transport_panel(ctx, transport, clock, timeline, editor.as_deref_mut());
        }
        if let Some(editor) = editor.as_mut() {
            editor::panels(ctx, editor, clock.current_time);
        }
        if overlay.inspector_visible {
            inspector_window(ctx, scene);
        }

        response.capture.pointer = ctx.wants_pointer_input();
        response.capture.keyboard = ctx.wants_keyboard_input();
        response.capture.pointer_over_ui = ctx.is_pointer_over_area();
    });

    response
}

fn transport_panel(
    ctx: &egui::Context,
    transport: &mut TransportState,
    clock: &mut Clock,
    timeline: &Timeline,
    editor: Option<&mut EditorState>,
) {
    // With an editor (dope sheet) the panel is user-resizable; its height
    // persists in egui memory for the app's lifetime, keyed by the panel id,
    // so it survives scene switches. The plain transport keeps a separate id
    // so it stays snug instead of inheriting the editor height.
    let panel = if editor.is_some() {
        egui::TopBottomPanel::bottom("transport_editor")
            .resizable(true)
            .default_height(250.0)
            .height_range(140.0..=640.0)
    } else {
        egui::TopBottomPanel::bottom("transport")
    };
    panel.show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let play_label = match clock.playback_state {
                PlaybackState::Playing => "Pause",
                PlaybackState::Paused => "Play",
            };
            if ui.add_sized([52.0, 20.0], egui::Button::new(play_label)).clicked() {
                clock.toggle();
            }
            if ui.button("<").on_hover_text("Step one frame back (Left)").clicked() {
                clock.pause();
                clock.step_backward();
            }
            if ui.button(">").on_hover_text("Step one frame forward (Right)").clicked() {
                clock.pause();
                clock.step_forward();
            }

            ui.separator();

            egui::ComboBox::from_id_salt("loop_mode")
                .selected_text(loop_label(clock.loop_mode))
                .width(90.0)
                .show_ui(ui, |ui| {
                    for mode in [LoopMode::Once, LoopMode::Loop, LoopMode::PingPong] {
                        ui.selectable_value(&mut clock.loop_mode, mode, loop_label(mode));
                    }
                });

            ui.separator();

            let mut speed = clock.playback_speed;
            ui.spacing_mut().slider_width = 90.0;
            ui.add(egui::Slider::new(&mut speed, 0.125..=8.0).logarithmic(true).text("Speed"));
            if speed != clock.playback_speed {
                clock.set_speed(speed);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.monospace(format!("{:.2} / {:.2} s", clock.current_time, clock.duration));
            });
        });

        // Full-width scrub slider with keyframe ticks.
        let mut time = clock.current_time;
        ui.spacing_mut().slider_width = ui.available_width() - 16.0;
        let slider = ui.add(egui::Slider::new(&mut time, 0.0..=clock.duration.max(f32::EPSILON)).show_value(false));

        if clock.duration > 0.0 {
            let rect = slider.rect;
            let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(48));
            for track in &timeline.tracks {
                for kf_time in track.keyframe_times() {
                    let x = egui::lerp(rect.left()..=rect.right(), (kf_time / clock.duration).clamp(0.0, 1.0));
                    ui.painter().vline(x, rect.y_range(), stroke);
                }
            }
        }

        apply_scrub(
            transport,
            clock,
            slider.drag_started(),
            slider.changed().then_some(time),
            slider.drag_stopped(),
        );

        // Dope sheet: track lanes with editable keyframes (documents only).
        if let Some(editor) = editor {
            editor::dope_sheet(ui, editor, clock, transport);
        }
        ui.add_space(4.0);
    });
}

fn loop_label(mode: LoopMode) -> &'static str {
    match mode {
        LoopMode::Once => "Once",
        LoopMode::Loop => "Loop",
        LoopMode::PingPong => "PingPong",
    }
}

/// Run the egui input+layout pass for the library mode: the full-screen
/// scene list over the registry. `pending_delete` holds the name of the doc
/// whose delete button is armed (two-click confirm).
pub fn library_layout(pending_delete: &mut Option<&'static str>) -> (UiCapture, UiRequest) {
    let mut capture = UiCapture {
        pointer: false,
        keyboard: false,
        pointer_over_ui: false,
    };
    let mut request = UiRequest::None;

    egui_macroquad::ui(|ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("rs-namin");
                if ui
                    .button("+ New scene")
                    .on_hover_text("Create an editable scene document in scenes/")
                    .clicked()
                {
                    request = UiRequest::NewScene;
                }
            });
            ui.weak("Pick a scene to open it in the viewer. Documents (doc) open with the editor.");
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("scene_list")
                    .num_columns(4)
                    .spacing([18.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in registry::scenes() {
                            if ui.button(entry.name).clicked() {
                                *pending_delete = None;
                                request = UiRequest::OpenScene(entry);
                            }
                            ui.weak(kind_label(entry.kind));
                            ui.label(entry.description);
                            if matches!(entry.source, crate::registry::SceneSource::Doc(_)) {
                                if *pending_delete == Some(entry.name) {
                                    if ui
                                        .button(egui::RichText::new("Confirm delete?").color(egui::Color32::LIGHT_RED))
                                        .on_hover_text("Permanently deletes the .ron file")
                                        .clicked()
                                    {
                                        *pending_delete = None;
                                        request = UiRequest::DeleteScene(entry);
                                    }
                                } else if ui
                                    .small_button("Delete")
                                    .on_hover_text("Delete this scene document's file")
                                    .clicked()
                                {
                                    *pending_delete = Some(entry.name);
                                }
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
            });
        });

        capture.pointer = ctx.wants_pointer_input();
        capture.keyboard = ctx.wants_keyboard_input();
    });

    (capture, request)
}

fn kind_label(kind: SceneKind) -> &'static str {
    match kind {
        SceneKind::Example => "example",
        SceneKind::Video => "video",
        SceneKind::Scratch => "scratch",
        SceneKind::Doc => "doc",
    }
}

/// Run the egui input+layout pass for the export mode: app bar plus the
/// config/progress side panel. The scene preview is drawn behind by the mode.
pub fn export_layout(phase: &mut ExportPhase, scene_name: &str, duration: f32, is_doc: bool) -> (UiCapture, ExportUiEvent) {
    let mut capture = UiCapture {
        pointer: false,
        keyboard: false,
        pointer_over_ui: false,
    };
    let mut event = ExportUiEvent::None;
    let rendering = matches!(phase, ExportPhase::Render(_));

    egui_macroquad::ui(|ctx| {
        egui::TopBottomPanel::top("app_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(!rendering, egui::Button::new("< Viewer")).clicked() {
                    event = ExportUiEvent::Back;
                }
                ui.separator();
                ui.strong(format!("Export — {scene_name}"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if rendering {
                        ui.weak("rendering…");
                    } else {
                        ui.weak("Esc: back to viewer");
                    }
                });
            });
        });

        egui::SidePanel::left("export_panel")
            .resizable(false)
            .exact_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                match phase {
                    ExportPhase::Configure(form) => {
                        if let Some(form_event) = export_form(ui, form, duration, is_doc) {
                            event = form_event;
                        }
                    }
                    ExportPhase::Render(job) => {
                        ui.heading("Rendering");
                        ui.add_space(8.0);
                        ui.add(egui::ProgressBar::new(job.progress()).show_percentage());
                        ui.label(format!("Frame {} / {}", job.frames_done(), job.total_frames()));
                        ui.add_space(4.0);
                        ui.monospace(job.output_path());
                        ui.add_space(8.0);
                        if ui.button("Cancel").clicked() {
                            event = ExportUiEvent::Cancel;
                        }
                    }
                    ExportPhase::Done(info) => {
                        ui.heading(if info.success { "Done" } else { "Export failed" });
                        ui.add_space(8.0);
                        let color = if info.success {
                            egui::Color32::LIGHT_GREEN
                        } else {
                            egui::Color32::LIGHT_RED
                        };
                        ui.colored_label(color, &info.message);
                        if info.success {
                            ui.monospace(&info.output_path);
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Back to viewer").clicked() {
                                event = ExportUiEvent::Back;
                            }
                            if ui.button("Export again").clicked() {
                                event = ExportUiEvent::ExportAgain;
                            }
                        });
                    }
                }
            });

        capture.pointer = ctx.wants_pointer_input();
        capture.keyboard = ctx.wants_keyboard_input();
    });

    (capture, event)
}

/// The Configure-phase form. Returns the event a button click requested.
fn export_form(ui: &mut egui::Ui, form: &mut ExportForm, duration: f32, is_doc: bool) -> Option<ExportUiEvent> {
    let mut event = None;

    ui.heading("Export settings");
    ui.add_space(8.0);

    if let Some(notice) = &form.notice {
        ui.colored_label(egui::Color32::LIGHT_YELLOW, notice);
        ui.add_space(4.0);
    }

    egui::Grid::new("export_form").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.weak("Resolution");
        egui::ComboBox::from_id_salt("resolution")
            .selected_text(RESOLUTION_PRESETS[form.resolution_index].to_string())
            .show_ui(ui, |ui| {
                for (i, preset) in RESOLUTION_PRESETS.iter().enumerate() {
                    ui.selectable_value(&mut form.resolution_index, i, preset.to_string());
                }
            });
        ui.end_row();

        ui.weak("Frame rate");
        egui::ComboBox::from_id_salt("fps")
            .selected_text(format!("{} fps", form.fps))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut form.fps, 30, "30 fps");
                ui.selectable_value(&mut form.fps, 60, "60 fps");
            });
        ui.end_row();

        ui.weak("Encoding");
        ui.vertical(|ui| {
            ui.radio_value(&mut form.use_bitrate, false, "CRF (constant quality)");
            if ui.radio_value(&mut form.use_bitrate, true, "Bitrate (YouTube)").clicked() {
                form.kbps = recommended_bitrate(form.resolution().label, form.fps);
            }
            if form.use_bitrate {
                ui.add(egui::DragValue::new(&mut form.kbps).range(500..=100_000).suffix(" kbps"));
            } else {
                ui.add(egui::DragValue::new(&mut form.crf).range(0..=51).prefix("CRF "));
            }
        });
        ui.end_row();

        ui.weak("Range");
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut form.start_time)
                    .range(0.0..=duration)
                    .speed(0.05)
                    .suffix(" s"),
            );
            ui.label("to");
            ui.add(
                egui::DragValue::new(&mut form.end_time)
                    .range(0.0..=duration)
                    .speed(0.05)
                    .suffix(" s"),
            );
        });
        ui.end_row();

        ui.weak("Audio");
        ui.add(egui::TextEdit::singleline(&mut form.audio_path).hint_text("none"));
        ui.end_row();

        ui.weak("Output");
        ui.add(egui::TextEdit::singleline(&mut form.output_path).hint_text("renders/<auto>.mp4"));
        ui.end_row();
    });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui.add_sized([120.0, 28.0], egui::Button::new("Start export")).clicked() {
            event = Some(ExportUiEvent::Start);
        }
        if is_doc
            && ui
                .button("Save as scene defaults")
                .on_hover_text("Store resolution/fps/output in the scene document")
                .clicked()
        {
            event = Some(ExportUiEvent::SaveDefaults);
        }
    });

    event
}

/// Paint the egui frame. Call after all macroquad drawing for the frame.
pub fn draw() {
    egui_macroquad::draw();
}

/// Fixed camera status strip, docked above the transport bar (F1).
fn camera_panel(ctx: &egui::Context, overlay: &mut DebugOverlay, scene: &Scene, camera: &Camera) {
    egui::TopBottomPanel::bottom("camera_hud").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut overlay.camera_follow_timeline, "Camera follows timeline (F5)");
            ui.separator();

            let p = camera.position;
            let t = camera.target;
            let fwd = camera.forward();
            ui.monospace(format!(
                "Cam ({:.1}, {:.1}, {:.1})  Target ({:.1}, {:.1}, {:.1})  Fwd ({:.2}, {:.2}, {:.2})  Dist {:.1}  FOV {:.0}",
                p.x,
                p.y,
                p.z,
                t.x,
                t.y,
                t.z,
                fwd.x,
                fwd.y,
                fwd.z,
                camera.distance(),
                camera.fov
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{} objects", scene.len()));
            });
        });
    });
}

fn inspector_window(ctx: &egui::Context, scene: &Scene) {
    egui::Window::new("Inspector")
        .default_pos([ctx.screen_rect().right() - 300.0, 40.0])
        .default_width(270.0)
        .vscroll(true)
        .show(ctx, |ui| {
            for (id, obj) in scene.iter() {
                egui::CollapsingHeader::new(format!("Object {id:?}"))
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new(format!("props_{id:?}"))
                            .num_columns(2)
                            .spacing([12.0, 2.0])
                            .show(ui, |ui| {
                                for name in obj.property_names() {
                                    ui.weak(*name);
                                    match obj.get(name) {
                                        Some(value) => ui.monospace(format_value(&value)),
                                        None => ui.monospace("???"),
                                    };
                                    ui.end_row();
                                }
                            });
                    });
            }
        });
}

fn format_value(value: &AnimValue) -> String {
    match value {
        AnimValue::Float(f) => format!("{:.2}", f),
        AnimValue::Vec2(v) => format!("({:.1}, {:.1})", v.x, v.y),
        AnimValue::Vec3(v) => format!("({:.1}, {:.1}, {:.1})", v.x, v.y, v.z),
        AnimValue::Vec4(v) => format!("({:.2}, {:.2}, {:.2}, {:.2})", v.x, v.y, v.z, v.w),
        AnimValue::Bool(b) => b.to_string(),
        AnimValue::Transform2D(t) => {
            let mut s = String::new();
            let _ = write!(
                s,
                "pos({:.1},{:.1}) rot={:.1} scl({:.1},{:.1})",
                t.position.x, t.position.y, t.rotation, t.scale.x, t.scale.y
            );
            s
        }
        AnimValue::Mat4(m) => {
            let cols = m.to_cols_array();
            format!("[{:.1},{:.1},{:.1},{:.1}; ...]", cols[0], cols[1], cols[2], cols[3])
        }
    }
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::{vec2, vec3, vec4};

    use super::*;
    use crate::scene::value::Transform2D;

    fn playing_clock() -> Clock {
        let mut clock = Clock::new(10.0, 60.0);
        clock.play();
        clock
    }

    #[test]
    fn scrub_pauses_while_dragging_and_resumes() {
        let mut state = TransportState::default();
        let mut clock = playing_clock();

        apply_scrub(&mut state, &mut clock, true, Some(3.0), false);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
        assert!((clock.current_time - 3.0).abs() < f32::EPSILON);

        apply_scrub(&mut state, &mut clock, false, Some(4.0), false);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
        assert!((clock.current_time - 4.0).abs() < f32::EPSILON);

        apply_scrub(&mut state, &mut clock, false, None, true);
        assert_eq!(clock.playback_state, PlaybackState::Playing);
    }

    #[test]
    fn scrub_stays_paused_if_paused_before_drag() {
        let mut state = TransportState::default();
        let mut clock = Clock::new(10.0, 60.0);

        apply_scrub(&mut state, &mut clock, true, Some(2.0), false);
        apply_scrub(&mut state, &mut clock, false, None, true);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
        assert!((clock.current_time - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scrub_clamps_to_duration() {
        let mut state = TransportState::default();
        let mut clock = playing_clock();
        apply_scrub(&mut state, &mut clock, true, Some(99.0), true);
        assert!((clock.current_time - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn resume_flag_resets_after_drag() {
        let mut state = TransportState::default();
        let mut clock = playing_clock();
        // Drag while playing → resumes.
        apply_scrub(&mut state, &mut clock, true, None, true);
        assert_eq!(clock.playback_state, PlaybackState::Playing);
        clock.pause();
        // Next drag starts from paused → must NOT resume from the stale flag.
        apply_scrub(&mut state, &mut clock, true, None, true);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn format_float() {
        assert_eq!(format_value(&AnimValue::Float(1.23456)), "1.23");
    }

    #[test]
    fn format_vec2() {
        assert_eq!(format_value(&AnimValue::Vec2(vec2(10.0, 20.0))), "(10.0, 20.0)");
    }

    #[test]
    fn format_vec3() {
        assert_eq!(format_value(&AnimValue::Vec3(vec3(1.0, 2.5, 3.0))), "(1.0, 2.5, 3.0)");
    }

    #[test]
    fn format_vec4() {
        assert_eq!(format_value(&AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0))), "(1.00, 0.00, 0.00, 1.00)");
    }

    #[test]
    fn format_bool() {
        assert_eq!(format_value(&AnimValue::Bool(true)), "true");
    }

    #[test]
    fn format_transform2d() {
        let t = AnimValue::Transform2D(Transform2D {
            position: vec2(1.0, 2.0),
            rotation: 45.0,
            scale: vec2(1.0, 1.0),
        });
        assert_eq!(format_value(&t), "pos(1.0,2.0) rot=45.0 scl(1.0,1.0)");
    }
}
