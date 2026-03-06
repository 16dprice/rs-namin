#![allow(dead_code)]

mod animation;
mod clock;
#[cfg(test)]
mod clock_tests;
mod scene;

use macroquad::prelude::*;

use animation::easing::quad_in_out;
use animation::timeline::Timeline;
use animation::track::{Keyframe, Track};
use clock::Clock;
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
    let circle_id = scene.add(Circle::new(vec3(200.0, 360.0, 0.0), 50.0, BLUE));
    scene.add(Line::new(
        vec3(0.0, 600.0, 0.0),
        vec3(1280.0, 600.0, 0.0),
        2.0,
        WHITE,
    ));

    let mut timeline = Timeline::new();

    // Animate circle position across the screen
    let mut pos_track = Track::new(circle_id, "position");
    pos_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(vec3(200.0, 360.0, 0.0))));
    pos_track.add_keyframe(Keyframe::with_easing(
        2.0,
        AnimValue::Vec3(vec3(1080.0, 360.0, 0.0)),
        quad_in_out,
    ));
    timeline.add_track(pos_track);

    // Animate circle radius
    let mut radius_track = Track::new(circle_id, "radius");
    radius_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(50.0)));
    radius_track.add_keyframe(Keyframe::with_easing(
        1.0,
        AnimValue::Float(100.0),
        quad_in_out,
    ));
    radius_track.add_keyframe(Keyframe::with_easing(
        2.0,
        AnimValue::Float(50.0),
        quad_in_out,
    ));
    timeline.add_track(radius_track);

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    loop {
        clear_background(BLACK);

        clock.tick(get_frame_time());
        timeline.apply(clock.current_time, &mut scene);
        scene.draw_all();
        draw_fps();

        next_frame().await;
    }
}
