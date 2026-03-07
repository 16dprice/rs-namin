use std::f32::consts::PI;

#[allow(unused_imports)]
use macroquad::prelude::*;

#[allow(unused_imports)]
use crate::animation::easing::{cubic_in, cubic_in_out, cubic_out, quad_in, quad_in_out, quad_out};
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::scene::Scene;
#[allow(unused_imports)]
use crate::scene::objects::{Circle, Line, Polygon, Rectangle, Spiral, Text};
use crate::scene::value::AnimValue;

#[allow(unused_variables, unused_mut)]
pub fn build() -> (Scene, Timeline) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let spiral_id = scene.add(Spiral::new(
        vec3(0.0, 0.0, 0.0),
        0.001,
        1.0 / PI,
        BLUE,
        2_000,
        0.01,
    ));

    let mut delta_theta_track = Track::new(spiral_id, "delta_theta");
    delta_theta_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.001)));
    delta_theta_track.add_keyframe(Keyframe::new(120.0, AnimValue::Float(1.0 / PI)));

    let phi = (1.0 + f32::sqrt(5.0)) / 2.0;
    delta_theta_track.add_keyframe(Keyframe::new(240.0, AnimValue::Float(phi)));
    timeline.add_track(delta_theta_track);

    (scene, timeline)
}
