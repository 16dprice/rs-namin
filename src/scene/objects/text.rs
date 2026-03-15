use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Text {
    pub content: String,
    pub position: Vec2,
    pub font_size: f32,
    pub color: Vec4,
    /// Fraction of the text to display, in [0.0, 1.0].
    /// 0.0 = nothing shown, 1.0 = full text. Reveals characters left-to-right.
    pub percentage_shown: f32,
}

impl Text {
    const PROPERTY_NAMES: &[&str] = &["position", "font_size", "color", "percentage_shown"];

    pub fn new(content: impl Into<String>, position: Vec2, font_size: f32, color: Color) -> Self {
        Self {
            content: content.into(),
            position,
            font_size,
            color: vec4(color.r, color.g, color.b, color.a),
            percentage_shown: 1.0,
        }
    }

    /// Returns a string slice containing only the visible portion of the content.
    fn visible_text(&self) -> &str {
        let pct = self.percentage_shown.clamp(0.0, 1.0);
        let total = self.content.chars().count();
        let visible = (pct * total as f32).floor() as usize;
        // Walk char_indices to find the byte offset at `visible` chars.
        let end = self
            .content
            .char_indices()
            .nth(visible)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len());
        &self.content[..end]
    }
}

impl SceneObject for Text {
    fn draw(&self) {
        let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
        draw_text(
            self.visible_text(),
            self.position.x,
            self.position.y,
            self.font_size,
            color,
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        // Screen-space object — no meaningful world-space bounding box.
        BoundingBox {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        }
    }

    fn is_screen_space(&self) -> bool {
        true
    }
}

impl Animatable for Text {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec2(self.position)),
            "font_size" => Some(AnimValue::Float(self.font_size)),
            "color" => Some(AnimValue::Vec4(self.color)),
            "percentage_shown" => Some(AnimValue::Float(self.percentage_shown)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec2(v)) => self.position = v,
            ("font_size", AnimValue::Float(v)) => self.font_size = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            ("percentage_shown", AnimValue::Float(v)) => self.percentage_shown = v.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_text(content: &str, pct: f32) -> Text {
        let mut t = Text::new(content, vec2(0.0, 0.0), 24.0, WHITE);
        t.percentage_shown = pct;
        t
    }

    #[test]
    fn visible_text_zero_shows_nothing() {
        let t = make_text("Hello", 0.0);
        assert_eq!(t.visible_text(), "");
    }

    #[test]
    fn visible_text_one_shows_all() {
        let t = make_text("Hello", 1.0);
        assert_eq!(t.visible_text(), "Hello");
    }

    #[test]
    fn visible_text_partial() {
        // 0.4 * 5 chars = 2.0 → floor = 2 → "He"
        let t = make_text("Hello", 0.4);
        assert_eq!(t.visible_text(), "He");
    }

    #[test]
    fn visible_text_empty_string() {
        let t = make_text("", 0.5);
        assert_eq!(t.visible_text(), "");
    }

    #[test]
    fn visible_text_unicode() {
        // Each char is one logical character even if multi-byte
        let t = make_text("héllo", 0.4); // 0.4 * 5 = 2.0 → "hé"
        assert_eq!(t.visible_text(), "hé");
    }

    #[test]
    fn is_screen_space_true() {
        let t = make_text("test", 1.0);
        assert!(t.is_screen_space());
    }

    #[test]
    fn animatable_round_trip_percentage_shown() {
        let mut t = make_text("Hello", 1.0);
        t.set("percentage_shown", AnimValue::Float(0.5));
        assert_eq!(t.get("percentage_shown"), Some(AnimValue::Float(0.5)));
    }

    #[test]
    fn animatable_clamps_percentage_shown_high() {
        let mut t = make_text("Hello", 1.0);
        t.set("percentage_shown", AnimValue::Float(1.5));
        assert_eq!(t.get("percentage_shown"), Some(AnimValue::Float(1.0)));
    }

    #[test]
    fn animatable_clamps_percentage_shown_low() {
        let mut t = make_text("Hello", 1.0);
        t.set("percentage_shown", AnimValue::Float(-0.5));
        assert_eq!(t.get("percentage_shown"), Some(AnimValue::Float(0.0)));
    }

    #[test]
    fn animatable_round_trip_position() {
        let mut t = make_text("Hi", 1.0);
        t.set("position", AnimValue::Vec2(vec2(100.0, 200.0)));
        assert_eq!(t.get("position"), Some(AnimValue::Vec2(vec2(100.0, 200.0))));
    }

    #[test]
    fn animatable_round_trip_font_size() {
        let mut t = make_text("Hi", 1.0);
        t.set("font_size", AnimValue::Float(48.0));
        assert_eq!(t.get("font_size"), Some(AnimValue::Float(48.0)));
    }

    #[test]
    fn animatable_property_names() {
        let t = make_text("X", 1.0);
        let names = t.property_names();
        assert!(names.contains(&"position"));
        assert!(names.contains(&"font_size"));
        assert!(names.contains(&"color"));
        assert!(names.contains(&"percentage_shown"));
    }
}
