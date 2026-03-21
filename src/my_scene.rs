use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::objects::LSystem;
use crate::scene::value::AnimValue;
use crate::scene::{Scene, l_system};
use crate::scene_builder::SceneBuilder;

pub fn build() -> (Scene, Timeline, Camera) {
    // ── Scene ─────────────────────────────────────────────────────────
    let mut sb = SceneBuilder::new();

    let (dragon_curve_config, dragon_curve_theta) = l_system::dragon_curve();
    let (sierpinski_config, sierpinski_theta) = l_system::sierpinski();
    let (koch_config, koch_theta) = l_system::koch();
    let (fractal_plant_config, fractal_plant_theta) = l_system::fractal_plant();

    let mut dragon_curve_l_system = LSystem::new(dragon_curve_config, dragon_curve_theta, WHITE)
        .with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    dragon_curve_l_system.position = vec3(7.0, 2.0, 0.0); // TOP RIGHT
    dragon_curve_l_system.iterations = 10.0;
    dragon_curve_l_system.progress = 1.0;
    dragon_curve_l_system.line_width = 0.2;
    dragon_curve_l_system.scale = 0.15;

    let mut sierpinski_l_system = LSystem::new(sierpinski_config, sierpinski_theta, WHITE)
        .with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    sierpinski_l_system.position = vec3(-8.0, -1.0, 0.0); // TOP LEFT
    sierpinski_l_system.iterations = 7.0;
    sierpinski_l_system.progress = 1.0;
    sierpinski_l_system.line_width = 0.5;
    sierpinski_l_system.scale = 0.05;

    let mut koch_l_system = LSystem::new(koch_config, koch_theta, WHITE)
        .with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    koch_l_system.position = vec3(-4.0, -5.0, 0.0); // BOTTOM LEFT
    koch_l_system.iterations = 5.0;
    koch_l_system.progress = 1.0;
    koch_l_system.line_width = 0.8;
    koch_l_system.scale = 0.02;
    koch_l_system.theta = std::f32::consts::FRAC_PI_2 - 0.05;

    let mut fractal_plant_l_system = LSystem::new(fractal_plant_config, fractal_plant_theta, WHITE)
        .with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    fractal_plant_l_system.position = vec3(4.0, -5.0, 0.0); // BOTTOM RIGHT
    fractal_plant_l_system.iterations = 5.0;
    fractal_plant_l_system.progress = 1.0;
    fractal_plant_l_system.line_width = 0.5;
    fractal_plant_l_system.scale = 0.08;

    let dragon_curve_ref = sb.add(dragon_curve_l_system);
    let sierpinski_ref = sb.add(sierpinski_l_system);
    let koch_ref = sb.add(koch_l_system);
    let fractal_plant_ref = sb.add(fractal_plant_l_system);

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), vec3(0.0, 0.0, 0.0)));

    // ── Animations ────────────────────────────────────────────────────
    sb.animate(&dragon_curve_ref, "theta", |tb| {
        tb.keyframe_with_easing(
            0.0,
            AnimValue::Float(dragon_curve_theta),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            3.0,
            AnimValue::Float(dragon_curve_theta + 0.05),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            9.0,
            AnimValue::Float(dragon_curve_theta - 0.05),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            12.0,
            AnimValue::Float(dragon_curve_theta),
            easing::sine_in_out,
        )
    });

    sb.animate(&sierpinski_ref, "theta", |tb| {
        tb.keyframe_with_easing(0.0, AnimValue::Float(sierpinski_theta), easing::sine_in_out)
            .keyframe_with_easing(
                3.0,
                AnimValue::Float(sierpinski_theta + 0.05),
                easing::sine_in_out,
            )
            .keyframe_with_easing(
                9.0,
                AnimValue::Float(sierpinski_theta - 0.05),
                easing::sine_in_out,
            )
            .keyframe_with_easing(
                12.0,
                AnimValue::Float(sierpinski_theta),
                easing::sine_in_out,
            )
    });

    sb.animate(&koch_ref, "theta", |tb| {
        tb.keyframe_with_easing(0.0, AnimValue::Float(koch_theta), easing::sine_in_out)
            .keyframe_with_easing(
                3.0,
                AnimValue::Float(koch_theta + 0.05),
                easing::sine_in_out,
            )
            .keyframe_with_easing(
                9.0,
                AnimValue::Float(koch_theta - 0.05),
                easing::sine_in_out,
            )
            .keyframe_with_easing(12.0, AnimValue::Float(koch_theta), easing::sine_in_out)
    });

    sb.animate(&fractal_plant_ref, "theta", |tb| {
        tb.keyframe_with_easing(
            0.0,
            AnimValue::Float(fractal_plant_theta),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            3.0,
            AnimValue::Float(fractal_plant_theta + 0.05),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            9.0,
            AnimValue::Float(fractal_plant_theta - 0.05),
            easing::sine_in_out,
        )
        .keyframe_with_easing(
            12.0,
            AnimValue::Float(fractal_plant_theta),
            easing::sine_in_out,
        )
    });

    // sb.animate(&l_system_ref, "progress", |tb| {
    //     tb.keyframe_with_easing(32.0, AnimValue::Float(1.0), easing::quint_in_out)
    //         .keyframe(70.0, AnimValue::Float(0.0))
    //         .keyframe(72.0, AnimValue::Float(0.0))
    // });

    // sb.animate_camera("position", |tb| {
    //     tb.keyframe_with_easing(
    //         32.0,
    //         AnimValue::Vec3(vec3(0.0, 14.5, 25.7)),
    //         easing::sine_in_out,
    //     )
    //     .keyframe(70.0, AnimValue::Vec3(vec3(0.0, 1.3, 5.0)))
    // });

    // sb.animate_camera("target", |tb| {
    //     tb.keyframe_with_easing(
    //         32.0,
    //         AnimValue::Vec3(vec3(0.0, 14.5, 0.0)),
    //         easing::sine_in_out,
    //     )
    //     .keyframe(70.0, AnimValue::Vec3(vec3(0.0, 1.3, 0.0)))
    // });

    sb.build()
}
