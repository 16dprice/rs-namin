use macroquad::prelude::*;

use rs_namin::camera::orbit::OrbitController;
use rs_namin::camera::Camera;
use rs_namin::clock::{self, Clock};
use rs_namin::debug::DebugOverlay;
use rs_namin::demo;

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
    let (mut scene, timeline) = demo::build();

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();

    let mut camera = Camera::new(vec3(0.0, 4.0, 15.0), vec3(0.0, 3.0, 0.0));
    let mut orbit = OrbitController::from_camera(&camera);

    loop {
        clear_background(BLACK);

        debug_overlay.handle_input(&mut clock);
        debug_overlay.update(&mut clock);

        clock.tick(get_frame_time());
        timeline.apply(clock.current_time, &mut scene);

        // 3D scene pass
        set_camera(&camera.to_macroquad());
        debug_overlay.draw_world();
        scene.draw_all();

        // Screen-space UI pass
        set_default_camera();
        debug_overlay.draw(&clock, &scene, &camera);
        debug_overlay.scrub_bar.draw_ticks(&timeline, clock.duration);

        // Orbit controller runs last so it doesn't consume input before UI
        orbit.update(&mut camera);

        next_frame().await;
    }
}
