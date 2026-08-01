use macroquad::prelude::*;

use crate::scene::polyline::{self, LineSegment, PolylineStyle, PolylineTransform, draw_polyline_mesh};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Polyline {
    pub segments: Vec<LineSegment>,
    pub position: Vec3,
    pub color: Vec4,
    pub scale: f32,
    pub progress: f32,
    pub line_width: f32,
    colors: Vec<Vec4>,
}

impl Polyline {
    pub fn new(segments: Vec<LineSegment>, color: Color) -> Self {
        Self {
            segments,
            position: Vec3::ZERO,
            color: vec4(color.r, color.g, color.b, color.a),
            scale: 1.0,
            progress: 1.0,
            line_width: 0.02,
            colors: Vec::new(),
        }
    }

    pub fn with_colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = colors.into_iter().map(|c| vec4(c.r, c.g, c.b, c.a)).collect();
        self
    }

    /// World-space drawing tip at the current progress (read-only output —
    /// a binding source).
    pub fn pen_position(&self) -> Vec3 {
        let local = polyline::pen_pose(&self.segments, self.progress).map_or(Vec2::ZERO, |(p, _)| p);
        vec3(
            local.x * self.scale + self.position.x,
            local.y * self.scale + self.position.y,
            self.position.z,
        )
    }

    /// Heading of the segment under the pen in radians.
    pub fn pen_angle(&self) -> f32 {
        polyline::pen_pose(&self.segments, self.progress).map_or(0.0, |(_, a)| a)
    }
}

impl SceneObject for Polyline {
    fn draw(&self) {
        let visible = polyline::take_progress(&self.segments, self.progress);
        draw_polyline_mesh(
            &visible,
            &PolylineStyle {
                line_width: self.line_width,
                color: self.color,
                colors: &self.colors,
                color_total: self.segments.len(),
            },
            &PolylineTransform {
                position: self.position,
                scale: self.scale,
            },
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        let visible = polyline::take_progress(&self.segments, self.progress);
        if visible.is_empty() {
            return BoundingBox {
                min: self.position,
                max: self.position,
            };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for seg in &visible {
            min_x = min_x.min(seg.start.x).min(seg.end.x);
            min_y = min_y.min(seg.start.y).min(seg.end.y);
            max_x = max_x.max(seg.start.x).max(seg.end.x);
            max_y = max_y.max(seg.start.y).max(seg.end.y);
        }

        BoundingBox {
            min: vec3(
                min_x * self.scale + self.position.x,
                min_y * self.scale + self.position.y,
                self.position.z,
            ),
            max: vec3(
                max_x * self.scale + self.position.x,
                max_y * self.scale + self.position.y,
                self.position.z,
            ),
        }
    }
}

animatable!(Polyline {
    position: Vec3,
    color: Vec4,
    scale: Float,
    progress: Float,
    line_width: Float,
} outputs {
    pen_position: Vec3,
    pen_angle: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn seg(x0: f32, y0: f32, x1: f32, y1: f32) -> LineSegment {
        LineSegment {
            start: vec2(x0, y0),
            end: vec2(x1, y1),
        }
    }

    fn make_polyline() -> Polyline {
        Polyline::new(vec![seg(0.0, 0.0, 1.0, 0.0), seg(1.0, 0.0, 1.0, 1.0)], WHITE)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_polyline());
    }

    #[test]
    fn progress_zero_empty_bbox() {
        let mut pl = make_polyline();
        pl.progress = 0.0;
        let bb = pl.bounding_box();
        assert_eq!(bb.min, pl.position);
        assert_eq!(bb.max, pl.position);
    }

    #[test]
    fn bounding_box_covers_segments() {
        let pl = make_polyline();
        let bb = pl.bounding_box();
        assert!(bb.max.x >= 1.0);
        assert!(bb.max.y >= 1.0);
    }
}
