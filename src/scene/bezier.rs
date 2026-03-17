use macroquad::prelude::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
}

impl CubicBezier {
    pub fn new(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Split the curve at parameter t using De Casteljau's algorithm.
    /// Returns (left, right) sub-curves.
    pub fn split_at(&self, t: f32) -> (CubicBezier, CubicBezier) {
        let p01 = self.p0.lerp(self.p1, t);
        let p12 = self.p1.lerp(self.p2, t);
        let p23 = self.p2.lerp(self.p3, t);
        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);
        let p0123 = p012.lerp(p123, t);

        (
            CubicBezier::new(self.p0, p01, p012, p0123),
            CubicBezier::new(p0123, p123, p23, self.p3),
        )
    }

    /// Extract the sub-curve for parameter range [a, b].
    pub fn subcurve(&self, a: f32, b: f32) -> CubicBezier {
        if b <= 0.0 || a >= 1.0 || a >= b {
            return CubicBezier::new(self.p0, self.p0, self.p0, self.p0);
        }
        let a = a.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);

        // Split at b, take left half
        let (left, _) = self.split_at(b);
        if a <= 0.0 {
            return left;
        }
        // Reparameterize: a' = a / b, then split and take right half
        let a_prime = a / b;
        let (_, right) = left.split_at(a_prime);
        right
    }

    /// Promote a TrueType quadratic bezier to cubic.
    /// Q(t) = (1-t)^2*p0 + 2(1-t)t*p1 + t^2*p2
    /// Cubic control points: p0, p0 + 2/3*(p1-p0), p2 + 2/3*(p1-p2), p2
    pub fn from_quadratic(p0: Vec2, p1: Vec2, p2: Vec2) -> Self {
        let cp1 = p0 + (p1 - p0) * (2.0 / 3.0);
        let cp2 = p2 + (p1 - p2) * (2.0 / 3.0);
        Self::new(p0, cp1, cp2, p2)
    }

    /// Evaluate point on curve at parameter t.
    pub fn evaluate(&self, t: f32) -> Vec2 {
        let u = 1.0 - t;
        self.p0 * (u * u * u)
            + self.p1 * (3.0 * u * u * t)
            + self.p2 * (3.0 * u * t * t)
            + self.p3 * (t * t * t)
    }
}

#[derive(Debug, Clone)]
pub struct BezierContour {
    pub segments: Vec<CubicBezier>,
    pub closed: bool,
}

