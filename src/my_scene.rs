use std::f32::consts::{FRAC_PI_2, PI};

use macroquad::prelude::*;

use crate::animation::easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Sprite, Turtle};
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut sb = SceneBuilder::new();

    let texture = Texture2D::from_file_with_format(
        include_bytes!("../assets/aseprite-files/tutle.png"),
        None,
    );
    texture.set_filter(FilterMode::Nearest);
    let mut sprite = Sprite::new(texture, vec3(0.0, 0.0, 0.0), Some(vec2(1.0, 1.0)), WHITE);
    sprite.center = vec2(0.0, -0.5);

    let turtle = Turtle::new(sprite, vec![]);
    let turtle_ref = sb.add(turtle);

    sb.camera(Camera::new(vec3(1.5, 1.5, 8.0), vec3(1.5, 1.5, 0.0)));

    // Walk right → rotate up → walk up → rotate left → walk left
    // Position holds still during rotations, rotation holds still during walks.
    sb.parallel(|p| {
        p.animate(&turtle_ref, "position", |tb| {
            tb.keyframe(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 0.0)))
                // walk right
                .animate_for(
                    1.0,
                    AnimValue::Vec3(vec3(3.0, 0.0, 0.0)),
                    easing::sine_in_out,
                )
                // hold during rotation
                .animate_for(1.0, AnimValue::Vec3(vec3(3.0, 0.0, 0.0)), easing::linear)
                // walk up
                .animate_for(
                    1.0,
                    AnimValue::Vec3(vec3(3.0, 3.0, 0.0)),
                    easing::sine_in_out,
                )
                // hold during rotation
                .animate_for(1.0, AnimValue::Vec3(vec3(3.0, 3.0, 0.0)), easing::linear)
                // walk left
                .animate_for(
                    1.0,
                    AnimValue::Vec3(vec3(0.0, 3.0, 0.0)),
                    easing::sine_in_out,
                )
        });

        p.animate(&turtle_ref, "rotation", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.0))
                // hold during walk right
                .animate_for(1.0, AnimValue::Float(0.0), easing::linear)
                // rotate to face up
                .animate_for(1.0, AnimValue::Float(FRAC_PI_2), easing::sine_in_out)
                // hold during walk up
                .animate_for(1.0, AnimValue::Float(FRAC_PI_2), easing::linear)
                // rotate to face left
                .animate_for(1.0, AnimValue::Float(PI), easing::sine_in_out)
                // hold during walk left
                .animate_for(1.0, AnimValue::Float(PI), easing::linear)
        });
    });

    sb.build()
}
