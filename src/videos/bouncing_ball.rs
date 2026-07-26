use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::animation::easing::{quad_in, quad_out};
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Disk, Line, Polygon, Rectangle, Text};
use crate::scene::value::AnimValue;

const DURATION: f32 = 20.0;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let radius = 0.5_f32;
    let rest_y = radius;

    let circle_id = scene.add(Disk::new(vec3(0.0, rest_y, 0.0), radius, BLUE));
    scene.add(Line::new(vec3(-10.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), 1.0, WHITE));

    let rect_id = scene.add(Rectangle::new(vec3(-3.0, 1.5, 0.0), vec2(2.0, 3.0), GREEN));
    let hex_id = scene.add(Polygon::new(vec3(3.0, 2.0, 0.0), 1.0, 6, YELLOW));

    // Repeating bounce pattern to fill 20 seconds
    let bounce_heights = [6.0_f32, 3.0, 1.2];
    let bounce_durations = [0.8_f32, 0.6, 0.4];

    let mut pos_track = Track::new(circle_id, "position");
    let mut t = 0.0_f32;

    pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, rest_y, 0.0)), quad_out));

    while t < DURATION {
        for i in 0..bounce_heights.len() {
            let peak_y = rest_y + bounce_heights[i];
            let dur = bounce_durations[i];

            t += dur;
            pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, peak_y, 0.0)), quad_in));

            t += dur;
            pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, rest_y, 0.0)), quad_out));
        }
    }
    timeline.add_track(pos_track);

    // Rectangle pulses over the full duration
    let mut rect_track = Track::new(rect_id, "size");
    rect_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec2(vec2(2.0, 3.0))));
    rect_track.add_keyframe(Keyframe::new(DURATION / 2.0, AnimValue::Vec2(vec2(3.0, 1.5))));
    rect_track.add_keyframe(Keyframe::new(DURATION, AnimValue::Vec2(vec2(2.0, 3.0))));
    timeline.add_track(rect_track);

    // Hexagon rotates continuously
    let mut hex_track = Track::new(hex_id, "rotation");
    hex_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    hex_track.add_keyframe(Keyframe::new(DURATION, AnimValue::Float(TAU * 3.0)));
    timeline.add_track(hex_track);

    // Text reveal in the first few seconds
    let mut label = Text::new("bouncing ball", vec2(40.0, 40.0), 48.0, WHITE);
    label.percentage_shown = 0.0;
    let text_id = scene.add(label);
    let mut text_track = Track::new(text_id, "percentage_shown");
    text_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    text_track.add_keyframe(Keyframe::new(3.0, AnimValue::Float(1.0)));
    timeline.add_track(text_track);

    // Slow camera orbit: rotate around the scene center at y=3
    // Camera orbits at radius 15, looking at (0, 3, 0)
    let cam_radius = 15.0_f32;
    let cam_center = vec3(0.0, 3.0, 0.0);
    let cam_y = 4.0_f32;

    let num_cam_keys = 8;
    let mut cam_pos_track = Track::camera("position");
    let mut cam_target_track = Track::camera("target");

    for i in 0..=num_cam_keys {
        let frac = i as f32 / num_cam_keys as f32;
        let angle = frac * TAU * 0.4; // ~144 degrees over 20s — slow partial orbit
        let cam_pos = vec3(cam_radius * angle.sin(), cam_y, cam_radius * angle.cos());
        let time = frac * DURATION;

        // Slight target drift to keep it interesting
        let target = cam_center + vec3(angle.sin() * 0.5, 0.0, 0.0);

        cam_pos_track.add_keyframe(Keyframe::new(time, AnimValue::Vec3(cam_pos)));
        cam_target_track.add_keyframe(Keyframe::new(time, AnimValue::Vec3(target)));
    }

    timeline.add_track(cam_pos_track);
    timeline.add_track(cam_target_track);

    // Camera starts at angle=0 → position (0, 4, 15) looking at (0, 3, 0)
    let camera = Camera::new(vec3(0.0, cam_y, cam_radius), cam_center);

    (scene, timeline, camera)
}
