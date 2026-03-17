use macroquad::prelude::*;
use ttf_parser::Face;

use super::bezier::{BezierContour, CubicBezier, GlyphOutline};

const DEFAULT_FONT: &[u8] = include_bytes!("../../assets/fonts/Roboto-Regular.ttf");

pub fn default_font() -> &'static [u8] {
    DEFAULT_FONT
}

/// Extract glyph outlines from font data.
/// `scale` controls the size: 1.0 means 1 em = 1 world unit.
pub fn extract_glyphs(font_data: &[u8], text: &str, scale: f32) -> Vec<GlyphOutline> {
    let face = Face::parse(font_data, 0).expect("failed to parse font");
    let units_per_em = face.units_per_em() as f32;
    let font_scale = scale / units_per_em;

    let mut glyphs = Vec::new();
    let mut cursor_x: f32 = 0.0;

    for ch in text.chars() {
        if let Some(glyph_id) = face.glyph_index(ch) {
            let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 * font_scale;

            let mut collector = OutlineCollector::new(font_scale);

            if face.outline_glyph(glyph_id, &mut collector).is_some() {
                // Offset all contour points by cursor_x
                for contour in &mut collector.contours {
                    for seg in &mut contour.segments {
                        seg.p0.x += cursor_x;
                        seg.p1.x += cursor_x;
                        seg.p2.x += cursor_x;
                        seg.p3.x += cursor_x;
                    }
                }

                glyphs.push(GlyphOutline {
                    contours: collector.contours,
                    advance_x: advance,
                });
            } else {
                // Space or glyph without outlines
                glyphs.push(GlyphOutline {
                    contours: vec![],
                    advance_x: advance,
                });
            }

            cursor_x += advance;
        }
    }

    glyphs
}

struct OutlineCollector {
    contours: Vec<BezierContour>,
    current_segments: Vec<CubicBezier>,
    current_pos: Vec2,
    contour_start: Vec2,
    scale: f32,
}

impl OutlineCollector {
    fn new(scale: f32) -> Self {
        Self {
            contours: Vec::new(),
            current_segments: Vec::new(),
            current_pos: Vec2::ZERO,
            contour_start: Vec2::ZERO,
            scale,
        }
    }

    fn scaled(&self, x: f32, y: f32) -> Vec2 {
        vec2(x * self.scale, y * self.scale)
    }
}

impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        // Flush previous contour if any
        if !self.current_segments.is_empty() {
            self.contours.push(BezierContour {
                segments: std::mem::take(&mut self.current_segments),
                closed: true,
            });
        }
        let pos = self.scaled(x, y);
        self.current_pos = pos;
        self.contour_start = pos;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let end = self.scaled(x, y);
        // Line as degenerate cubic with control points along the line
        let p0 = self.current_pos;
        let p1 = p0.lerp(end, 1.0 / 3.0);
        let p2 = p0.lerp(end, 2.0 / 3.0);
        self.current_segments
            .push(CubicBezier::new(p0, p1, p2, end));
        self.current_pos = end;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let control = self.scaled(x1, y1);
        let end = self.scaled(x, y);
        self.current_segments
            .push(CubicBezier::from_quadratic(self.current_pos, control, end));
        self.current_pos = end;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let cp1 = self.scaled(x1, y1);
        let cp2 = self.scaled(x2, y2);
        let end = self.scaled(x, y);
        self.current_segments
            .push(CubicBezier::new(self.current_pos, cp1, cp2, end));
        self.current_pos = end;
    }

    fn close(&mut self) {
        // Close the contour: add a line back to start if needed
        if (self.current_pos - self.contour_start).length() > 1e-6 {
            let p0 = self.current_pos;
            let end = self.contour_start;
            let p1 = p0.lerp(end, 1.0 / 3.0);
            let p2 = p0.lerp(end, 2.0 / 3.0);
            self.current_segments
                .push(CubicBezier::new(p0, p1, p2, end));
        }
        self.contours.push(BezierContour {
            segments: std::mem::take(&mut self.current_segments),
            closed: true,
        });
        self.current_pos = self.contour_start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_glyph_has_contours() {
        let glyphs = extract_glyphs(default_font(), "A", 1.0);
        assert_eq!(glyphs.len(), 1);
        // 'A' should have contours (outer + inner triangle hole)
        assert!(!glyphs[0].contours.is_empty());
        assert!(glyphs[0].advance_x > 0.0);
    }

    #[test]
    fn multi_char_layout_advances() {
        let glyphs = extract_glyphs(default_font(), "AB", 1.0);
        assert_eq!(glyphs.len(), 2);
        // Both glyphs should have positive advance
        assert!(glyphs[0].advance_x > 0.0);
        assert!(glyphs[1].advance_x > 0.0);
        // Second glyph's contours should be offset by first glyph's advance
        if let Some(first_seg) = glyphs[1].contours.first().and_then(|c| c.segments.first()) {
            assert!(
                first_seg.p0.x >= glyphs[0].advance_x * 0.5,
                "second glyph should be offset"
            );
        }
    }

    #[test]
    fn empty_string_returns_empty() {
        let glyphs = extract_glyphs(default_font(), "", 1.0);
        assert!(glyphs.is_empty());
    }

    #[test]
    fn space_has_no_contours_but_has_advance() {
        let glyphs = extract_glyphs(default_font(), " ", 1.0);
        assert_eq!(glyphs.len(), 1);
        assert!(glyphs[0].contours.is_empty());
        assert!(glyphs[0].advance_x > 0.0);
    }

    #[test]
    fn scale_affects_size() {
        let small = extract_glyphs(default_font(), "A", 0.5);
        let big = extract_glyphs(default_font(), "A", 2.0);
        assert!(big[0].advance_x > small[0].advance_x);
    }

    #[test]
    fn contours_are_closed() {
        let glyphs = extract_glyphs(default_font(), "O", 1.0);
        for contour in &glyphs[0].contours {
            assert!(contour.closed, "font contours should be closed");
        }
    }
}
