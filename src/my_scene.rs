use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::Tube;
use crate::scene::value::AnimValue;

#[allow(unused_variables, unused_mut)]
pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let p = 5;
    let q = 7;
    let c = 3.0; // distance from center of tube to center of torus
    let a = 1.0; // radius of the torus cross-section

    let num_points = 1_000;
    let points: Vec<Vec3> = (0..num_points)
        .map(|i| {
            let t = i as f32 / num_points as f32 * TAU;
            let x = (p as f32 * t).cos() * (c + a * (q as f32 * t).cos());
            let y = (p as f32 * t).sin() * (c + a * (q as f32 * t).cos());
            let z = a * (q as f32 * t).sin();
            vec3(x, y, z)
        })
        .collect();

    let mut knot = Tube::with_colors(
        points,
        0.15,
        vec![
            RED,                              // red
            ORANGE,                           // orange
            YELLOW,                           // yellow
            GREEN,                            // green
            BLUE,                             // blue
            Color::new(0.29, 0.0, 0.51, 1.0), // indigo
            Color::new(0.56, 0.0, 1.0, 1.0),  // violet
        ],
    );
    knot.closed = true;
    scene.add(knot);

    // Camera orbits around the knot
    let cam_radius = 10.0_f32;
    let mut cam_pos_track = Track::camera("position");
    let mut cam_target_track = Track::camera("target");

    let num_keys = 8;
    for i in 0..=num_keys {
        let frac = i as f32 / num_keys as f32;
        let angle = frac * PI * 2.0;
        let t = frac * 6.0;
        cam_pos_track.add_keyframe(Keyframe::new(
            t,
            AnimValue::Vec3(vec3(
                cam_radius * angle.sin(),
                4.0,
                cam_radius * angle.cos(),
            )),
        ));
        cam_target_track.add_keyframe(Keyframe::new(t, AnimValue::Vec3(Vec3::ZERO)));
    }
    timeline.add_track(cam_pos_track);
    timeline.add_track(cam_target_track);

    let camera = Camera::new(vec3(0.0, 4.0, cam_radius), Vec3::ZERO);

    (scene, timeline, camera)
}