impl BezierContour {
    /// Return a truncated contour visible up to `progress` (0.0..1.0).
    /// Progress maps linearly across segments.
    pub fn truncate(&self, progress: f32) -> BezierContour {
        if self.segments.is_empty() || progress <= 0.0 {
            return BezierContour {
                segments: vec![],
                closed: false,
            };
        }
        if progress >= 1.0 {
            return self.clone();
        }

        let total = self.segments.len() as f32;
        let visible_len = progress * total;
        let full_segments = visible_len.floor() as usize;
        let partial_t = visible_len.fract();

        let mut segments = Vec::with_capacity(full_segments + 1);

        // Add fully visible segments
        for seg in self.segments.iter().take(full_segments) {
            segments.push(*seg);
        }

        // Add partial segment
        if full_segments < self.segments.len() && partial_t > 0.0 {
            let (left, _) = self.segments[full_segments].split_at(partial_t);
            segments.push(left);
        }

        BezierContour {
            segments,
            closed: false, // truncated contour is never closed
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlyphOutline {
    pub contours: Vec<BezierContour>,
    pub advance_x: f32,
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::vec2;

    use super::*;

    fn sample_curve() -> CubicBezier {
        CubicBezier::new(
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(1.0, 1.0),
            vec2(0.0, 1.0),
        )
    }

    #[test]
    fn split_at_zero() {
        let c = sample_curve();
        let (left, right) = c.split_at(0.0);
        // Left is degenerate at p0
        assert_eq!(left.p0, c.p0);
        assert_eq!(left.p3, c.p0);
        // Right is the full curve
        assert_eq!(right.p0, c.p0);
        assert_eq!(right.p3, c.p3);
    }

    #[test]
    fn split_at_one() {
        let c = sample_curve();
        let (left, right) = c.split_at(1.0);
        // Left is the full curve
        assert_eq!(left.p0, c.p0);
        assert_eq!(left.p3, c.p3);
        // Right is degenerate at p3
        assert_eq!(right.p0, c.p3);
        assert_eq!(right.p3, c.p3);
    }

    #[test]
    fn split_at_half_meets_at_midpoint() {
        let c = sample_curve();
        let (left, right) = c.split_at(0.5);
        let mid = c.evaluate(0.5);
        assert!((left.p3 - mid).length() < 1e-5);
        assert!((right.p0 - mid).length() < 1e-5);
        assert_eq!(left.p0, c.p0);
        assert_eq!(right.p3, c.p3);
    }

    #[test]
    fn subcurve_endpoints_match_evaluate() {
        let c = CubicBezier::new(
            vec2(0.0, 0.0),
            vec2(4.0, 0.0),
            vec2(4.0, 4.0),
            vec2(0.0, 4.0),
        );
        let sub = c.subcurve(0.25, 0.75);
        let expected_start = c.evaluate(0.25);
        let expected_end = c.evaluate(0.75);
        assert!((sub.p0 - expected_start).length() < 1e-4);
        assert!((sub.p3 - expected_end).length() < 1e-4);
    }

    #[test]
    fn subcurve_full_range() {
        let c = sample_curve();
        let sub = c.subcurve(0.0, 1.0);
        assert!((sub.p0 - c.p0).length() < 1e-5);
        assert!((sub.p3 - c.p3).length() < 1e-5);
    }

    #[test]
    fn subcurve_degenerate_when_a_equals_b() {
        let c = sample_curve();
        let sub = c.subcurve(0.5, 0.5);
        assert!((sub.p0 - sub.p3).length() < 1e-5);
    }

    #[test]
    fn quadratic_promotion_endpoints() {
        let p0 = vec2(0.0, 0.0);
        let p1 = vec2(1.0, 2.0);
        let p2 = vec2(2.0, 0.0);
        let cubic = CubicBezier::from_quadratic(p0, p1, p2);
        assert_eq!(cubic.p0, p0);
        assert_eq!(cubic.p3, p2);
    }

    #[test]
    fn quadratic_promotion_midpoint_matches() {
        let p0 = vec2(0.0, 0.0);
        let p1 = vec2(1.0, 2.0);
        let p2 = vec2(2.0, 0.0);
        let cubic = CubicBezier::from_quadratic(p0, p1, p2);
        // Quadratic midpoint: (1-t)^2*p0 + 2(1-t)t*p1 + t^2*p2 at t=0.5
        let quad_mid = p0 * 0.25 + p1 * 0.5 + p2 * 0.25;
        let cubic_mid = cubic.evaluate(0.5);
        assert!((cubic_mid - quad_mid).length() < 1e-5);
    }

    #[test]
    fn truncate_empty_contour() {
        let contour = BezierContour {
            segments: vec![],
            closed: false,
        };
        let t = contour.truncate(0.5);
        assert!(t.segments.is_empty());
    }

    #[test]
    fn truncate_at_zero() {
        let contour = BezierContour {
            segments: vec![sample_curve()],
            closed: true,
        };
        let t = contour.truncate(0.0);
        assert!(t.segments.is_empty());
        assert!(!t.closed);
    }

    #[test]
    fn truncate_at_one_preserves_all() {
        let contour = BezierContour {
            segments: vec![sample_curve(), sample_curve()],
            closed: true,
        };
        let t = contour.truncate(1.0);
        assert_eq!(t.segments.len(), 2);
        assert!(t.closed);
    }

    #[test]
    fn truncate_half_of_two_segments() {
        let seg1 = CubicBezier::new(
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(1.0, 1.0),
            vec2(1.0, 1.0),
        );
        let seg2 = CubicBezier::new(
            vec2(1.0, 1.0),
            vec2(2.0, 1.0),
            vec2(2.0, 0.0),
            vec2(2.0, 0.0),
        );
        let contour = BezierContour {
            segments: vec![seg1, seg2],
            closed: false,
        };
        let t = contour.truncate(0.5);
        // 50% of 2 segments = 1 full segment
        assert_eq!(t.segments.len(), 1);
        assert!(!t.closed);
    }

    #[test]
    fn truncate_partial_segment() {
        let contour = BezierContour {
            segments: vec![sample_curve()],
            closed: true,
        };
        let t = contour.truncate(0.5);
        assert_eq!(t.segments.len(), 1);
        // The truncated segment should end at the midpoint of the original
        let expected_end = sample_curve().evaluate(0.5);
        assert!((t.segments[0].p3 - expected_end).length() < 1e-5);
    }

    #[test]
    fn evaluate_at_endpoints() {
        let c = sample_curve();
        assert!((c.evaluate(0.0) - c.p0).length() < 1e-6);
        assert!((c.evaluate(1.0) - c.p3).length() < 1e-6);
    }
}
