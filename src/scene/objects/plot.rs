use macroquad::prelude::*;

use crate::scene::expr::{self, Expr};
use crate::scene::polyline::{self, LineSegment, PolylineStyle, PolylineTransform, draw_polyline_mesh};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

/// A function plot: axes with auto-spaced ticks and the curve `y = f(x)`
/// sampled over `x_bounds`, drawn into a `size`-sized rectangle centered on
/// `position`. The function comes from an expression string in `x` (see
/// `scene::expr`); an unparsable expression plots axes only.
///
/// `x_bounds`/`y_bounds` are animatable (keyframe or bind them to zoom/pan
/// the plot window), `progress` reveals the curve left-to-right, and the
/// read-only `pen_position` output is the world-space tip of the revealed
/// curve — bind a marker or label to it to have something ride the graph.
pub struct Plot {
    /// Source text of the function, kept for the editor. Use
    /// [`set_expression`](Self::set_expression) to change it.
    expression: String,
    /// Parsed function; `None` when `expression` doesn't parse.
    expr: Option<Expr>,
    /// Curve samples across `x_bounds` (structural, not animatable).
    pub samples: usize,
    pub position: Vec3,
    /// World-space width/height of the plot rectangle.
    pub size: Vec2,
    /// Function-space window: (min, max) along each axis.
    pub x_bounds: Vec2,
    pub y_bounds: Vec2,
    pub color: Vec4,
    pub axis_color: Vec4,
    pub line_width: f32,
    pub axis_width: f32,
    pub progress: f32,
}

