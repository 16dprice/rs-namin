use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Text {
    pub content: String,
    pub position: Vec2,
    pub font_size: f32,
    pub color: Vec4,
    /// Fraction of the text to display, in [0.0, 1.0].
    /// 0.0 = nothing shown, 1.0 = full text. Reveals characters left-to-right.
    /// Values outside the range are clamped at draw time.
    pub progress: f32,
}

impl Text {
    pub fn new(content: impl Into<String>, position: Vec2, font_size: f32, color: Color) -> Self {
        Self {
            content: content.into(),
            position,
            font_size,
            color: vec4(color.r, color.g, color.b, color.a),
            progress: 1.0,
        }
    }

    /// Returns a string slice containing only the visible portion of the content.
    fn visible_text(&self) -> &str {
        let pct = self.progress.clamp(0.0, 1.0);
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
        draw_text(self.visible_text(), self.position.x, self.position.y, self.font_size, color);
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

animatable!(Text {
    position: Vec2,
    font_size: Float,
    color: Vec4,
    progress: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_text(content: &str, progress: f32) -> Text {
        let mut t = Text::new(content, vec2(0.0, 0.0), 24.0, WHITE);
        t.progress = progress;
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
    fn visible_text_clamps_out_of_range_progress() {
        assert_eq!(make_text("Hello", 1.5).visible_text(), "Hello");
        assert_eq!(make_text("Hello", -0.5).visible_text(), "");
    }

    #[test]
    fn is_screen_space_true() {
        let t = make_text("test", 1.0);
        assert!(t.is_screen_space());
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_text("Hello", 0.5));
    }
}
