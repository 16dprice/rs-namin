use macroquad::prelude::{
    BLACK, Conf, clear_background, get_frame_time, next_frame, set_camera, set_default_camera,
};

use rs_namin::camera::orbit::OrbitController;
use rs_namin::clock::{self, Clock};
use rs_namin::debug::{DebugOverlay, SnapView};
use rs_namin::input::MacroquadInput;
use rs_namin::my_scene;

fn window_conf() -> Conf {
    Conf {
        window_title: "rs-namin".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let (mut scene, timeline, mut camera) = my_scene::build();
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

        // Apply snap-to-view if requested
        match snap {
            SnapView::Front => orbit.snap_front(),
            SnapView::Right => orbit.snap_right(),
            SnapView::Top => orbit.snap_top(),
            SnapView::None => {}
        }

        clock.tick(get_frame_time());

        // In timeline camera mode, reset to initial camera then apply all tracks.
        // In orbit mode, only apply scene tracks — orbit controller drives the camera.
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
        debug_overlay
            .scrub_bar
            .draw_ticks(&timeline, clock.duration);

        // Orbit controller runs last so it doesn't consume input before UI.
        if debug_overlay.camera_follow_timeline {
            orbit = OrbitController::from_camera(&camera);
        } else {
            orbit.update(&mut camera, &input);
        }

        // Record camera state for debugging
        debug_overlay.record_camera(&camera, clock.current_time);

        next_frame().await;
    }
}
