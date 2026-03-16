use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Ring, Tube};
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

/// Dolly zoom easing: follows the 1/tan curve so that d * tan(fov/2) stays constant
/// when FOV is linearly interpolated from 60° to 2°.
///
/// TECH DEBT: FOV values are hardcoded because EasingFn is `fn(f32) -> f32` and
/// can't capture state. Changing EasingFn to accept closures (e.g. Box<dyn Fn>)
/// would allow a `dolly_zoom(fov_start, fov_end)` factory instead.
fn dolly_zoom(t: f32) -> f32 {
    let fov_start = 60.0_f32.to_radians();
    let fov_end = 2.0_f32.to_radians();
    let fov = fov_start + t * (fov_end - fov_start);
    let inv_tan = |f: f32| 1.0 / (f / 2.0).tan();
    (inv_tan(fov) - inv_tan(fov_start)) / (inv_tan(fov_end) - inv_tan(fov_start))
}

pub fn build() -> (Scene, Timeline, Camera) {
    // ── Timeline ──────────────────────────────────────────────────────
    let cam_radius = 10.0_f32;
    let fov_start: f32 = 60.0;
    let fov_end: f32 = 2.0;
    let dolly_distance =
        cam_radius * (fov_start / 2.0).to_radians().tan() / (fov_end / 2.0).to_radians().tan();

    let rotation_start = 0.0;
    let rotation_end = 30.0;
    let dolly_start = rotation_end;
    let dolly_end = dolly_start + 4.0;
    let ring1_start = dolly_end + 1.0;
    let ring1_end = dolly_end + 3.0;
    let ring2_start = dolly_end + 2.0;
    let ring2_end = dolly_end + 4.0;
    let ring3_start = dolly_end + 3.0;
    let ring3_end = dolly_end + 5.0;

    let knot_zoom_move_back_start = ring3_end + 2.0;
    let knot_zoom_move_back_end = knot_zoom_move_back_start + 6.0;
    let scene_end = knot_zoom_move_back_end + 5.0;

    // ── Scene ─────────────────────────────────────────────────────────
    let mut sb = SceneBuilder::new();

    // Torus knot geometry
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
    let knot_tube = sb.add(knot);

    let ring1_start_position = vec3(1.54, 2.61, 2.0);
    let ring2_start_position = vec3(-3.0, 0.0, 2.0);
    let ring3_start_position = vec3(1.54, -2.61, 2.0);

    let ring1 = sb.add(Ring::new(ring1_start_position, 0.5, WHITE, 0.0));
    let ring2 = sb.add(Ring::new(ring2_start_position, 0.5, WHITE, 0.0));
    let ring3 = sb.add(Ring::new(ring3_start_position, 0.5, WHITE, 0.0));

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 4.0, cam_radius), Vec3::ZERO));

    // ── Animations ────────────────────────────────────────────────────
    sb.animate_camera("rotation_y", |tb| {
        tb.keyframe_with_easing(rotation_start, AnimValue::Float(0.0), easing::sine_in_out)
            .keyframe(rotation_end, AnimValue::Float(TAU))
    });

    sb.animate_camera("position", |tb| {
        tb.keyframe(rotation_start, AnimValue::Vec3(vec3(0.0, 4.0, cam_radius)))
            .keyframe_with_easing(
                dolly_start,
                AnimValue::Vec3(vec3(0.0, 4.0, cam_radius)),
                dolly_zoom,
            )
            .keyframe(dolly_end, AnimValue::Vec3(vec3(0.0, 0.0, dolly_distance)))
    });

    sb.animate_camera("target", |tb| {
        tb.keyframe(rotation_start, AnimValue::Vec3(Vec3::ZERO))
            .keyframe(scene_end, AnimValue::Vec3(Vec3::ZERO))
    });

    sb.animate_camera("fov", |tb| {
        tb.keyframe(rotation_start, AnimValue::Float(fov_start))
            .keyframe(dolly_start, AnimValue::Float(fov_start))
            .keyframe(dolly_end, AnimValue::Float(fov_end))
    });

    sb.animate(&ring1, "sweep", |tb| {
        tb.keyframe_with_easing(ring1_start, AnimValue::Float(0.0), easing::quart_out)
            .keyframe(ring1_end, AnimValue::Float(1.0))
    });
    sb.animate(&ring2, "sweep", |tb| {
        tb.keyframe_with_easing(ring2_start, AnimValue::Float(0.0), easing::quart_out)
            .keyframe(ring2_end, AnimValue::Float(1.0))
    });
    sb.animate(&ring3, "sweep", |tb| {
        tb.keyframe_with_easing(ring3_start, AnimValue::Float(0.0), easing::quart_out)
            .keyframe(ring3_end, AnimValue::Float(1.0))
    });

    sb.animate(&ring1, "position", |tb| {
        tb.keyframe_with_easing(
            knot_zoom_move_back_start,
            AnimValue::Vec3(ring1_start_position),
            easing::sine_in_out,
        )
        .keyframe(
            knot_zoom_move_back_end,
            AnimValue::Vec3(vec3(-2.87, 1.96, 2.0)),
        )
    });
    sb.animate(&ring2, "position", |tb| {
        tb.keyframe_with_easing(
            knot_zoom_move_back_start,
            AnimValue::Vec3(ring2_start_position),
            easing::sine_in_out,
        )
        .keyframe(
            knot_zoom_move_back_end,
            AnimValue::Vec3(vec3(-6.24, 0.0, 2.0)),
        )
    });
    sb.animate(&ring3, "position", |tb| {
        tb.keyframe_with_easing(
            knot_zoom_move_back_start,
            AnimValue::Vec3(ring3_start_position),
            easing::sine_in_out,
        )
        .keyframe(
            knot_zoom_move_back_end,
            AnimValue::Vec3(vec3(-2.87, -1.96, 2.0)),
        )
    });

    sb.animate(&ring1, "radius", |tb| {
        tb.keyframe(knot_zoom_move_back_start, AnimValue::Float(0.5))
            .keyframe(knot_zoom_move_back_end, AnimValue::Float(0.4))
    });
    sb.animate(&ring2, "radius", |tb| {
        tb.keyframe(knot_zoom_move_back_start, AnimValue::Float(0.5))
            .keyframe(knot_zoom_move_back_end, AnimValue::Float(0.4))
    });
    sb.animate(&ring3, "radius", |tb| {
        tb.keyframe(knot_zoom_move_back_start, AnimValue::Float(0.5))
            .keyframe(knot_zoom_move_back_end, AnimValue::Float(0.4))
    });

    sb.animate(&knot_tube, "position", |tb| {
        tb.keyframe_with_easing(
            knot_zoom_move_back_start,
            AnimValue::Vec3(Vec3::ZERO),
            easing::sine_in_out,
        )
        .keyframe(
            knot_zoom_move_back_end,
            AnimValue::Vec3(vec3(-4.0, 0.0, 0.0)),
        )
    });
    sb.animate(&knot_tube, "scale", |tb| {
        tb.keyframe_with_easing(
            knot_zoom_move_back_start,
            AnimValue::Float(1.0),
            easing::sine_in_out,
        )
        .keyframe(knot_zoom_move_back_end, AnimValue::Float(0.75))
    });

    sb.build()
}
