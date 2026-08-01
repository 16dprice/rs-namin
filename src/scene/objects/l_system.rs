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

    /// Rewriting budget: rewriting reruns every frame, and user-authored
    /// rules can grow exponentially — expansion stops at the last iteration
    /// that fits this many chars (~at most that many segments).
    const MAX_REWRITE_LEN: usize = 200_000;

    fn rewritten(&self) -> String {
        let iters = self.iterations.floor().max(0.0) as usize;
        l_system::apply_rules_budgeted(&self.config, iters, Self::MAX_REWRITE_LEN)
    }

    /// Total number of segments at full progress (for stable color mapping).
    fn total_segment_count(&self) -> usize {
        l_system::get_lines(&self.rewritten(), self.theta, 1.0).len()
    }

    fn get_segments(&self) -> Vec<LineSegment> {
        let all = l_system::get_lines(&self.rewritten(), self.theta, 1.0);
        polyline::take_progress(&all, self.progress)
    }

    /// World-space position of the drawing tip ("pen") at the current
    /// progress: the end of the partially drawn path, or the path's start at
    /// zero progress. Exposed as a read-only output property so other
    /// objects can bind to it (e.g. a label following the drawing).
    pub fn pen_position(&self) -> Vec3 {
        let all = l_system::get_lines(&self.rewritten(), self.theta, 1.0);
        let local = polyline::pen_pose(&all, self.progress).map_or(Vec2::ZERO, |(p, _)| p);
        vec3(
            local.x * self.scale + self.position.x,
            local.y * self.scale + self.position.y,
            self.position.z,
        )
    }

    /// Heading of the drawing tip in radians (`atan2(dy, dx)` of the segment
    /// under the pen — the same convention as `Turtle`'s sprite rotation).
    /// At zero progress this is the first segment's heading. Exposed as a
    /// read-only output: bind a Sprite's `rotation` to it (plus
    /// `pen_position` for its position) to ride the drawing turtle-style.
    pub fn pen_angle(&self) -> f32 {
        let all = l_system::get_lines(&self.rewritten(), self.theta, 1.0);
        polyline::pen_pose(&all, self.progress).map_or(std::f32::consts::FRAC_PI_2, |(_, a)| a)
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
} outputs {
    pen_position: Vec3,
    pen_angle: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::l_system::dragon_curve;
    use crate::scene::traits::Animatable;
    use crate::scene::traits::test_support::assert_property_roundtrip;
    use crate::scene::value::AnimValue;

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
    fn pen_position_tracks_the_drawing_tip() {
        let mut ls = make_lsystem();

        // Full progress: pen sits at the last segment's end.
        ls.progress = 1.0;
        let last_end = ls.get_segments().last().unwrap().end;
        let pen = ls.pen_position();
        assert!((pen.x - last_end.x).abs() < 1e-5 && (pen.y - last_end.y).abs() < 1e-5);
        assert_eq!(pen.z, 0.0);

        // Mid progress: take_progress clips the last segment, so the pen
        // moves smoothly within a segment.
        ls.progress = 0.505;
        let partial_end = ls.get_segments().last().unwrap().end;
        let pen = ls.pen_position();
        assert!((pen.x - partial_end.x).abs() < 1e-5 && (pen.y - partial_end.y).abs() < 1e-5);

        // Zero progress: pen rests at the path's start.
        ls.progress = 1.0;
        let path_start = ls.get_segments()[0].start;
        ls.progress = 0.0;
        let pen = ls.pen_position();
        assert!((pen.x - path_start.x).abs() < 1e-5 && (pen.y - path_start.y).abs() < 1e-5);
    }

    #[test]
    fn pen_position_applies_scale_and_position() {
        let mut ls = make_lsystem();
        ls.progress = 1.0;
        let local = ls.get_segments().last().unwrap().end;
        ls.scale = 2.0;
        ls.position = vec3(1.0, -2.0, 3.0);
        let pen = ls.pen_position();
        assert!((pen.x - (local.x * 2.0 + 1.0)).abs() < 1e-5);
        assert!((pen.y - (local.y * 2.0 - 2.0)).abs() < 1e-5);
        assert_eq!(pen.z, 3.0);
    }

    #[test]
    fn pen_position_is_a_readable_output_not_a_settable_property() {
        let ls = make_lsystem();
        assert_eq!(ls.output_names(), &["pen_position", "pen_angle"]);
        assert!(matches!(ls.get("pen_position"), Some(AnimValue::Vec3(_))));
        assert!(!ls.property_names().contains(&"pen_position"));
    }

    #[test]
    fn pen_angle_follows_the_current_segment_heading() {
        let mut ls = make_lsystem();
        ls.iterations = 1.0; // "F+G": up, then (after a left turn) left.

        ls.progress = 0.0;
        assert!((ls.pen_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-5, "initial heading is up");
        ls.progress = 0.4; // mid first segment, still heading up
        assert!((ls.pen_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        ls.progress = 0.9; // on the second segment, heading left (π)
        assert!((ls.pen_angle().abs() - std::f32::consts::PI).abs() < 1e-4);

        // Scale and position don't affect heading.
        ls.scale = 3.0;
        ls.position = vec3(5.0, -2.0, 1.0);
        assert!((ls.pen_angle().abs() - std::f32::consts::PI).abs() < 1e-4);
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
