use macroquad::prelude::*;

use crate::scene::l_system::{self, LSystemConfig};
use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

/// Maximum line segments per draw_mesh call.
/// Each segment uses 4 vertices and 6 indices. Limit: 5000 indices / 6 = 833.
const MAX_SEGMENTS_PER_MESH: usize = 833;

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
    const PROPERTY_NAMES: &[&str] = &[
        "position",
        "color",
        "theta",
        "scale",
        "iterations",
        "progress",
        "line_width",
    ];

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
        self.colors = colors
            .into_iter()
            .map(|c| vec4(c.r, c.g, c.b, c.a))
            .collect();
        self
    }

    fn color_for_segment(&self, seg_index: usize, total: usize) -> [u8; 4] {
        let c = if self.colors.len() >= 2 {
            let t = if total <= 1 {
                0.0
            } else {
                seg_index as f32 / (total - 1) as f32
            };
            let span = (self.colors.len() - 1) as f32;
            let pos = t * span;
            let lo = (pos.floor() as usize).min(self.colors.len() - 2);
            let frac = pos - lo as f32;
            self.colors[lo].lerp(self.colors[lo + 1], frac)
        } else {
            self.color
        };
        Color::new(c.x, c.y, c.z, c.w).into()
    }

    /// Total number of segments at full progress (for stable color mapping).
    fn total_segment_count(&self) -> usize {
        let iters = self.iterations.floor().max(0.0) as usize;
        let l_string = l_system::apply_rules(&self.config, iters);
        l_system::get_lines(&l_string, self.theta, 1.0).len()
    }

    fn get_segments(&self) -> Vec<l_system::LineSegment> {
        let iters = self.iterations.floor().max(0.0) as usize;
        let l_string = l_system::apply_rules(&self.config, iters);
        let all = l_system::get_lines(&l_string, self.theta, 1.0);
        if all.is_empty() {
            return Vec::new();
        }

        let exact = self.progress.clamp(0.0, 1.0) * all.len() as f32;
        let full_count = (exact.floor() as usize).min(all.len());
        let frac = exact - full_count as f32;

        let mut result = all[..full_count].to_vec();

        // Partially draw the next segment based on the fractional remainder.
        if frac > 0.0 && full_count < all.len() {
            let seg = &all[full_count];
            result.push(l_system::LineSegment {
                start: seg.start,
                end: seg.start.lerp(seg.end, frac),
            });
        }

        result
    }
}

impl SceneObject for LSystem {
    fn draw(&self) {
        let segments = self.get_segments();
        if segments.is_empty() {
            return;
        }

        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let half_w = self.line_width / 2.0;
        let z = self.position.z;
        // Full segment count for stable color mapping; visible count for iteration.
        let color_total = self.total_segment_count();
        let visible = segments.len();

        for chunk_start in (0..visible).step_by(MAX_SEGMENTS_PER_MESH) {
            let chunk_end = (chunk_start + MAX_SEGMENTS_PER_MESH).min(visible);
            let chunk_len = chunk_end - chunk_start;

            let mut vertices = Vec::with_capacity(chunk_len * 4);
            let mut indices = Vec::with_capacity(chunk_len * 6);

            for (i, seg) in segments[chunk_start..chunk_end].iter().enumerate() {
                let seg_index = chunk_start + i;
                let color_bytes = self.color_for_segment(seg_index, color_total);
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
                    position: vec3(
                        v.x * self.scale + self.position.x,
                        v.y * self.scale + self.position.y,
                        z,
                    ),
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

impl Animatable for LSystem {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "color" => Some(AnimValue::Vec4(self.color)),
            "theta" => Some(AnimValue::Float(self.theta)),
            "scale" => Some(AnimValue::Float(self.scale)),
            "iterations" => Some(AnimValue::Float(self.iterations)),
            "progress" => Some(AnimValue::Float(self.progress)),
            "line_width" => Some(AnimValue::Float(self.line_width)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            ("theta", AnimValue::Float(v)) => self.theta = v,
            ("scale", AnimValue::Float(v)) => self.scale = v,
            ("iterations", AnimValue::Float(v)) => self.iterations = v,
            ("progress", AnimValue::Float(v)) => self.progress = v,
            ("line_width", AnimValue::Float(v)) => self.line_width = v,
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
    use crate::scene::l_system::dragon_curve;

    fn make_lsystem() -> LSystem {
        let (config, theta) = dragon_curve();
        LSystem::new(config, theta, WHITE)
    }

    #[test]
    fn property_round_trip() {
        let mut ls = make_lsystem();
        for name in LSystem::PROPERTY_NAMES {
            let val = ls.get(name).unwrap();
            ls.set(name, val.clone());
            assert_eq!(ls.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn unknown_property_returns_none() {
        let ls = make_lsystem();
        assert!(ls.get("nonexistent").is_none());
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

    #[test]
    fn gradient_colors_interpolate() {
        let (config, theta) = dragon_curve();
        let ls = LSystem::new(config, theta, WHITE).with_colors(vec![RED, BLUE]);
        let total = ls.get_segments().len();
        assert!(total > 2);

        let first = ls.color_for_segment(0, total);
        assert!(
            first[0] > first[2],
            "first segment should have more red than blue"
        );

        let last = ls.color_for_segment(total - 1, total);
        assert!(
            last[2] > last[0],
            "last segment should have more blue than red"
        );

        let mid = ls.color_for_segment(total / 2, total);
        assert!(
            mid[0] > 0 && mid[2] > 0,
            "mid segment should have both red and blue"
        );
    }

    #[test]
    fn empty_gradient_uses_color_field() {
        let ls = make_lsystem();
        assert!(ls.colors.is_empty());
        let total = ls.get_segments().len();
        let c0 = ls.color_for_segment(0, total);
        let c1 = ls.color_for_segment(total - 1, total);
        assert_eq!(c0, c1);
    }

    #[test]
    fn progress_renders_partial_segments() {
        use crate::scene::l_system::{LSystemConfig, ReplacementRule};

        // Two-segment L-system: axiom "FF" with no rules produces 2 forward steps.
        let config = LSystemConfig {
            axiom: "FF".to_string(),
            rules: vec![ReplacementRule {
                from: 'X',
                to: "X".to_string(),
            }],
        };
        let mut ls = LSystem::new(config, std::f32::consts::FRAC_PI_2, WHITE);
        ls.iterations = 0.0; // no expansion, just "FF"

        // 2 segments, progress 0.25 → exact 0.5 → 0 full + 50% of first.
        ls.progress = 0.25;
        let segs = ls.get_segments();
        assert_eq!(segs.len(), 1, "should have one partial segment");
        let seg = &segs[0];
        let expected_end = seg.start.lerp(
            // Original full first segment end
            vec2(seg.start.x, seg.start.y + 1.0),
            0.5,
        );
        assert!(
            (seg.end - expected_end).length() < 1e-4,
            "partial segment end should be at 50% of full segment, got {:?}",
            seg.end
        );

        // 2 segments, progress 0.75 → exact 1.5 → 1 full + 50% of second.
        ls.progress = 0.75;
        let segs = ls.get_segments();
        assert_eq!(segs.len(), 2, "should have one full + one partial segment");
    }

    #[test]
    fn property_names_complete() {
        let ls = make_lsystem();
        let names = ls.property_names();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&"position"));
        assert!(names.contains(&"color"));
        assert!(names.contains(&"theta"));
        assert!(names.contains(&"scale"));
        assert!(names.contains(&"iterations"));
        assert!(names.contains(&"progress"));
        assert!(names.contains(&"line_width"));
    }
}
