use std::f32::consts::PI;

use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Polyline, Sprite, Turtle};
use crate::scene::polyline::LineSegment;
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

fn get_star_line_segments() -> Vec<LineSegment> {
    let delta = 2.0 * PI / 5.0;
    let offset = PI / 10.0;
    let radius = 2.0;

    let top_right_angle = offset;
    let top_angle = offset + delta;
    let top_left_angle = offset + 2.0 * delta;
    let bottom_left_angle = offset + 3.0 * delta;
    let bottom_right_angle = offset + 4.0 * delta;

    let ordering = vec![
        top_right_angle,
        top_left_angle,
        bottom_right_angle,
        top_angle,
        bottom_left_angle,
        top_right_angle,
        top_angle,
        top_left_angle,
        bottom_left_angle,
        bottom_right_angle,
        top_right_angle,
    ];
    let mut segments: Vec<LineSegment> = vec![];

    for (idx, _) in ordering.iter().enumerate() {
        if idx == ordering.len() - 1 {
            continue;
        }

        let start_angle = ordering[idx];
        let end_angle = ordering[idx + 1];

        segments.push(LineSegment {
            start: vec2(radius * start_angle.cos(), radius * start_angle.sin()),
            end: vec2(radius * end_angle.cos(), radius * end_angle.sin()),
        });
    }

    segments
}

pub fn build() -> (Scene, Timeline, Camera) {
    let mut sb = SceneBuilder::new();

    let texture = Texture2D::from_file_with_format(include_bytes!("../assets/aseprite-files/tutle.png"), None);
    texture.set_filter(FilterMode::Nearest);
    let mut sprite = Sprite::new(texture, vec3(0.0, 0.0, 0.0), Some(vec2(1.0, 1.0)), WHITE);
    sprite.center = vec2(0.0, -0.5);

    // A star made of five line segments around a circle

    let segments = get_star_line_segments();

    let turtle = Turtle::new(sprite, segments.clone());
    let turtle_ref = sb.add(turtle);

    let polyline = Polyline::new(segments.clone(), WHITE).with_colors(vec![RED, BLUE]);
    let pl_ref = sb.add(polyline);

    sb.camera(Camera::new(vec3(1.5, 1.5, 8.0), vec3(1.5, 1.5, 0.0)));

    let seconds_per_move = 1.0;
    let seconds_per_rotation = 1.0;

    // Walk right → rotate up → walk up → rotate left → walk left
    // Position holds still during rotations, rotation holds still during walks.
    sb.parallel(|p| {
        p.animate(&turtle_ref, "progress", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Float(0.0));

            let total_segments = segments.clone().len();
            for i in 0..total_segments {
                let progress_value = (i + 1) as f32 / total_segments as f32;
                tb = tb
                    .animate_for(seconds_per_move, AnimValue::Float(progress_value), easing::sine_in_out)
                    .animate_for(seconds_per_rotation, AnimValue::Float(progress_value), easing::linear);
            }

            tb.animate_for(5.0, AnimValue::Float(1.0), easing::linear)
        });

        p.animate(&pl_ref, "progress", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Float(0.0));

            let total_segments = segments.clone().len();
            for i in 0..total_segments {
                let progress_value = (i + 1) as f32 / total_segments as f32;
                tb = tb
                    .animate_for(seconds_per_move, AnimValue::Float(progress_value), easing::sine_in_out)
                    .animate_for(seconds_per_rotation, AnimValue::Float(progress_value), easing::linear);
            }

            tb.animate_for(5.0, AnimValue::Float(1.0), easing::linear)
        });

        p.animate(&turtle_ref, "rotation", |mut tb| {
            let first_seg = segments.clone()[0];
            let d = first_seg.end - first_seg.start;
            let angle = d.y.atan2(d.x);

            tb = tb
                .keyframe(0.0, AnimValue::Float(angle))
                .animate_for(seconds_per_move, AnimValue::Float(angle), easing::linear);

            let mut angle = 0.0;
            for (idx, seg) in segments.clone().iter().enumerate() {
                if idx == 0 {
                    // first segment is already handled outside of this
                    continue;
                }

                let d = seg.end - seg.start;
                let target = d.y.atan2(d.x);
                let delta = (target - angle + PI).rem_euclid(std::f32::consts::TAU) - PI;
                angle += delta;

                tb = tb
                    .animate_for(seconds_per_rotation, AnimValue::Float(angle), easing::sine_in_out)
                    .animate_for(seconds_per_move, AnimValue::Float(angle), easing::linear);
            }

            tb.animate_for(5.0, AnimValue::Float(angle), easing::linear)
        });
    });

    sb.build()
}
