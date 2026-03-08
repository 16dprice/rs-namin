use std::f32::consts::PI;

#[allow(unused_imports)]
use macroquad::prelude::*;

#[allow(unused_imports)]
use crate::animation::easing::{cubic_in, cubic_in_out, cubic_out, quad_in, quad_in_out, quad_out};
#[allow(unused_imports)]
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
#[allow(unused_imports)]
use crate::scene::objects::{Circle, Line, Polygon, Rectangle, Spiral, Text};
use crate::scene::value::AnimValue;
use crate::{animation::timeline::Timeline, camera::ProjectionMode};

#[allow(unused_variables, unused_mut)]
pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let spiral_id = scene.add(Spiral::new(
        vec3(0.0, 0.0, 0.0),
        0.0001,
        1.0 / PI,
        BLUE,
        10_000,
        0.002,
    ));

    let mut delta_theta_track = Track::new(spiral_id, "delta_theta");
    delta_theta_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.001)));
    delta_theta_track.add_keyframe(Keyframe::new(360.0, AnimValue::Float(1.0 / PI)));

    let phi = (1.0 + f32::sqrt(5.0)) / 2.0;
    delta_theta_track.add_keyframe(Keyframe::new(600.0, AnimValue::Float(phi)));
    timeline.add_track(delta_theta_track);

    let mut camera = Camera::new(vec3(0.0, 0.0, 15.0), vec3(0.0, 0.0, 0.0));
    camera.projection = ProjectionMode::Orthographic;
    camera.fov = 100.0;

    let mut cam_pos_track = Track::camera("position");
    let mut cam_target_track = Track::camera("target");

    cam_pos_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 15.0))));
    cam_target_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 0.0))));

    cam_pos_track.add_keyframe(Keyframe::new(20.0, AnimValue::Vec3(vec3(1.0, 0.0, 15.0))));
    cam_target_track.add_keyframe(Keyframe::new(20.0, AnimValue::Vec3(vec3(1.0, 0.0, 0.0))));

    cam_pos_track.add_keyframe(Keyframe::new(40.0, AnimValue::Vec3(vec3(1.0, 1.0, 15.0))));
    cam_target_track.add_keyframe(Keyframe::new(40.0, AnimValue::Vec3(vec3(1.0, 1.0, 0.0))));

    cam_pos_track.add_keyframe(Keyframe::new(60.0, AnimValue::Vec3(vec3(0.0, 0.0, 15.0))));
    cam_target_track.add_keyframe(Keyframe::new(60.0, AnimValue::Vec3(vec3(0.0, 0.0, 0.0))));

    timeline.add_track(cam_pos_track);
    timeline.add_track(cam_target_track);

    (scene, timeline, camera)
}
