//! egui-based application UI.
//!
//! Frame protocol: each mode calls its `*_layout` function once per frame
//! BEFORE scene input handling (it runs egui's input+layout pass and reports
//! what egui captured), wraps the frame's `InputProvider` in `UiGatedInput`
//! with the returned flags, then calls [`draw`] AFTER all macroquad drawing
//! so the UI paints on top. See docs/module_layout.md.

use egui_macroquad::egui;

use crate::camera::Camera;
use crate::clock::{Clock, LoopMode, PlaybackState};
use crate::debug::DebugOverlay;
use crate::registry::{self, SceneEntry, SceneKind};
use crate::scene::Scene;

/// Which input domains egui captured this frame. Feed into
/// [`crate::input::UiGatedInput`] so scene controls ignore captured input.
pub struct UiCapture {
    pub pointer: bool,
    pub keyboard: bool,
}

/// App-level navigation requested by this frame's UI.
#[derive(Clone, Copy)]
pub enum UiRequest {
    None,
    OpenLibrary,
    OpenScene(&'static SceneEntry),
}

/// Run the egui input+layout pass for the viewer mode: top app bar plus the
/// HUD window (toggled by F1 via `overlay.hud_visible`).
pub fn viewer_layout(
    overlay: &mut DebugOverlay,
    clock: &mut Clock,
    scene: &Scene,
    camera: &Camera,
    scene_name: &str,
) -> (UiCapture, UiRequest) {
    let mut capture = UiCapture {
        pointer: false,
        keyboard: false,
    };
    let mut request = UiRequest::None;

    egui_macroquad::ui(|ctx| {
        egui::TopBottomPanel::top("app_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("< Library").clicked() {
                    request = UiRequest::OpenLibrary;
                }
                ui.separator();
                ui.strong(scene_name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("Esc: library · F1: HUD");
                });
            });
        });

        if overlay.hud_visible {
            hud_window(ctx, overlay, clock, scene, camera);
        }

        capture.pointer = ctx.wants_pointer_input();
        capture.keyboard = ctx.wants_keyboard_input();
    });

    (capture, request)
}

/// Run the egui input+layout pass for the library mode: the full-screen
/// scene list over `registry::SCENES`.
pub fn library_layout() -> (UiCapture, UiRequest) {
    let mut capture = UiCapture {
        pointer: false,
        keyboard: false,
    };
    let mut request = UiRequest::None;

    egui_macroquad::ui(|ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.heading("rs-namin");
            ui.weak("Pick a scene to open it in the viewer.");
            ui.add_space(8.0);
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("scene_list")
                    .num_columns(3)
                    .spacing([18.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for entry in registry::SCENES {
                            if ui.button(entry.name).clicked() {
                                request = UiRequest::OpenScene(entry);
                            }
                            ui.weak(kind_label(entry.kind));
                            ui.label(entry.description);
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
    }
}

/// Paint the egui frame. Call after all macroquad drawing for the frame.
pub fn draw() {
    egui_macroquad::draw();
}

fn hud_window(ctx: &egui::Context, overlay: &mut DebugOverlay, clock: &mut Clock, scene: &Scene, camera: &Camera) {
    egui::Window::new("rs-namin")
        .default_pos([10.0, 40.0])
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Time: {:.2} / {:.2} s", clock.current_time, clock.duration));

            ui.horizontal(|ui| {
                let label = match clock.playback_state {
                    PlaybackState::Playing => "Pause",
                    PlaybackState::Paused => "Play",
                };
                if ui.button(label).clicked() {
                    clock.toggle();
                }
                let loop_str = match clock.loop_mode {
                    LoopMode::Once => "Once",
                    LoopMode::Loop => "Loop",
                    LoopMode::PingPong => "PingPong",
                };
                ui.label(format!("Loop: {loop_str}"));
            });

            let mut speed = clock.playback_speed;
            ui.add(egui::Slider::new(&mut speed, 0.125..=8.0).logarithmic(true).text("Speed"));
            if speed != clock.playback_speed {
                clock.set_speed(speed);
            }

            ui.checkbox(&mut overlay.camera_follow_timeline, "Camera follows timeline (F5)");

            ui.separator();

            let p = camera.position;
            let t = camera.target;
            let fwd = camera.forward();
            ui.label(format!(
                "Cam: ({:.1}, {:.1}, {:.1})  Target: ({:.1}, {:.1}, {:.1})",
                p.x, p.y, p.z, t.x, t.y, t.z
            ));
            ui.label(format!(
                "Fwd: ({:.2}, {:.2}, {:.2})  Dist: {:.1}  FOV: {:.0}",
                fwd.x,
                fwd.y,
                fwd.z,
                camera.distance(),
                camera.fov
            ));
            ui.label(format!("Objects: {}", scene.len()));
        });
}
