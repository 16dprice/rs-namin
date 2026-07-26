use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

/// A circular arc (partial ring) on the XY plane.
/// Defined by center, inner/outer radius, start angle, and sweep angle.
pub struct Arc {
    pub position: Vec3,
    pub inner_radius: f32,
    pub outer_radius: f32,
    /// Start angle in radians (0 = +X direction).
    pub start_angle: f32,
    /// Sweep angle in radians (positive = counter-clockwise).
    pub sweep_angle: f32,
    pub color: Vec4,
}

const ARC_SEGMENTS: usize = 64;

impl Arc {
    pub fn new(position: Vec3, inner_radius: f32, outer_radius: f32, start_angle: f32, sweep_angle: f32, color: Color) -> Self {
        Self {
            position,
            inner_radius,
            outer_radius,
            start_angle,
            sweep_angle,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Convenience: full-radius arc (disk sector) with inner_radius = 0.
    pub fn sector(position: Vec3, radius: f32, start_angle: f32, sweep_angle: f32, color: Color) -> Self {
        Self::new(position, 0.0, radius, start_angle, sweep_angle, color)
    }

    /// Build the arc as a quad strip between the inner and outer radius rows.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);

        let mut outer_row = Vec::with_capacity(ARC_SEGMENTS + 1);
        let mut inner_row = Vec::with_capacity(ARC_SEGMENTS + 1);

        for i in 0..=ARC_SEGMENTS {
            let t = i as f32 / ARC_SEGMENTS as f32;
            let angle = self.start_angle + t * self.sweep_angle;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            outer_row.push(flat_vertex(
                vec3(
                    self.position.x + self.outer_radius * cos_a,
                    self.position.y + self.outer_radius * sin_a,
                    self.position.z,
                ),
                vec2(t, 0.0),
                color,
            ));

            inner_row.push(flat_vertex(
                vec3(
                    self.position.x + self.inner_radius * cos_a,
                    self.position.y + self.inner_radius * sin_a,
                    self.position.z,
                ),
                vec2(t, 1.0),
                color,
            ));
        }

        mb.strip(&inner_row, &outer_row);
    }
}

impl SceneObject for Arc {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        // Conservative: use outer_radius in all directions
        BoundingBox {
            min: vec3(
                self.position.x - self.outer_radius,
                self.position.y - self.outer_radius,
                self.position.z,
            ),
            max: vec3(
                self.position.x + self.outer_radius,
                self.position.y + self.outer_radius,
                self.position.z,
            ),
        }
    }
}

animatable!(Arc {
    position: Vec3,
    inner_radius: Float,
    outer_radius: Float,
    start_angle: Float,
    sweep_angle: Float,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_arc() -> Arc {
        Arc::new(vec3(1.0, 2.0, 0.0), 0.5, 1.0, 0.0, std::f32::consts::PI, WHITE)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_arc());
    }

    #[test]
    fn bounding_box_uses_outer_radius() {
        let arc = make_arc();
        let bb = arc.bounding_box();
        assert_eq!(bb.min.x, 0.0); // 1.0 - 1.0
        assert_eq!(bb.max.x, 2.0); // 1.0 + 1.0
    }

    #[test]
    fn sector_convenience_sets_inner_radius_zero() {
        let arc = Arc::sector(vec3(0.0, 0.0, 0.0), 2.0, 0.0, std::f32::consts::PI, RED);
        assert_eq!(arc.inner_radius, 0.0);
        assert_eq!(arc.outer_radius, 2.0);
    }

    #[test]
    fn mesh_vertex_count() {
        let arc = make_arc();
        let mut mb = MeshBuilder::new();
        arc.build(&mut mb);
        let meshes = mb.build();
        // ARC_SEGMENTS quads of 4 vertices / 6 indices each
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), ARC_SEGMENTS * 4);
        assert_eq!(meshes[0].indices.len(), ARC_SEGMENTS * 6);
    }
}
