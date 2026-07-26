use macroquad::prelude::*;

use crate::scene::l_system::{self, LSystemConfig};
use crate::scene::polyline::{self, LineSegment, PolylineStyle, PolylineTransform, draw_polyline_mesh};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct LSystem {
    config: LSystemConfig,
    pub position: Vec3,
    pub color: Vec4,
    pub theta: f32,
    pub scale: f32,
    pub iterations: f32,
    pub progress: f32,
    pub line_width: f32,
    /// When 2+ colors are provided, segments interpolate through this gradient
    /// in draw order. When empty, falls back to `self.color`.
    colors: Vec<Vec4>,
}

impl LSystem {
    pub fn new(config: LSystemConfig, theta: f32, color: Color) -> Self {
        Self {
            config,
            position: Vec3::ZERO,
            color: vec4(color.r, color.g, color.b, color.a),
            theta,
            scale: 1.0,
            iterations: 3.0,
            progress: 1.0,
            line_width: 0.02,
            colors: Vec::new(),
        }
    }

    /// Set a gradient color list. Segments will interpolate through these
    /// colors in draw order. Pass an empty vec to revert to `self.color`.
    pub fn with_colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = colors.into_iter().map(|c| vec4(c.r, c.g, c.b, c.a)).collect();
        self
    }

    /// Total number of segments at full progress (for stable color mapping).
    fn total_segment_count(&self) -> usize {
        let iters = self.iterations.floor().max(0.0) as usize;
        let l_string = l_system::apply_rules(&self.config, iters);
        l_system::get_lines(&l_string, self.theta, 1.0).len()
    }

    fn get_segments(&self) -> Vec<LineSegment> {
        let iters = self.iterations.floor().max(0.0) as usize;
        let l_string = l_system::apply_rules(&self.config, iters);
        let all = l_system::get_lines(&l_string, self.theta, 1.0);
        polyline::take_progress(&all, self.progress)
    }
}

impl SceneObject for LSystem {
    fn draw(&self) {
        let segments = self.get_segments();
        // Full segment count for stable color mapping during progress animation.
        let color_total = self.total_segment_count();
        draw_polyline_mesh(
            &segments,
            &PolylineStyle {
                line_width: self.line_width,
                color: self.color,
                colors: &self.colors,
                color_total,
            },
            &PolylineTransform {
                position: self.position,
                scale: self.scale,
            },
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        let segments = self.get_segments();
        if segments.is_empty() {
            return BoundingBox {
                min: self.position,
                max: self.position,
            };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for seg in &segments {
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

animatable!(LSystem {
    position: Vec3,
    color: Vec4,
    theta: Float,
    scale: Float,
    iterations: Float,
    progress: Float,
    line_width: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::l_system::dragon_curve;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_lsystem() -> LSystem {
        let (config, theta) = dragon_curve();
        LSystem::new(config, theta, WHITE)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_lsystem());
    }

    #[test]
    fn progress_zero_yields_no_segments() {
        let mut ls = make_lsystem();
        ls.progress = 0.0;
        let segs = ls.get_segments();
        assert!(segs.is_empty());
    }

    #[test]
    fn progress_one_yields_segments() {
        let ls = make_lsystem();
        let segs = ls.get_segments();
        assert!(!segs.is_empty());
    }

    #[test]
    fn iterations_floor_behavior() {
        let mut ls = make_lsystem();
        ls.iterations = 3.7;
        let segs_3_7 = ls.get_segments();
        ls.iterations = 3.0;
        let segs_3 = ls.get_segments();
        assert_eq!(segs_3_7.len(), segs_3.len());
    }

    #[test]
    fn bounding_box_at_zero_progress() {
        let mut ls = make_lsystem();
        ls.progress = 0.0;
        let bb = ls.bounding_box();
        assert_eq!(bb.min, ls.position);
        assert_eq!(bb.max, ls.position);
    }

    #[test]
    fn bounding_box_scales_with_scale() {
        let mut ls = make_lsystem();
        ls.scale = 1.0;
        let bb1 = ls.bounding_box();
        ls.scale = 2.0;
        let bb2 = ls.bounding_box();
        let w1 = bb1.max.x - bb1.min.x;
        let w2 = bb2.max.x - bb2.min.x;
        assert!((w2 - w1 * 2.0).abs() < 1e-4);
    }
}
