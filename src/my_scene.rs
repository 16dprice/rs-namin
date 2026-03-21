use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::l_system::{apply_rules, dragon_curve, get_lines};
use crate::scene::objects::{LSystem, Sprite, Turtle};
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
    let mut sprite = Sprite::new(texture, vec3(0.0, 0.0, 0.0), Some(vec2(1.0, 1.0)), WHITE);
    sprite.center = vec2(0.0, -0.5);

    let (config, theta) = dragon_curve();
    let iterations = 5;
    let s = apply_rules(&config, iterations);
    let lines = get_lines(&s, theta, 1.0);

    let mut l_system = LSystem::new(config, theta, BLUE)
        .with_colors(vec![RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE]);
    l_system.iterations = iterations as f32;
    l_system.scale = 1.0;

    let turtle = Turtle::new(sprite, lines.clone());

    let l_system_ref = sb.add(l_system);
    let turtle_ref = sb.add(turtle);

    let total_seconds = 1.0 * lines.len() as f32;

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), vec3(0.0, 0.0, 0.0)));

    // ── Animations ────────────────────────────────────────────────────
    sb.animate_camera("position", |mut tb| {
        let seconds_per_segment = total_seconds / lines.len() as f32;

        tb = tb.keyframe_with_easing(
            0.0,
            AnimValue::Vec3(vec3(lines[0].start.x, lines[0].start.y, 6.0)),
            easing::sine_in_out,
        );
        for (i, seg) in lines.iter().enumerate() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Vec3(vec3(seg.end.x, seg.end.y, 6.0)),
                easing::sine_in_out,
            );
        }
        tb
    });

    sb.animate_camera("target", |mut tb| {
        let seconds_per_segment = total_seconds / lines.len() as f32;

        tb = tb.keyframe_with_easing(
            0.0,
            AnimValue::Vec3(vec3(lines[0].start.x, lines[0].start.y, 0.0)),
            easing::sine_in_out,
        );
        for (i, seg) in lines.iter().enumerate() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Vec3(vec3(seg.end.x, seg.end.y, 0.0)),
                easing::sine_in_out,
            );
        }
        tb
    });

    sb.animate(&turtle_ref, "progress", |mut tb| {
        let seconds_per_segment = total_seconds / lines.len() as f32;

        tb = tb.keyframe_with_easing(0.0, AnimValue::Float(0.0), easing::sine_in_out);
        for i in 0..lines.len() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Float((i + 1) as f32 / lines.len() as f32),
                easing::sine_in_out,
            )
        }
        tb
    });

    sb.animate(&l_system_ref, "progress", |mut tb| {
        let seconds_per_segment = total_seconds / lines.len() as f32;

        tb = tb.keyframe_with_easing(0.0, AnimValue::Float(0.0), easing::sine_in_out);
        for i in 0..lines.len() {
            tb = tb.keyframe_with_easing(
                (i + 1) as f32 * seconds_per_segment,
                AnimValue::Float((i + 1) as f32 / lines.len() as f32),
                easing::sine_in_out,
            )
        }
        tb
    });

    sb.animate(&l_system_ref, "line_width", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.02))
            .keyframe(total_seconds, AnimValue::Float(0.2))
    });

    sb.build()
}
