use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::l_system::LineSegment;
use crate::scene::objects::{Sprite, Turtle};
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

pub fn build() -> (Scene, Timeline, Camera) {
    // ── Scene ─────────────────────────────────────────────────────────
    let mut sb = SceneBuilder::new();

    let texture = Texture2D::from_file_with_format(
        include_bytes!("../assets/aseprite-files/tutle.png"),
        None,
    );
    texture.set_filter(FilterMode::Nearest);
    let sprite = Sprite::new(texture, vec3(0.0, 0.5, 0.0), Some(vec2(1.0, 1.0)), WHITE);
    let path = vec![
        LineSegment {
            start: vec2(0.0, 0.0),
            end: vec2(10.0, 0.0),
        },
        LineSegment {
            start: vec2(10.0, 0.0),
            end: vec2(10.0, 10.0),
        },
    ];
    let turtle = Turtle::new(sprite, path.clone());
    let turtle_ref = sb.add(turtle);

    let total_seconds = 6.0;

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), vec3(0.0, 0.0, 0.0)));

    // ── Animations ────────────────────────────────────────────────────
    sb.animate_camera("position", |mut tb| {
        let seconds_per_segment = total_seconds / path.len() as f32;

        tb = tb.keyframe_with_easing(
            0.0,
            AnimValue::Vec3(vec3(path[0].start.x, path[0].start.y, 10.0)),
            easing::sine_in_out,
        );
        for (i, seg) in path.iter().enumerate() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Vec3(vec3(seg.end.x, seg.end.y, 10.0)),
                easing::sine_in_out,
            );
        }
        tb
    });

    sb.animate_camera("target", |mut tb| {
        let seconds_per_segment = total_seconds / path.len() as f32;

        tb = tb.keyframe_with_easing(
            0.0,
            AnimValue::Vec3(vec3(path[0].start.x, path[0].start.y, 0.0)),
            easing::sine_in_out,
        );
        for (i, seg) in path.iter().enumerate() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Vec3(vec3(seg.end.x, seg.end.y, 0.0)),
                easing::sine_in_out,
            );
        }
        tb
    });

    sb.animate(&turtle_ref, "progress", |tb| {
        tb.keyframe_with_easing(0.0, AnimValue::Float(0.0), easing::sine_in_out)
            .keyframe_with_easing(3.0, AnimValue::Float(0.5), easing::sine_in_out)
            .keyframe(6.0, AnimValue::Float(1.0))
    });

    sb.build()
}
