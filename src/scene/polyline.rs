use macroquad::prelude::*;

use crate::scene::color::gradient_sample;
use crate::scene::mesh::{MeshBuilder, flat_vertex};

#[derive(Debug, Clone, Copy)]
pub struct LineSegment {
    pub start: Vec2,
    pub end: Vec2,
}

pub struct PolylineStyle<'a> {
    pub line_width: f32,
    /// Fallback color used when `colors` has fewer than 2 entries.
    pub color: Vec4,
    /// Gradient colors; when 2+ entries are provided each segment samples from here.
    pub colors: &'a [Vec4],
    /// Total segment count used for gradient mapping. Pass `segments.len()` to
    /// span the drawn range, or a stable larger count (e.g. segments at full
    /// progress) to anchor colors during progressive reveal animations.
    pub color_total: usize,
}

pub struct PolylineTransform {
    pub position: Vec3,
    pub scale: f32,
}

/// Return the first `progress` fraction of `segments`, with the last segment
/// partially drawn if `progress` lands mid-segment. `progress` is clamped to
/// `[0.0, 1.0]`.
pub fn take_progress(segments: &[LineSegment], progress: f32) -> Vec<LineSegment> {
    if segments.is_empty() {
        return Vec::new();
    }
    let exact = progress.clamp(0.0, 1.0) * segments.len() as f32;
    let full_count = (exact.floor() as usize).min(segments.len());
    let frac = exact - full_count as f32;

    let mut result = segments[..full_count].to_vec();
    if frac > 0.0 && full_count < segments.len() {
        let seg = &segments[full_count];
        result.push(LineSegment {
            start: seg.start,
            end: seg.start.lerp(seg.end, frac),
        });
    }
    result
}

/// Position and heading (radians) of the drawing tip at `progress`: the end
/// of the last revealed segment (`take_progress` semantics, so it moves
/// smoothly mid-segment), or the path's start at zero progress. The heading
/// always comes from the *full* segment under the pen — the partially-drawn
/// copy can be degenerate right at a step boundary. `None` for an empty
/// path. Shared by every object exposing `pen_position`/`pen_angle` outputs.
pub fn pen_pose(segments: &[LineSegment], progress: f32) -> Option<(Vec2, f32)> {
    let first = segments.first()?;
    let drawn = take_progress(segments, progress);
    let (point, segment) = match drawn.len() {
        0 => (first.start, first),
        n => (drawn[n - 1].end, &segments[n - 1]),
    };
    let d = segment.end - segment.start;
    Some((point, d.y.atan2(d.x)))
}

/// Tessellate `segments` into screen-aligned quads (one per segment) and emit
/// `draw_mesh` calls via `MeshBuilder`, which handles draw-call chunking.
/// Segment `i` is colored by sampling `style.colors` at position
/// `i / (style.color_total - 1)`, falling back to `style.color` when the
/// gradient has fewer than 2 entries.
pub fn draw_polyline_mesh(segments: &[LineSegment], style: &PolylineStyle, xform: &PolylineTransform) {
    if segments.is_empty() {
        return;
    }
    let half_w = style.line_width / 2.0;
    let z = xform.position.z;

    let mut mb = MeshBuilder::new();
    for (i, seg) in segments.iter().enumerate() {
        let color = gradient_sample(style.colors, style.color, i, style.color_total);
        let dir = seg.end - seg.start;
        let len = dir.length();
        let perp = if len > 1e-8 {
            let fwd = dir / len;
            vec2(-fwd.y, fwd.x)
        } else {
            vec2(0.0, 1.0)
        };
        let p = perp * half_w;

        let mk = |v: Vec2| {
            flat_vertex(
                vec3(v.x * xform.scale + xform.position.x, v.y * xform.scale + xform.position.y, z),
                vec2(0.0, 0.0),
                color,
            )
        };

        mb.quad([mk(seg.start - p), mk(seg.start + p), mk(seg.end + p), mk(seg.end - p)]);
    }
    mb.draw();
}

