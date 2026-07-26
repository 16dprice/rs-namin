use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::clock::{self, Clock};
use crate::debug::{DebugOverlay, SnapView};
use crate::input::{MacroquadInput, UiGatedInput};
use crate::render_util;
use crate::scene::Scene;
use crate::ui;

/// Parse `RS_NAMIN_FRAME_DUMP="path.png@N"`: capture frame N to path, then exit.
/// Dev/agent utility — the only way to visually verify the live viewer
/// (including UI chrome) without a human at the window.
fn frame_dump_spec() -> Option<(String, u32)> {
    let spec = std::env::var("RS_NAMIN_FRAME_DUMP").ok()?;
    let (path, frame) = spec.rsplit_once('@')?;
    Some((path.to_string(), frame.parse().ok()?))
}

/// Interactive viewer loop: orbit controls, debug overlay, scrub bar.
/// Call from an async macroquad context.
pub async fn run(mut scene: Scene, timeline: Timeline, mut camera: Camera) {
    let initial_camera = camera.clone();

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();
    let mut orbit = OrbitController::from_camera(&camera);

    let frame_dump = frame_dump_spec();
    let mut frame_index: u32 = 0;

    loop {
        clear_background(BLACK);

        // egui input+layout pass first: it decides whether it captures this
        // frame's pointer/keyboard, and scene input is gated accordingly.
        let capture = ui::layout(&mut debug_overlay, &mut clock, &scene, &camera);
        let raw_input = MacroquadInput;
        let input = UiGatedInput::new(&raw_input, capture.pointer, capture.keyboard);

        let snap = debug_overlay.handle_input(&mut clock, &input);
        debug_overlay.update(&mut clock);

        match snap {
            SnapView::Front => orbit.snap_front(),
            SnapView::Right => orbit.snap_right(),
            SnapView::Top => orbit.snap_top(),
            SnapView::None => {}
        }

        clock.tick(get_frame_time());

        if debug_overlay.camera_follow_timeline {
            camera = initial_camera.clone();
            timeline.apply(clock.current_time, &mut scene, &mut camera);
        } else {
            timeline.apply_scene_only(clock.current_time, &mut scene);
        }

        // 3D scene pass
        set_camera(&camera.to_macroquad());
        debug_overlay.draw_world(&orbit, &scene);
        scene.draw_world();

        // Screen-space scene pass: design-space coordinates, so screen-space
        // objects match their size/position in exports (WYSIWYG).
        set_camera(&render_util::screen_space_camera(None));
        scene.draw_screen();

        // Debug overlay uses real window pixels.
        set_default_camera();
        debug_overlay.draw(&clock, &scene, &camera, &input);
        debug_overlay.scrub_bar.draw_ticks(&timeline, clock.duration);

        // egui paint pass — last, so the UI draws on top of everything.
        ui::draw();

        if let Some((path, target_frame)) = &frame_dump {
            if frame_index == *target_frame {
                get_screen_data().export_png(path);
                eprintln!("Frame dump saved: {path}");
                return;
            }
            frame_index += 1;
        }

        // Orbit controller runs last so it doesn't consume input before UI.
        if debug_overlay.camera_follow_timeline {
            orbit = OrbitController::from_camera(&camera);
        } else {
            orbit.update(&mut camera, &input);
        }

        debug_overlay.record_camera(&camera, clock.current_time);

        next_frame().await;
    }
}
