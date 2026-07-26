use macroquad::prelude::*;

use crate::scene::color::gradient_sample;

#[derive(Debug, Clone, Copy)]
pub struct LineSegment {
    pub start: Vec2,
    pub end: Vec2,
}

/// Maximum line segments per `draw_mesh` call.
/// Each segment uses 4 vertices and 6 indices. Limit: 5000 indices / 6 = 833.
const MAX_SEGMENTS_PER_MESH: usize = 833;

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

/// Tessellate `segments` into screen-aligned quad strips and emit `draw_mesh`
/// calls. Segment `i` is colored by sampling `style.colors` at position
/// `i / (style.color_total - 1)`, falling back to `style.color` when the
/// gradient has fewer than 2 entries.
pub fn draw_polyline_mesh(segments: &[LineSegment], style: &PolylineStyle, xform: &PolylineTransform) {
    if segments.is_empty() {
        return;
    }
    let normal = vec4(0.0, 0.0, 1.0, 0.0);
    let half_w = style.line_width / 2.0;
    let z = xform.position.z;
    let visible = segments.len();

    for chunk_start in (0..visible).step_by(MAX_SEGMENTS_PER_MESH) {
        let chunk_end = (chunk_start + MAX_SEGMENTS_PER_MESH).min(visible);
        let chunk_len = chunk_end - chunk_start;

        let mut vertices = Vec::with_capacity(chunk_len * 4);
        let mut indices = Vec::with_capacity(chunk_len * 6);

        for (i, seg) in segments[chunk_start..chunk_end].iter().enumerate() {
            let seg_index = chunk_start + i;
            let color_bytes = gradient_sample(style.colors, style.color, seg_index, style.color_total);
            let dir = seg.end - seg.start;
            let len = dir.length();
            let perp = if len > 1e-8 {
                let fwd = dir / len;
                vec2(-fwd.y, fwd.x)
            } else {
                vec2(0.0, 1.0)
            };

            let base = vertices.len() as u16;
            let p = perp * half_w;

            let mk = |v: Vec2| Vertex {
                position: vec3(v.x * xform.scale + xform.position.x, v.y * xform.scale + xform.position.y, z),
                uv: vec2(0.0, 0.0),
                color: color_bytes,
                normal,
            };

            vertices.push(mk(seg.start - p));
            vertices.push(mk(seg.start + p));
            vertices.push(mk(seg.end + p));
            vertices.push(mk(seg.end - p));

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);
        }

        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
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