#[cfg(test)]
mod pen_pose_tests {
    use super::*;
    use macroquad::prelude::vec2;

    fn l_path() -> Vec<LineSegment> {
        // Right 2 units, then up 2 units.
        vec![
            LineSegment {
                start: vec2(0.0, 0.0),
                end: vec2(2.0, 0.0),
            },
            LineSegment {
                start: vec2(2.0, 0.0),
                end: vec2(2.0, 2.0),
            },
        ]
    }

    #[test]
    fn pen_pose_moves_smoothly_and_turns_with_segments() {
        let path = l_path();
        // Zero progress: path start, first segment's heading.
        let (p, a) = pen_pose(&path, 0.0).unwrap();
        assert_eq!(p, vec2(0.0, 0.0));
        assert!(a.abs() < 1e-6);
        // Mid first segment.
        let (p, a) = pen_pose(&path, 0.25).unwrap();
        assert!((p - vec2(1.0, 0.0)).length() < 1e-5);
        assert!(a.abs() < 1e-6);
        // Mid second segment: heading up.
        let (p, a) = pen_pose(&path, 0.75).unwrap();
        assert!((p - vec2(2.0, 1.0)).length() < 1e-5);
        assert!((a - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // Full.
        let (p, _) = pen_pose(&path, 1.0).unwrap();
        assert!((p - vec2(2.0, 2.0)).length() < 1e-5);
    }

    #[test]
    fn pen_pose_empty_path_is_none() {
        assert!(pen_pose(&[], 0.5).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(x0: f32, y0: f32, x1: f32, y1: f32) -> LineSegment {
        LineSegment {
            start: vec2(x0, y0),
            end: vec2(x1, y1),
        }
    }

    #[test]
    fn take_progress_empty_input() {
        let segs: Vec<LineSegment> = Vec::new();
        assert!(take_progress(&segs, 0.5).is_empty());
    }

    #[test]
    fn take_progress_zero_yields_empty() {
        let segs = vec![seg(0.0, 0.0, 1.0, 0.0), seg(1.0, 0.0, 2.0, 0.0)];
        assert!(take_progress(&segs, 0.0).is_empty());
    }

    #[test]
    fn take_progress_one_yields_all() {
        let segs = vec![seg(0.0, 0.0, 1.0, 0.0), seg(1.0, 0.0, 2.0, 0.0)];
        let out = take_progress(&segs, 1.0);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].end, vec2(2.0, 0.0));
    }

    #[test]
    fn take_progress_clamps_above_one() {
        let segs = vec![seg(0.0, 0.0, 1.0, 0.0), seg(1.0, 0.0, 2.0, 0.0)];
        let out = take_progress(&segs, 5.0);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn take_progress_renders_partial_final_segment() {
        // 2 segments, progress 0.25 → exact=0.5 → 0 full + 50% of first.
        let segs = vec![seg(0.0, 0.0, 0.0, 1.0), seg(0.0, 1.0, 0.0, 2.0)];
        let out = take_progress(&segs, 0.25);
        assert_eq!(out.len(), 1);
        assert!((out[0].end - vec2(0.0, 0.5)).length() < 1e-5);
        assert_eq!(out[0].start, vec2(0.0, 0.0));
    }

    #[test]
    fn take_progress_full_plus_partial() {
        // 2 segments, progress 0.75 → exact=1.5 → 1 full + 50% of second.
        let segs = vec![seg(0.0, 0.0, 0.0, 1.0), seg(0.0, 1.0, 0.0, 2.0)];
        let out = take_progress(&segs, 0.75);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].end, vec2(0.0, 1.0));
        assert!((out[1].end - vec2(0.0, 1.5)).length() < 1e-5);
    }

    #[test]
    fn take_progress_negative_clamped_to_zero() {
        let segs = vec![seg(0.0, 0.0, 1.0, 0.0)];
        assert!(take_progress(&segs, -0.5).is_empty());
    }
}
