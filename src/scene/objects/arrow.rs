use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

/// An arrow on the XY plane: a rectangular shaft with a triangular head.
pub struct Arrow {
    pub start: Vec3,
    pub end: Vec3,
    /// Width of the shaft.
    pub shaft_width: f32,
    /// Width of the arrowhead (perpendicular to direction).
    pub head_width: f32,
    /// Length of the arrowhead along the arrow direction.
    pub head_length: f32,
    pub color: Vec4,
}

impl Arrow {
    pub fn new(start: Vec3, end: Vec3, color: Color) -> Self {
        let length = (end - start).length();
        Self {
            start,
            end,
            shaft_width: length * 0.04,
            head_width: length * 0.12,
            head_length: length * 0.15,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    pub fn with_dimensions(start: Vec3, end: Vec3, shaft_width: f32, head_width: f32, head_length: f32, color: Color) -> Self {
        Self {
            start,
            end,
            shaft_width,
            head_width,
            head_length,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build the shaft quad and head triangle. Appends nothing for a
    /// zero-length arrow.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);

        let dir = self.end - self.start;
        let length = dir.length();
        if length < 1e-6 {
            return;
        }
        let fwd = dir / length;
        // Perpendicular in XY plane
        let perp = vec3(-fwd.y, fwd.x, 0.0);

        let shaft_half = self.shaft_width / 2.0;
        let head_half = self.head_width / 2.0;
        let shaft_end_len = (length - self.head_length).max(0.0);
        let shaft_end = self.start + fwd * shaft_end_len;
        let z = self.start.z;

        // Shaft: 4 vertices, 2 triangles
        // v0 = start - perp * shaft_half
        // v1 = start + perp * shaft_half
        // v2 = shaft_end + perp * shaft_half
        // v3 = shaft_end - perp * shaft_half
        let v0 = self.start - perp * shaft_half;
        let v1 = self.start + perp * shaft_half;
        let v2 = shaft_end + perp * shaft_half;
        let v3 = shaft_end - perp * shaft_half;

        // Head: 3 vertices (triangle)
        // v4 = shaft_end - perp * head_half
        // v5 = shaft_end + perp * head_half
        // v6 = end (tip)
        let v4 = shaft_end - perp * head_half;
        let v5 = shaft_end + perp * head_half;
        let v6 = self.end;

        let mk = |pos: Vec3| flat_vertex(vec3(pos.x, pos.y, z), vec2(0.0, 0.0), color);

        mb.quad([mk(v0), mk(v1), mk(v2), mk(v3)]);
        mb.primitive(&[mk(v4), mk(v5), mk(v6)], &[0, 1, 2]);
    }
}

impl SceneObject for Arrow {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        let half = self.head_width / 2.0;
        let dir = self.end - self.start;
        let length = dir.length();
        let perp = if length > 1e-6 {
            let fwd = dir / length;
            vec3(-fwd.y, fwd.x, 0.0) * half
        } else {
            vec3(half, half, 0.0)
        };

        let xs = [
            self.start.x - perp.x.abs(),
            self.start.x + perp.x.abs(),
            self.end.x - perp.x.abs(),
            self.end.x + perp.x.abs(),
        ];
        let ys = [
            self.start.y - perp.y.abs(),
            self.start.y + perp.y.abs(),
            self.end.y - perp.y.abs(),
            self.end.y + perp.y.abs(),
        ];

        BoundingBox {
            min: vec3(
                xs.iter().copied().reduce(f32::min).unwrap(),
                ys.iter().copied().reduce(f32::min).unwrap(),
                self.start.z.min(self.end.z),
            ),
            max: vec3(
                xs.iter().copied().reduce(f32::max).unwrap(),
                ys.iter().copied().reduce(f32::max).unwrap(),
                self.start.z.max(self.end.z),
            ),
        }
    }
}

animatable!(Arrow {
    start: Vec3,
    end: Vec3,
    shaft_width: Float,
    head_width: Float,
    head_length: Float,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_arrow() -> Arrow {
        Arrow::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), WHITE)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_arrow());
    }

    #[test]
    fn default_proportions() {
        let arrow = make_arrow();
        // length = 10, shaft_width = 10 * 0.04 = 0.4
        assert!((arrow.shaft_width - 0.4).abs() < 1e-5);
        assert!((arrow.head_width - 1.2).abs() < 1e-5);
        assert!((arrow.head_length - 1.5).abs() < 1e-5);
    }

    fn build_meshes(arrow: &Arrow) -> Vec<Mesh> {
        let mut mb = MeshBuilder::new();
        arrow.build(&mut mb);
        mb.build()
    }

    #[test]
    fn mesh_has_correct_topology() {
        let arrow = make_arrow();
        let meshes = build_meshes(&arrow);
        assert_eq!(meshes.len(), 1);
        // 7 vertices: 4 shaft + 3 head
        assert_eq!(meshes[0].vertices.len(), 7);
        // 9 indices: 6 shaft + 3 head
        assert_eq!(meshes[0].indices.len(), 9);
    }

    #[test]
    fn zero_length_arrow_produces_empty_mesh() {
        let arrow = Arrow::new(vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0), WHITE);
        assert!(build_meshes(&arrow).is_empty());
    }

    #[test]
    fn bounding_box_contains_endpoints() {
        let arrow = Arrow::new(vec3(1.0, 2.0, 0.0), vec3(5.0, 8.0, 0.0), RED);
        let bb = arrow.bounding_box();
        assert!(bb.min.x <= 1.0);
        assert!(bb.min.y <= 2.0);
        assert!(bb.max.x >= 5.0);
        assert!(bb.max.y >= 8.0);
    }

    #[test]
    fn tip_vertex_at_end_position() {
        let arrow = make_arrow();
        let meshes = build_meshes(&arrow);
        let tip = meshes[0].vertices[6].position;
        assert!((tip.x - 10.0).abs() < 1e-5);
        assert!(tip.y.abs() < 1e-5);
    }

    #[test]
    fn with_dimensions_constructor() {
        let arrow = Arrow::with_dimensions(vec3(0.0, 0.0, 0.0), vec3(5.0, 0.0, 0.0), 0.5, 1.0, 2.0, BLUE);
        assert_eq!(arrow.shaft_width, 0.5);
        assert_eq!(arrow.head_width, 1.0);
        assert_eq!(arrow.head_length, 2.0);
    }
}
