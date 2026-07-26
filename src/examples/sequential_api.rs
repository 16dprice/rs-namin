use macroquad::prelude::*;

use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::{Disk, Line, Rectangle};
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

/// Demonstrates the sequential animation API: animate_seq, animate_for,
/// parallel, wait, set_cursor, and mixing with absolute-time animate.
pub fn build() -> (Scene, Timeline, Camera) {
    let mut sb = SceneBuilder::new();

    let white = vec4(1.0, 1.0, 1.0, 1.0);
    let red = vec4(1.0, 0.0, 0.0, 1.0);
    let green = vec4(0.0, 1.0, 0.0, 1.0);
    let blue = vec4(0.0, 0.0, 1.0, 1.0);
    let bg = vec4(0.15, 0.15, 0.15, 1.0);

    // ── Objects ───────────────────────────────────────────────────────
    let ground = sb.add(Line::new(vec3(-10.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), WHITE));

    let red_circle = sb.add(Disk::new(vec3(-3.0, 2.0, 0.0), 0.0, RED));
    let green_circle = sb.add(Disk::new(vec3(0.0, 2.0, 0.0), 0.0, GREEN));
    let blue_circle = sb.add(Disk::new(vec3(3.0, 2.0, 0.0), 0.0, BLUE));

    let backdrop = sb.add(Rectangle::new(vec3(0.0, 2.0, -0.1), vec2(12.0, 6.0), DARKGRAY));

    sb.camera(Camera::new(vec3(0.0, 2.5, 10.0), vec3(0.0, 2.0, 0.0)));

    // ── Phase 1: circles appear one after another (sequential) ────────
    // Each circle grows from radius 0 → 0.8 over 0.5s, with a 0.3s gap between.
    sb.animate_seq(&red_circle, "radius", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(0.5, AnimValue::Float(0.8), Easing::BackOut)
    });
    sb.wait(0.3);

    sb.animate_seq(&green_circle, "radius", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(0.5, AnimValue::Float(0.8), Easing::BackOut)
    });
    sb.wait(0.3);

    sb.animate_seq(&blue_circle, "radius", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(0.5, AnimValue::Float(0.8), Easing::BackOut)
    });
    // cursor is now at 2.1 (0.5 + 0.3 + 0.5 + 0.3 + 0.5)

    // ── Phase 2: all circles pulse in parallel ────────────────────────
    sb.wait(0.5);
    sb.parallel(|p| {
        p.animate(&red_circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.8))
                .animate_for(0.4, AnimValue::Float(1.2), Easing::SineOut)
                .animate_for(0.4, AnimValue::Float(0.8), Easing::SineIn)
        });
        p.animate(&green_circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.8))
                .animate_for(0.4, AnimValue::Float(1.2), Easing::SineOut)
                .animate_for(0.4, AnimValue::Float(0.8), Easing::SineIn)
        });
        p.animate(&blue_circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.8))
                .animate_for(0.4, AnimValue::Float(1.2), Easing::SineOut)
                .animate_for(0.4, AnimValue::Float(0.8), Easing::SineIn)
        });
    });
    // cursor is now at 3.4

    // ── Phase 3: sequential color wave ────────────────────────────────
    // Each circle briefly flashes white then returns, one after the other.
    sb.wait(0.3);

    sb.animate_seq(&red_circle, "color", |tb| {
        tb.keyframe(0.0, AnimValue::Vec4(red))
            .animate_for(0.2, AnimValue::Vec4(white), Easing::ExpoOut)
            .animate_for(0.3, AnimValue::Vec4(red), Easing::ExpoIn)
    });

    sb.animate_seq(&green_circle, "color", |tb| {
        tb.keyframe(0.0, AnimValue::Vec4(green))
            .animate_for(0.2, AnimValue::Vec4(white), Easing::ExpoOut)
            .animate_for(0.3, AnimValue::Vec4(green), Easing::ExpoIn)
    });

    sb.animate_seq(&blue_circle, "color", |tb| {
        tb.keyframe(0.0, AnimValue::Vec4(blue))
            .animate_for(0.2, AnimValue::Vec4(white), Easing::ExpoOut)
            .animate_for(0.3, AnimValue::Vec4(blue), Easing::ExpoIn)
    });

    // ── Camera: slow zoom using absolute-time animate ─────────────────
    // Runs from t=0 alongside everything else, showing that both APIs coexist.
    sb.animate_camera("position", |tb| {
        tb.keyframe(0.0, AnimValue::Vec3(vec3(0.0, 2.5, 10.0))).keyframe_with_easing(
            5.0,
            AnimValue::Vec3(vec3(0.0, 2.5, 8.0)),
            Easing::SineInOut,
        )
    });

    // ── Ground line fade-in: absolute-time animate (independent of cursor) ──
    sb.animate(&ground, "color", |tb| {
        tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 0.0)))
            .keyframe(1.0, AnimValue::Vec4(white))
    });

    // ── Backdrop fade-in: absolute-time animate ───────────────────────
    sb.animate(&backdrop, "color", |tb| {
        tb.keyframe(0.0, AnimValue::Vec4(vec4(bg.x, bg.y, bg.z, 0.0)))
            .keyframe_with_easing(1.5, AnimValue::Vec4(bg), Easing::SineOut)
    });

    sb.build()
}
