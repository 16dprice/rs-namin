#![allow(dead_code)]

mod animation;
mod camera;
mod clock;
mod debug;
mod export;
mod scene;
#[cfg(test)]
mod tests;

use macroquad::prelude::*;

use animation::easing::{quad_in, quad_out};
use animation::timeline::Timeline;
use animation::track::{Keyframe, Track};
use camera::orbit::OrbitController;
use camera::Camera;
use clock::Clock;
use debug::DebugOverlay;
use export::Exporter;
use scene::objects::{Circle, Line, Polygon, Rectangle};
use scene::value::AnimValue;
use scene::Scene;

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
    let mut scene = Scene::new();

    let radius = 0.5_f32;
    let rest_y = radius; // ball center when sitting on ground (Y=0)

    let circle_id = scene.add(Circle::new(vec3(0.0, rest_y, 0.0), radius, BLUE));
    // Ground line along the X axis
    scene.add(Line::new(
        vec3(-10.0, 0.0, 0.0),
        vec3(10.0, 0.0, 0.0),
        1.0,
        WHITE,
    ));

    // Static rectangle to the left
    let rect_id = scene.add(Rectangle::new(vec3(-3.0, 1.5, 0.0), vec2(2.0, 3.0), GREEN));

    // Spinning hexagon to the right
    let hex_id = scene.add(Polygon::new(vec3(3.0, 2.0, 0.0), 1.0, 6, YELLOW));

    let mut timeline = Timeline::new();

    // Bouncing ball: 3 bounces with decreasing height, then rest
    // Each bounce: quad_out up (decelerating), quad_in down (accelerating)
    let bounce_heights = [6.0_f32, 3.0, 1.2];
    let bounce_durations = [0.8_f32, 0.6, 0.4]; // seconds per half-bounce

    let mut pos_track = Track::new(circle_id, "position");
    let mut t = 0.0_f32;

    // Start on the ground
    pos_track.add_keyframe(Keyframe::with_easing(
        t,
        AnimValue::Vec3(vec3(0.0, rest_y, 0.0)),
        quad_out,
    ));

    for i in 0..bounce_heights.len() {
        let peak_y = rest_y + bounce_heights[i];
        let dur = bounce_durations[i];

        // Up to peak
        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(
            t,
            AnimValue::Vec3(vec3(0.0, peak_y, 0.0)),
            quad_in,
        ));

        // Down to ground
        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(
            t,
            AnimValue::Vec3(vec3(0.0, rest_y, 0.0)),
            quad_out,
        ));
    }

    timeline.add_track(pos_track);

    // Rectangle: pulse size over the full duration
    let duration = t; // reuse final t from bouncing ball
    let mut rect_track = Track::new(rect_id, "size");
    rect_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec2(vec2(2.0, 3.0))));
    rect_track.add_keyframe(Keyframe::new(duration / 2.0, AnimValue::Vec2(vec2(3.0, 1.5))));
    rect_track.add_keyframe(Keyframe::new(duration, AnimValue::Vec2(vec2(2.0, 3.0))));
    timeline.add_track(rect_track);

    // Hexagon: full rotation over the duration
    let mut hex_track = Track::new(hex_id, "rotation");
    hex_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    hex_track.add_keyframe(Keyframe::new(
        duration,
        AnimValue::Float(std::f32::consts::TAU),
    ));
    timeline.add_track(hex_track);

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();

    // Camera positioned to see the bouncing ball from the side
    let mut camera = Camera::new(vec3(0.0, 4.0, 15.0), vec3(0.0, 3.0, 0.0));
    let mut orbit = OrbitController::from_camera(&camera);

    let mut exporter = Exporter::new(1280, 720, 60.0, "export_frames");

    loop {
        clear_background(BLACK);

        if exporter.is_active() {
            // --- Export mode ---

            // Save the previously rendered frame (needs GPU flush from last next_frame)
            if exporter.save_pending_frame() {
                // Export complete
                continue;
            }

            // Cancel on Escape
            if is_key_pressed(KeyCode::Escape) {
                exporter.cancel();
                continue;
            }

            // Apply scene at synthetic export time
            let export_time = exporter.current_time();
            timeline.apply(export_time, &mut scene);

            // Render scene to export render target
            exporter.clear();
            let export_cam = exporter.export_camera(&camera.to_macroquad());
            set_camera(&export_cam);
            scene.draw_all();

            exporter.mark_rendered();

            // Show preview on screen
            set_default_camera();
            exporter.draw_preview();
        } else {
            // --- Interactive mode ---

            // Start export on F5
            if is_key_pressed(debug_overlay.keybindings.export) {
                exporter.start(clock.duration);
            }

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
        }

        next_frame().await;
    }
}
