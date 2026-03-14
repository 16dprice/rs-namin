use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::animation::easing::cubic_in_out;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::Tube;
use crate::scene::value::AnimValue;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    // A helix path
    let helix_points: Vec<Vec3> = (0..120)
        .map(|i| {
            let t = i as f32 / 119.0;
            let angle = t * TAU * 3.0;
            vec3(angle.cos() * 2.0, t * 8.0 - 4.0, angle.sin() * 2.0)
        })
        .collect();

    let helix_id = scene.add(Tube::new(helix_points, 0.15, BLUE));

    // Animate the radius pulsing
    let mut radius_track = Track::new(helix_id, "radius");
    radius_track.add_keyframe(Keyframe::with_easing(
        0.0,
        AnimValue::Float(0.15),
        cubic_in_out,
    ));
    radius_track.add_keyframe(Keyframe::with_easing(
        3.0,
        AnimValue::Float(0.4),
        cubic_in_out,
    ));
    radius_track.add_keyframe(Keyframe::new(6.0, AnimValue::Float(0.15)));
    timeline.add_track(radius_track);

    // A closed trefoil knot
    let knot_points: Vec<Vec3> = (0..200)
        .map(|i| {
            let t = i as f32 / 200.0 * TAU;
            let x = t.sin() + 2.0 * (2.0 * t).sin();
            let y = t.cos() - 2.0 * (2.0 * t).cos();
            let z = -(3.0 * t).sin();
            vec3(x, y, z)
        })
        .collect();

    let mut knot = Tube::with_colors(knot_points, 0.1, vec![RED, GREEN, BLUE]);
    knot.position = vec3(6.0, 0.0, 0.0);
    knot.closed = true;
    scene.add(knot);

    // A simple L-shaped path
    let l_points = vec![
        vec3(0.0, -2.0, 0.0),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 2.0, 0.0),
        vec3(0.0, 2.0, 2.0),
    ];
    let mut l_tube = Tube::new(l_points, 0.25, YELLOW);
    l_tube.position = vec3(-6.0, 0.0, 0.0);
    scene.add(l_tube);

    // Camera orbits slowly
    let cam_radius = 12.0_f32;
    let mut cam_pos_track = Track::camera("position");
    let mut cam_target_track = Track::camera("target");

    let num_keys = 6;
    for i in 0..=num_keys {
        let frac = i as f32 / num_keys as f32;
        let angle = frac * PI * 0.5;
        let t = frac * 6.0;
        cam_pos_track.add_keyframe(Keyframe::new(
            t,
            AnimValue::Vec3(vec3(
                cam_radius * angle.sin(),
                3.0,
                cam_radius * angle.cos(),
            )),
        ));
        cam_target_track.add_keyframe(Keyframe::new(t, AnimValue::Vec3(vec3(0.0, 0.0, 0.0))));
    }
    timeline.add_track(cam_pos_track);
    timeline.add_track(cam_target_track);

    let camera = Camera::new(vec3(0.0, 3.0, cam_radius), Vec3::ZERO);

    (scene, timeline, camera)
}
