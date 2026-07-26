use macroquad::prelude::*;

use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::l_system::my3;
use crate::scene::objects::LSystem;
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

pub fn build() -> (Scene, Timeline, Camera) {
    // ── Scene ─────────────────────────────────────────────────────────
    let mut sb = SceneBuilder::new();

    let (config, theta) = my3();
    let mut l_system = LSystem::new(config, theta, GREEN).with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    l_system.iterations = 5.0;
    l_system.progress = 1.0;
    l_system.line_width = 0.1;
    l_system.scale = 0.5;
    let l_system_ref = sb.add(l_system);

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 14.5, 25.7), vec3(0.0, 14.5, 0.0)));

    // ── Animations ────────────────────────────────────────────────────
    sb.animate(&l_system_ref, "theta", |tb| {
        tb.keyframe_with_easing(0.0, AnimValue::Float(1.0), Easing::QuartInOut)
            .keyframe(27.0, AnimValue::Float(5.8))
            .keyframe(37.0, AnimValue::Float(5.8))
    });

    sb.animate(&l_system_ref, "progress", |tb| {
        tb.keyframe_with_easing(32.0, AnimValue::Float(1.0), Easing::QuintInOut)
            .keyframe(70.0, AnimValue::Float(0.0))
            .keyframe(72.0, AnimValue::Float(0.0))
    });

    sb.animate_camera("position", |tb| {
        tb.keyframe_with_easing(32.0, AnimValue::Vec3(vec3(0.0, 14.5, 25.7)), Easing::SineInOut)
            .keyframe(70.0, AnimValue::Vec3(vec3(0.0, 1.3, 5.0)))
    });

    sb.animate_camera("target", |tb| {
        tb.keyframe_with_easing(32.0, AnimValue::Vec3(vec3(0.0, 14.5, 0.0)), Easing::SineInOut)
            .keyframe(70.0, AnimValue::Vec3(vec3(0.0, 1.3, 0.0)))
    });

    sb.build()
}
