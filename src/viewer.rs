use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::clock::{self, Clock};
use crate::debug::{DebugOverlay, SnapView};
use crate::input::MacroquadInput;
use crate::scene::Scene;

/// Interactive viewer loop: orbit controls, debug overlay, scrub bar.
/// Call from an async macroquad context.
pub async fn run(mut scene: Scene, timeline: Timeline, mut camera: Camera) {
    let initial_camera = camera.clone();

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();
    let mut orbit = OrbitController::from_camera(&camera);

    loop {
        clear_background(BLACK);

        let input = MacroquadInput;
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

        // Screen-space UI pass
        set_default_camera();
        scene.draw_screen();
        debug_overlay.draw(&clock, &scene, &camera);
        debug_overlay.scrub_bar.draw_ticks(&timeline, clock.duration);

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
