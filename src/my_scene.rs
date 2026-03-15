use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Ring, Tube};
use crate::scene::value::AnimValue;

#[allow(unused_variables, unused_mut)]
pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let p = 2;
    let q = 3;
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

    let camera_rotation_time = 30.0;

    // --------------------------------
    // CAMERA ROTATION TRACK
    // --------------------------------
    let mut cam_rot_track = Track::camera("rotation_y");
    cam_rot_track.add_keyframe(Keyframe::with_easing(
        0.0,
        AnimValue::Float(0.0),
        easing::sine_in_out,
    ));
    cam_rot_track.add_keyframe(Keyframe::new(camera_rotation_time, AnimValue::Float(TAU)));
    // --------------------------------
    // CAMERA ROTATION TRACK
    // --------------------------------

    // --------------------------------
    // CAMERA POSITION TRACK
    // --------------------------------
    cam_pos_track.add_keyframe(Keyframe::new(
        0.0,
        AnimValue::Vec3(vec3(0.0, 4.0, cam_radius)),
    ));
    cam_pos_track.add_keyframe(Keyframe::new(
        camera_rotation_time,
        AnimValue::Vec3(vec3(0.0, 4.0, cam_radius)),
    ));

    // Dolly zoom: shrink FOV while pulling camera back to keep subject same size.
    // d * tan(fov/2) = constant
    let fov_start: f32 = 60.0;
    let fov_end: f32 = 2.0;
    let dolly_start = camera_rotation_time;
    let dolly_end = dolly_start + 4.0;
    let dolly_distance =
        cam_radius * (fov_start / 2.0).to_radians().tan() / (fov_end / 2.0).to_radians().tan();

    // Use multiple keyframes to approximate the nonlinear distance curve
    let dolly_steps = 500;
    for i in 0..=dolly_steps {
        let frac = i as f32 / dolly_steps as f32;
        let t = dolly_start + frac * (dolly_end - dolly_start);
        let fov = fov_start + frac * (fov_end - fov_start);
        let d = cam_radius * (fov_start / 2.0).to_radians().tan() / (fov / 2.0).to_radians().tan();
        cam_pos_track.add_keyframe(Keyframe::new(
            t,
            AnimValue::Vec3(vec3(0.0, 4.0 - 4.0 * frac, d)),
        ));
    }

    cam_pos_track.add_keyframe(Keyframe::new(
        dolly_end,
        AnimValue::Vec3(vec3(0.0, 0.0, dolly_distance)),
    ));
    // --------------------------------
    // CAMERA POSITION TRACK
    // --------------------------------

    // --------------------------------
    // CAMERA TARGET TRACK
    // --------------------------------
    let mut cam_target_track = Track::camera("target");
    cam_target_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(Vec3::ZERO)));
    // --------------------------------
    // CAMERA TARGET TRACK
    // --------------------------------

    // --------------------------------
    // CAMERA FOV TRACK
    // --------------------------------
    let mut cam_fov_track = Track::camera("fov");
    cam_fov_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(fov_start)));
    cam_fov_track.add_keyframe(Keyframe::new(dolly_start, AnimValue::Float(fov_start)));
    cam_fov_track.add_keyframe(Keyframe::new(dolly_end, AnimValue::Float(fov_end)));
    // --------------------------------
    // CAMERA FOV TRACK
    // --------------------------------

    timeline.add_track(cam_rot_track);
    timeline.add_track(cam_pos_track);
    timeline.add_track(cam_target_track);
    timeline.add_track(cam_fov_track);

    let camera = Camera::new(vec3(0.0, 4.0, cam_radius), Vec3::ZERO);

    let ring1_id = scene.add(Ring::new(vec3(1.54, 2.61, 2.0), 0.5, WHITE, 0.0));
    let ring2_id = scene.add(Ring::new(vec3(-3.0, 0.0, 2.0), 0.5, WHITE, 0.0));
    let ring3_id = scene.add(Ring::new(vec3(1.54, -2.61, 2.0), 0.5, WHITE, 0.0));
    // --------------------------------
    // RING SWEEP TRACK
    // --------------------------------
    let mut ring1_sweep_track = Track::new(ring1_id, "sweep");
    ring1_sweep_track.add_keyframe(Keyframe::with_easing(
        dolly_end + 1.0,
        AnimValue::Float(0.0),
        easing::quart_out,
    ));
    ring1_sweep_track.add_keyframe(Keyframe::new(dolly_end + 3.0, AnimValue::Float(1.0)));

    let mut ring2_sweep_track = Track::new(ring2_id, "sweep");
    ring2_sweep_track.add_keyframe(Keyframe::with_easing(
        dolly_end + 2.0,
        AnimValue::Float(0.0),
        easing::quart_out,
    ));
    ring2_sweep_track.add_keyframe(Keyframe::new(dolly_end + 4.0, AnimValue::Float(1.0)));

    let mut ring3_sweep_track = Track::new(ring3_id, "sweep");
    ring3_sweep_track.add_keyframe(Keyframe::with_easing(
        dolly_end + 3.0,
        AnimValue::Float(0.0),
        easing::quart_out,
    ));
    ring3_sweep_track.add_keyframe(Keyframe::new(dolly_end + 5.0, AnimValue::Float(1.0)));
    // --------------------------------
    // RING SWEEP TRACK
    // --------------------------------

    timeline.add_track(ring1_sweep_track);
    timeline.add_track(ring2_sweep_track);
    timeline.add_track(ring3_sweep_track);

    (scene, timeline, camera)
}
