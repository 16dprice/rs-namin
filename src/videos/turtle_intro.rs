use macroquad::prelude::*;

use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::l_system::{apply_rules, dragon_curve, get_lines};
use crate::scene::objects::{LSystem, Sprite, Turtle, VectorText};
use crate::scene::value::AnimValue;
use crate::scene_builder::SceneBuilder;

pub fn build() -> (Scene, Timeline, Camera) {
    // ── Scene ─────────────────────────────────────────────────────────
    let mut sb = SceneBuilder::new();

    let texture = Texture2D::from_file_with_format(include_bytes!("../../assets/aseprite-files/tutle.png"), None);
    texture.set_filter(FilterMode::Nearest);
    let mut sprite = Sprite::new(texture, vec3(0.0, 0.0, 0.0), Some(vec2(1.0, 1.0)), WHITE);
    sprite.center = vec2(0.0, -0.5);

    let (config, theta) = dragon_curve();
    let iterations = 7;
    let s = apply_rules(&config, iterations);
    let lines = get_lines(&s, theta, 1.0);

    let mut l_system = LSystem::new(config, theta, BLUE).with_colors(vec![RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE]);
    l_system.iterations = iterations as f32;
    l_system.scale = 1.0;

    let turtle = Turtle::new(sprite, lines.clone());

    let top_text_y = 8.0;
    let text_horizontal_spacing = 4.0;

    let mut alphabet_text = VectorText::from_latex(r"$V = \{ F,\, G \}$", WHITE);
    alphabet_text.scale = 1.4;
    alphabet_text.progress = 0.0;
    alphabet_text.stagger = 0.5;
    alphabet_text.position = vec3(13.0, top_text_y, 0.0);

    let mut axiom_text = VectorText::from_latex(r"$\omega = F$", WHITE);
    axiom_text.scale = 1.4;
    axiom_text.progress = 0.0;
    axiom_text.stagger = 0.5;
    axiom_text.position = vec3(13.0, top_text_y - 1.0 * text_horizontal_spacing, 0.0);

    let mut production_rules_text = VectorText::from_latex(r"$P = \{ F \rightarrow F + G,\; G \rightarrow F - G \}$", WHITE);
    production_rules_text.scale = 1.4;
    production_rules_text.progress = 0.0;
    production_rules_text.stagger = 0.5;
    production_rules_text.position = vec3(13.0, top_text_y - 2.0 * text_horizontal_spacing, 0.0);

    let mut theta_text = VectorText::from_latex(r"$\theta = 90^{\circ}$", WHITE);
    theta_text.scale = 1.4;
    theta_text.progress = 0.0;
    theta_text.stagger = 0.5;
    theta_text.position = vec3(13.0, top_text_y - 3.0 * text_horizontal_spacing, 0.0);

    let l_system_ref = sb.add(l_system);
    let turtle_ref = sb.add(turtle);
    let alphabet_text_ref = sb.add(alphabet_text);
    let axiom_text_ref = sb.add(axiom_text);
    let production_rules_text_ref = sb.add(production_rules_text);
    let theta_text_ref = sb.add(theta_text);

    let total_seconds_l_system_drawing_seconds = 1.0 * lines.len() as f32;
    let camera_start_zoom_out_time = 30.0;
    let camera_finish_zoom_out_time = camera_start_zoom_out_time + 5.0;
    let camera_position_final_z = 24.0;

    // ── Camera ────────────────────────────────────────────────────────
    sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), vec3(0.0, 0.0, 0.0)));

    // ── Drawing phase: camera, turtle, and l-system all animate from t=0 ──
    let seconds_per_segment = total_seconds_l_system_drawing_seconds / lines.len() as f32;

    sb.parallel(|p| {
        p.animate_camera("position", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Vec3(vec3(lines[0].start.x, lines[0].start.y, 6.0)));

            for (i, seg) in lines.iter().enumerate() {
                let time = (i + 1) as f32 * seconds_per_segment;
                if time > camera_start_zoom_out_time {
                    break;
                }
                tb = tb.animate_for(
                    seconds_per_segment,
                    AnimValue::Vec3(vec3(
                        seg.end.x,
                        seg.end.y,
                        (camera_position_final_z - 6.0) * ((i + 1) as f32 / lines.len() as f32) + 6.0,
                    )),
                    Easing::SineInOut,
                );
            }

            tb.keyframe_with_easing(
                camera_finish_zoom_out_time,
                AnimValue::Vec3(vec3(15.0, 2.0, camera_position_final_z)),
                Easing::SineInOut,
            )
        });

        p.animate_camera("target", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Vec3(vec3(lines[0].start.x, lines[0].start.y, 0.0)));

            for (i, seg) in lines.iter().enumerate() {
                let time = (i + 1) as f32 * seconds_per_segment;
                if time > camera_start_zoom_out_time {
                    break;
                }
                tb = tb.animate_for(
                    seconds_per_segment,
                    AnimValue::Vec3(vec3(seg.end.x, seg.end.y, 0.0)),
                    Easing::SineInOut,
                );
            }

            tb.keyframe_with_easing(
                camera_finish_zoom_out_time,
                AnimValue::Vec3(vec3(15.0, 2.0, 0.0)),
                Easing::SineInOut,
            )
        });

        p.animate(&turtle_ref, "progress", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Float(0.0));
            for i in 0..lines.len() {
                tb = tb.animate_for(
                    seconds_per_segment,
                    AnimValue::Float((i + 1) as f32 / lines.len() as f32),
                    Easing::SineInOut,
                )
            }
            tb
        });

        p.animate(&l_system_ref, "progress", |mut tb| {
            tb = tb.keyframe(0.0, AnimValue::Float(0.0));
            for i in 0..lines.len() {
                tb = tb.animate_for(
                    seconds_per_segment,
                    AnimValue::Float((i + 1) as f32 / lines.len() as f32),
                    Easing::SineInOut,
                )
            }
            tb
        });

        p.animate(&l_system_ref, "line_width", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.02))
                .keyframe(camera_finish_zoom_out_time, AnimValue::Float(0.15))
        });
    });

    // ── Text reveal phase: staggered progress animations ──────────────
    let text_sequence_start = camera_finish_zoom_out_time - 1.0;

    sb.set_cursor(text_sequence_start);
    sb.animate_seq(&alphabet_text_ref, "progress", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(3.0, AnimValue::Float(1.0), Easing::SineInOut)
    });

    sb.set_cursor(text_sequence_start + 1.0);
    sb.animate_seq(&axiom_text_ref, "progress", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(3.0, AnimValue::Float(1.0), Easing::SineInOut)
    });

    sb.set_cursor(text_sequence_start + 2.0);
    sb.animate_seq(&production_rules_text_ref, "progress", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(4.0, AnimValue::Float(1.0), Easing::SineInOut)
    });

    sb.set_cursor(text_sequence_start + 3.0);
    sb.animate_seq(&theta_text_ref, "progress", |tb| {
        tb.keyframe(0.0, AnimValue::Float(0.0))
            .animate_for(3.0, AnimValue::Float(1.0), Easing::SineInOut)
    });
    // cursor is now at theta_text_end equivalent

    // ── Scale pulse phase: sequential pulses highlighting each text ────
    sb.wait(4.0);

    sb.animate_seq(&alphabet_text_ref, "scale", |tb| {
        tb.keyframe(0.0, AnimValue::Float(1.4))
            .animate_for(1.0, AnimValue::Float(1.55), Easing::SineOut)
            .animate_for(1.0, AnimValue::Float(1.4), Easing::SineIn)
    });
    sb.wait(3.0);

    sb.animate_seq(&axiom_text_ref, "scale", |tb| {
        tb.keyframe(0.0, AnimValue::Float(1.4))
            .animate_for(1.0, AnimValue::Float(1.6), Easing::SineOut)
            .animate_for(1.0, AnimValue::Float(1.4), Easing::SineIn)
    });
    sb.wait(3.0);

    sb.animate_seq(&production_rules_text_ref, "scale", |tb| {
        tb.keyframe(0.0, AnimValue::Float(1.4))
            .animate_for(1.0, AnimValue::Float(1.5), Easing::SineOut)
            .animate_for(1.0, AnimValue::Float(1.4), Easing::SineIn)
    });
    sb.wait(3.0);

    sb.animate_seq(&theta_text_ref, "scale", |tb| {
        tb.keyframe(0.0, AnimValue::Float(1.4))
            .animate_for(1.0, AnimValue::Float(1.6), Easing::SineOut)
            .animate_for(1.0, AnimValue::Float(1.4), Easing::SineIn)
    });

    sb.build()
}
