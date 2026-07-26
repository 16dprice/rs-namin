//! egui-based viewer UI.
//!
//! M1.1 spike scope (see docs/gui_plan.md): the HUD is an egui window with
//! interactive transport controls, proving the egui-macroquad stack and the
//! input-gating seam.
//!
//! Frame protocol: call [`layout`] once per frame BEFORE scene input handling
//! (it runs egui's input+layout pass and reports what egui captured), wrap the
//! frame's `InputProvider` in `UiGatedInput` with the returned flags, then
//! call [`draw`] AFTER all macroquad drawing so the UI paints on top.

use egui_macroquad::egui;

use crate::camera::Camera;
use crate::clock::{Clock, LoopMode, PlaybackState};
use crate::debug::DebugOverlay;
use crate::scene::Scene;

/// Which input domains egui captured this frame. Feed into
/// [`crate::input::UiGatedInput`] so scene controls ignore captured input.
pub struct UiCapture {
    pub pointer: bool,
    pub keyboard: bool,
}

/// Run the egui input+layout pass for the viewer.
pub fn layout(overlay: &mut DebugOverlay, clock: &mut Clock, scene: &Scene, camera: &Camera) -> UiCapture {
    let mut capture = UiCapture {
        pointer: false,
        keyboard: false,
    };
    egui_macroquad::ui(|ctx| {
        if overlay.hud_visible {
            hud_window(ctx, overlay, clock, scene, camera);
        }
        capture.pointer = ctx.wants_pointer_input();
        capture.keyboard = ctx.wants_keyboard_input();
    });
    capture
}

/// Paint the egui frame. Call after all macroquad drawing for the frame.
pub fn draw() {
    egui_macroquad::draw();
}

fn hud_window(ctx: &egui::Context, overlay: &mut DebugOverlay, clock: &mut Clock, scene: &Scene, camera: &Camera) {
    egui::Window::new("rs-namin")
        .default_pos([10.0, 10.0])
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