impl Plot {
    pub fn new(expression: &str, position: Vec3, size: Vec2, x_bounds: Vec2, y_bounds: Vec2, color: Color) -> Self {
        Self {
            expression: expression.to_string(),
            expr: expr::parse(expression).ok(),
            samples: 200,
            position,
            size,
            x_bounds,
            y_bounds,
            color: vec4(color.r, color.g, color.b, color.a),
            axis_color: vec4(0.6, 0.6, 0.6, 1.0),
            line_width: 0.05,
            axis_width: 0.025,
            progress: 1.0,
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Replace the plotted function. Returns whether the new source parses
    /// (on failure the plot keeps rendering, axes only).
    pub fn set_expression(&mut self, source: &str) -> bool {
        self.expression = source.to_string();
        self.expr = expr::parse(source).ok();
        self.expr.is_some()
    }

    /// Map a function-space point into the local (centered) plot rectangle.
    fn map_point(&self, x: f32, y: f32) -> Vec2 {
        let u = (x - self.x_bounds.x) / (self.x_bounds.y - self.x_bounds.x);
        let v = (y - self.y_bounds.x) / (self.y_bounds.y - self.y_bounds.x);
        vec2((u - 0.5) * self.size.x, (v - 0.5) * self.size.y)
    }

    fn bounds_are_valid(&self) -> bool {
        self.x_bounds.y > self.x_bounds.x && self.y_bounds.y > self.y_bounds.x
    }

    /// The full curve as local-space segments, clipped to `y_bounds`.
    /// Sample pairs straddling a bound are cut at the crossing; pairs fully
    /// outside (including asymptote jumps across the window) and non-finite
    /// samples produce no segment, splitting the curve into visible pieces.
    fn curve_segments(&self) -> Vec<LineSegment> {
        let Some(expr) = &self.expr else { return Vec::new() };
        if !self.bounds_are_valid() {
            return Vec::new();
        }
        let n = self.samples.max(2);
        let (y_min, y_max) = (self.y_bounds.x, self.y_bounds.y);
        let sample = |i: usize| {
            let t = i as f32 / (n - 1) as f32;
            let x = self.x_bounds.x + t * (self.x_bounds.y - self.x_bounds.x);
            (x, expr.eval(x))
        };

        let mut segments = Vec::new();
        let mut previous = sample(0);
        for i in 1..n {
            let current = sample(i);
            let (x0, y0) = previous;
            let (x1, y1) = current;
            previous = current;
            if !y0.is_finite() || !y1.is_finite() {
                continue;
            }
            let in0 = (y_min..=y_max).contains(&y0);
            let in1 = (y_min..=y_max).contains(&y1);
            // Cut a straddling pair at the bound it crosses.
            let crossing = |bound: f32| {
                let t = (bound - y0) / (y1 - y0);
                (x0 + t * (x1 - x0), bound)
            };
            let (start, end) = match (in0, in1) {
                (true, true) => ((x0, y0), (x1, y1)),
                (true, false) => ((x0, y0), crossing(if y1 > y_max { y_max } else { y_min })),
                (false, true) => (crossing(if y0 > y_max { y_max } else { y_min }), (x1, y1)),
                (false, false) => continue,
            };
            segments.push(LineSegment {
                start: self.map_point(start.0, start.1),
                end: self.map_point(end.0, end.1),
            });
        }
        segments
    }

    /// Axis lines and ticks, in local space. Each axis sits at the other
    /// coordinate's zero, clamped into the window when zero is out of view.
    fn axis_segments(&self) -> Vec<LineSegment> {
        if !self.bounds_are_valid() {
            return Vec::new();
        }
        let (half_w, half_h) = (self.size.x / 2.0, self.size.y / 2.0);
        let x_axis_y = self.map_point(0.0, 0.0f32.clamp(self.y_bounds.x, self.y_bounds.y)).y;
        let y_axis_x = self.map_point(0.0f32.clamp(self.x_bounds.x, self.x_bounds.y), 0.0).x;
        let tick = (self.size.x.min(self.size.y) * 0.03).max(self.axis_width * 2.0);

        let mut segments = vec![
            LineSegment {
                start: vec2(-half_w, x_axis_y),
                end: vec2(half_w, x_axis_y),
            },
            LineSegment {
                start: vec2(y_axis_x, -half_h),
                end: vec2(y_axis_x, half_h),
            },
        ];

        let step_x = tick_step(self.x_bounds.y - self.x_bounds.x);
        let mut x = (self.x_bounds.x / step_x).ceil() * step_x;
        while x <= self.x_bounds.y {
            if x.abs() > step_x * 0.5 {
                let cx = self.map_point(x, 0.0).x;
                segments.push(LineSegment {
                    start: vec2(cx, x_axis_y - tick),
                    end: vec2(cx, x_axis_y + tick),
                });
            }
            x += step_x;
        }
        let step_y = tick_step(self.y_bounds.y - self.y_bounds.x);
        let mut y = (self.y_bounds.x / step_y).ceil() * step_y;
        while y <= self.y_bounds.y {
            if y.abs() > step_y * 0.5 {
                let cy = self.map_point(0.0, y).y;
                segments.push(LineSegment {
                    start: vec2(y_axis_x - tick, cy),
                    end: vec2(y_axis_x + tick, cy),
                });
            }
            y += step_y;
        }
        segments
    }

    /// World-space tip of the revealed curve (the last point drawn at the
    /// current progress), falling back to the curve start, then the plot
    /// center. Exposed as a read-only output property for bindings.
    pub fn pen_position(&self) -> Vec3 {
        let curve = self.curve_segments();
        let local = polyline::pen_pose(&curve, self.progress).map_or(Vec2::ZERO, |(p, _)| p);
        vec3(local.x + self.position.x, local.y + self.position.y, self.position.z)
    }
}

impl Plot {
    /// Heading of the curve at the pen in radians (`atan2` of the last
    /// revealed segment in plot space — the same convention as the
    /// L-system's `pen_angle`). At zero progress it is the first segment's
    /// direction; 0 with no curve. Bind a Sprite's `rotation` to it (plus
    /// `pen_position`) to ride the graph facing along it.
    pub fn pen_angle(&self) -> f32 {
        let curve = self.curve_segments();
        polyline::pen_pose(&curve, self.progress).map_or(0.0, |(_, a)| a)
    }
}

/// A "nice" tick interval (1/2/5 ladder) yielding roughly 4–10 ticks per
/// span.
fn tick_step(span: f32) -> f32 {
    let raw = span / 8.0;
    let magnitude = 10.0f32.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let step = if normalized < 1.5 {
        1.0
    } else if normalized < 3.5 {
        2.0
    } else if normalized < 7.5 {
        5.0
    } else {
        10.0
    };
    step * magnitude
}

impl SceneObject for Plot {
    fn draw(&self) {
        let transform = PolylineTransform {
            position: self.position,
            scale: 1.0,
        };
        let axes = self.axis_segments();
        draw_polyline_mesh(
            &axes,
            &PolylineStyle {
                line_width: self.axis_width,
                color: self.axis_color,
                colors: &[],
                color_total: axes.len(),
            },
            &transform,
        );

        let curve = self.curve_segments();
        let drawn = polyline::take_progress(&curve, self.progress);
        draw_polyline_mesh(
            &drawn,
            &PolylineStyle {
                line_width: self.line_width,
                color: self.color,
                colors: &[],
                color_total: curve.len(),
            },
            &transform,
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        let half = vec3(self.size.x / 2.0, self.size.y / 2.0, 0.0);
        BoundingBox {
            min: self.position - half,
            max: self.position + half,
        }
    }
}

animatable!(Plot {
    position: Vec3,
    size: Vec2,
    x_bounds: Vec2,
    y_bounds: Vec2,
    color: Vec4,
    axis_color: Vec4,
    line_width: Float,
    axis_width: Float,
    progress: Float,
} outputs {
    pen_position: Vec3,
    pen_angle: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::Animatable;
    use crate::scene::traits::test_support::assert_property_roundtrip;
    use crate::scene::value::AnimValue;

    fn identity_plot() -> Plot {
        // y = x over a symmetric window mapped onto a 2x2 rect: function
        // space and local space coincide.
        Plot::new("x", Vec3::ZERO, vec2(2.0, 2.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), WHITE)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut identity_plot());
    }

    #[test]
    fn curve_follows_the_function() {
        let plot = identity_plot();
        let segments = plot.curve_segments();
        assert_eq!(segments.len(), plot.samples - 1);
        let first = segments.first().unwrap();
        let last = segments.last().unwrap();
        assert!((first.start - vec2(-1.0, -1.0)).length() < 1e-4);
        assert!((last.end - vec2(1.0, 1.0)).length() < 1e-4);
        // y = x: every point sits on the diagonal.
        for segment in &segments {
            assert!((segment.start.x - segment.start.y).abs() < 1e-4);
        }
    }

    #[test]
    fn curve_is_clipped_to_y_bounds() {
        let mut plot = identity_plot();
        // A constant above the window: nothing to draw.
        plot.set_expression("5");
        assert!(plot.curve_segments().is_empty());

        // y = x with a halved window: clipped at the crossing, not clamped.
        plot.set_expression("x");
        plot.y_bounds = vec2(-0.5, 0.5);
        let segments = plot.curve_segments();
        assert!(!segments.is_empty());
        for segment in &segments {
            assert!(segment.start.y >= -1.0 - 1e-4 && segment.end.y <= 1.0 + 1e-4);
        }
        // The clipped curve spans exactly half the x range (|x| <= 0.5
        // mapped onto the 2-wide rect).
        let min_x = segments.iter().map(|s| s.start.x).fold(f32::MAX, f32::min);
        assert!((min_x + 0.5).abs() < 0.05, "clip should start near x=-0.5, got {min_x}");
    }

    #[test]
    fn non_finite_samples_split_the_curve() {
        let mut plot = identity_plot();
        plot.set_expression("1 / x");
        plot.y_bounds = vec2(-10.0, 10.0);
        let segments = plot.curve_segments();
        assert!(!segments.is_empty());
        // No segment may jump across the asymptote through y=0 between the
        // branches (the two branches never connect).
        for segment in &segments {
            assert!(
                (segment.start.x < 0.0) == (segment.end.x < 0.0),
                "segment crosses the asymptote: {:?} -> {:?}",
                segment.start,
                segment.end
            );
        }
    }

    #[test]
    fn invalid_expression_plots_axes_only() {
        let mut plot = identity_plot();
        assert!(!plot.set_expression("foo(x"));
        assert!(plot.curve_segments().is_empty());
        assert!(!plot.axis_segments().is_empty());
        assert_eq!(plot.expression(), "foo(x");
    }

    #[test]
    fn degenerate_bounds_draw_nothing() {
        let mut plot = identity_plot();
        plot.x_bounds = vec2(2.0, 2.0);
        assert!(plot.curve_segments().is_empty());
        assert!(plot.axis_segments().is_empty());
    }

    #[test]
    fn pen_position_tracks_the_reveal() {
        let mut plot = identity_plot();
        plot.position = vec3(10.0, 0.0, 2.0);

        plot.progress = 0.0;
        let pen = plot.pen_position();
        assert!((pen - vec3(9.0, -1.0, 2.0)).length() < 1e-3, "pen at curve start, got {pen}");

        plot.progress = 0.5;
        let pen = plot.pen_position();
        assert!((pen - vec3(10.0, 0.0, 2.0)).length() < 0.05, "pen mid-curve, got {pen}");

        plot.progress = 1.0;
        let pen = plot.pen_position();
        assert!((pen - vec3(11.0, 1.0, 2.0)).length() < 1e-3, "pen at curve end, got {pen}");
    }

    #[test]
    fn pen_position_is_an_output() {
        let plot = identity_plot();
        assert_eq!(plot.output_names(), &["pen_position", "pen_angle"]);
        assert!(matches!(plot.get("pen_position"), Some(AnimValue::Vec3(_))));
    }

    #[test]
    fn pen_angle_follows_the_curve_slope() {
        // y = x on the identity mapping: slope 1 everywhere, angle = pi/4.
        let mut plot = identity_plot();
        for progress in [0.0, 0.5, 1.0] {
            plot.progress = progress;
            assert!(
                (plot.pen_angle() - std::f32::consts::FRAC_PI_4).abs() < 1e-3,
                "progress {progress}: {}",
                plot.pen_angle()
            );
        }

        // A constant function runs flat.
        plot.set_expression("0.5");
        assert!(plot.pen_angle().abs() < 1e-3);

        // No curve at all (parse failure): a stable 0.
        plot.set_expression("wat(");
        assert_eq!(plot.pen_angle(), 0.0);
    }

    #[test]
    fn axes_clamp_to_window_when_zero_is_out_of_view() {
        let mut plot = identity_plot();
        plot.x_bounds = vec2(2.0, 4.0);
        plot.y_bounds = vec2(1.0, 3.0);
        let axes = plot.axis_segments();
        // Both axes hug the low edges of the rect.
        assert!((axes[0].start.y + 1.0).abs() < 1e-4, "x-axis clamped to bottom");
        assert!((axes[1].start.x + 1.0).abs() < 1e-4, "y-axis clamped to left");
    }

    #[test]
    fn tick_step_uses_1_2_5_ladder() {
        for (span, expected) in [(8.0, 1.0), (16.0, 2.0), (40.0, 5.0), (0.8, 0.1), (100.0, 10.0)] {
            let step = tick_step(span);
            assert!((step - expected).abs() < 1e-5, "tick_step({span}) = {step}, expected {expected}");
        }
    }

    #[test]
    fn bounding_box_is_the_plot_rect() {
        let mut plot = identity_plot();
        plot.position = vec3(3.0, 1.0, 0.5);
        plot.size = vec2(4.0, 2.0);
        let bb = plot.bounding_box();
        assert_eq!(bb.min, vec3(1.0, 0.0, 0.5));
        assert_eq!(bb.max, vec3(5.0, 2.0, 0.5));
    }
}
