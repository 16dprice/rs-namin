#![allow(dead_code)]

mod animation;
mod clock;
mod debug;
mod scene;
#[cfg(test)]
mod tests;

use macroquad::prelude::*;

use animation::easing::{quad_in, quad_out};
use animation::timeline::Timeline;
use animation::track::{Keyframe, Track};
use clock::Clock;
use debug::DebugOverlay;
use scene::objects::{Circle, Line};
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

    let ground_y = 600.0_f32;
    let radius = 50.0_f32;
    let rest_y = ground_y - radius; // ball center when sitting on ground

    let circle_id = scene.add(Circle::new(vec3(640.0, rest_y, 0.0), radius, BLUE));
    scene.add(Line::new(
        vec3(0.0, ground_y, 0.0),
        vec3(1280.0, ground_y, 0.0),
        2.0,
        WHITE,
    ));

    let mut timeline = Timeline::new();

    // Bouncing ball: 3 bounces with decreasing height, then rest
    // Each bounce: quad_out up (decelerating), quad_in down (accelerating)
    let bounce_heights = [300.0_f32, 150.0, 60.0];
    let bounce_durations = [0.8_f32, 0.6, 0.4]; // seconds per half-bounce

    let mut pos_track = Track::new(circle_id, "position");
    let mut t = 0.0_f32;

    // Start on the ground
    pos_track.add_keyframe(Keyframe::with_easing(
        t,
        AnimValue::Vec3(vec3(640.0, rest_y, 0.0)),
        quad_out,
    ));

    for i in 0..bounce_heights.len() {
        let peak_y = rest_y - bounce_heights[i];
        let dur = bounce_durations[i];

        // Up to peak
        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(
            t,
            AnimValue::Vec3(vec3(640.0, peak_y, 0.0)),
            quad_in,
        ));

        // Down to ground
        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(
            t,
            AnimValue::Vec3(vec3(640.0, rest_y, 0.0)),
            quad_out,
        ));
    }

    timeline.add_track(pos_track);

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();

    loop {
        clear_background(BLACK);

        debug_overlay.handle_input(&mut clock);
        debug_overlay.update(&mut clock);

        clock.tick(get_frame_time());
        timeline.apply(clock.current_time, &mut scene);

        scene.draw_all();
        debug_overlay.draw(&clock, &scene);
        debug_overlay.scrub_bar.draw_ticks(&timeline, clock.duration);

        next_frame().await;
    }
}
