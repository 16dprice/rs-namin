use macroquad::prelude::*;
use std::fmt::Write;

use crate::scene::Scene;
use crate::scene::value::AnimValue;

const PANEL_COLOR: Color = Color::new(0.1, 0.1, 0.1, 0.85);
const TITLE_COLOR: Color = YELLOW;
const LABEL_COLOR: Color = Color::new(0.7, 0.7, 0.7, 1.0);
const VALUE_COLOR: Color = WHITE;

pub struct ValueInspector {
    pub visible: bool,
}

impl ValueInspector {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub fn draw(&self, scene: &Scene) {
        if !self.visible {
            return;
        }

        let font_size = 14.0;
        let line_h = 18.0;
        let panel_x = screen_width() - 260.0;
        let panel_margin = 10.0;
        let mut y = 30.0;

        // Count total lines for panel height
        let mut total_lines = 0_usize;
        for (_, obj) in scene.iter() {
            total_lines += 1; // header
            total_lines += obj.property_names().len();
            total_lines += 1; // spacing
        }

        if total_lines == 0 {
            return;
        }

        let panel_h = total_lines as f32 * line_h + panel_margin * 2.0;
        draw_rectangle(
            panel_x - panel_margin,
            y - line_h,
            260.0,
            panel_h,
            PANEL_COLOR,
        );

        for (id, obj) in scene.iter() {
            draw_text(
                &format!("Object {:?}", id),
                panel_x,
                y,
                font_size,
                TITLE_COLOR,
            );
            y += line_h;

            for name in obj.property_names() {
                let value_str = match obj.get(name) {
                    Some(v) => format_value(&v),
                    None => "???".to_string(),
                };
                draw_text(
                    &format!("  {}: {}", name, value_str),
                    panel_x,
                    y,
                    font_size,
                    if !name.is_empty() {
                        LABEL_COLOR
                    } else {
                        VALUE_COLOR
                    },
                );
                y += line_h;
            }
            y += line_h; // spacing between objects
        }
    }
}

impl Default for ValueInspector {
    fn default() -> Self {
        Self::new()
    }
}

fn format_value(value: &AnimValue) -> String {
    match value {
        AnimValue::Float(f) => format!("{:.2}", f),
        AnimValue::Vec2(v) => format!("({:.1}, {:.1})", v.x, v.y),
        AnimValue::Vec3(v) => format!("({:.1}, {:.1}, {:.1})", v.x, v.y, v.z),
        AnimValue::Vec4(v) => format!("({:.2}, {:.2}, {:.2}, {:.2})", v.x, v.y, v.z, v.w),
        AnimValue::Bool(b) => b.to_string(),
        AnimValue::Transform2D(t) => {
            let mut s = String::new();
            let _ = write!(
                s,
                "pos({:.1},{:.1}) rot={:.1} scl({:.1},{:.1})",
                t.position.x, t.position.y, t.rotation, t.scale.x, t.scale.y
            );
            s
        }
        AnimValue::Mat4(m) => {
            let cols = m.to_cols_array();
            format!(
                "[{:.1},{:.1},{:.1},{:.1}; ...]",
                cols[0], cols[1], cols[2], cols[3]
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::value::Transform2D;

    #[test]
    fn format_float() {
        assert_eq!(format_value(&AnimValue::Float(3.14159)), "3.14");
    }

    #[test]
    fn format_vec3() {
        let v = AnimValue::Vec3(vec3(1.0, 2.5, 3.0));
        assert_eq!(format_value(&v), "(1.0, 2.5, 3.0)");
    }

    #[test]
    fn format_vec4() {
        let v = AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0));
        assert_eq!(format_value(&v), "(1.00, 0.00, 0.00, 1.00)");
    }

    #[test]
    fn format_bool() {
        assert_eq!(format_value(&AnimValue::Bool(true)), "true");
    }

    #[test]
    fn format_vec2() {
        let v = AnimValue::Vec2(vec2(10.0, 20.0));
        assert_eq!(format_value(&v), "(10.0, 20.0)");
    }

    #[test]
    fn format_transform2d() {
        let t = AnimValue::Transform2D(Transform2D {
            position: vec2(1.0, 2.0),
            rotation: 45.0,
            scale: vec2(1.0, 1.0),
        });
        assert_eq!(format_value(&t), "pos(1.0,2.0) rot=45.0 scl(1.0,1.0)");
    }
}
