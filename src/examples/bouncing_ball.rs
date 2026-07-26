use macroquad::prelude::*;

use crate::animation::easing::{quad_in, quad_out};
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Disk, Line, Polygon, Rectangle, Text};
use crate::scene::value::AnimValue;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();

    let radius = 0.5_f32;
    let rest_y = radius;

    let circle_id = scene.add(Disk::new(vec3(0.0, rest_y, 0.0), radius, BLUE));
    scene.add(Line::new(vec3(-10.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), WHITE));

    let rect_id = scene.add(Rectangle::new(vec3(-3.0, 1.5, 0.0), vec2(2.0, 3.0), GREEN));
    let hex_id = scene.add(Polygon::new(vec3(3.0, 2.0, 0.0), 1.0, 6, YELLOW));

    let mut timeline = Timeline::new();

    let bounce_heights = [6.0_f32, 3.0, 1.2];
    let bounce_durations = [0.8_f32, 0.6, 0.4];

    let mut pos_track = Track::new(circle_id, "position");
    let mut t = 0.0_f32;

    pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, rest_y, 0.0)), quad_out));

    for i in 0..bounce_heights.len() {
        let peak_y = rest_y + bounce_heights[i];
        let dur = bounce_durations[i];

        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, peak_y, 0.0)), quad_in));

        t += dur;
        pos_track.add_keyframe(Keyframe::with_easing(t, AnimValue::Vec3(vec3(0.0, rest_y, 0.0)), quad_out));
    }

    timeline.add_track(pos_track);

    let duration = t;
    let mut rect_track = Track::new(rect_id, "size");
    rect_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec2(vec2(2.0, 3.0))));
    rect_track.add_keyframe(Keyframe::new(duration / 2.0, AnimValue::Vec2(vec2(3.0, 1.5))));
    rect_track.add_keyframe(Keyframe::new(duration, AnimValue::Vec2(vec2(2.0, 3.0))));
    timeline.add_track(rect_track);

    let mut hex_track = Track::new(hex_id, "rotation");
    hex_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    hex_track.add_keyframe(Keyframe::new(duration, AnimValue::Float(std::f32::consts::TAU)));
    timeline.add_track(hex_track);

    let mut label = Text::new("bouncing ball", vec2(40.0, 40.0), 48.0, WHITE);
    label.progress = 0.0;
    let text_id = scene.add(label);
    let mut text_track = Track::new(text_id, "progress");
    text_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    text_track.add_keyframe(Keyframe::new(duration / 2.0, AnimValue::Float(1.0)));
    timeline.add_track(text_track);

    let camera = Camera::new(vec3(0.0, 4.0, 15.0), vec3(0.0, 3.0, 0.0));

    (scene, timeline, camera)
}
