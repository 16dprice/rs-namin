use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const SEGMENTS: usize = 64;

pub struct Ring {
    pub position: Vec3,
    pub radius: f32,
    pub thickness: f32,
    pub progress: f32, // 0.0 to 1.0 — fraction of the full arc to draw
    pub color: Vec4,
}

impl Ring {
    pub fn new(position: Vec3, radius: f32, color: Color, progress: f32) -> Self {
        Self {
            position,
            radius,
            thickness: 0.05,
            progress,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat ring on the XY plane: a quad strip between an inner
    /// circle (radius - thickness/2) and outer circle (radius + thickness/2),
    /// sweeping `progress` fraction of the full arc.
    fn build(&self, mb: &mut MeshBuilder) {
        let sweep_angle = self.progress.clamp(0.0, 1.0) * TAU;
        let color = color_bytes(self.color);

        let inner_r = (self.radius - self.thickness * 0.5).max(0.0);
        let outer_r = self.radius + self.thickness * 0.5;

        // Number of segments proportional to progress
        let num_segs = ((SEGMENTS as f32 * self.progress.clamp(0.0, 1.0)).ceil() as usize).max(1);

        let mut inner_row = Vec::with_capacity(num_segs + 1);
        let mut outer_row = Vec::with_capacity(num_segs + 1);

        for i in 0..=num_segs {
            let t = i as f32 / num_segs as f32;
            let angle = t * sweep_angle;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            inner_row.push(flat_vertex(
                vec3(
                    self.position.x + inner_r * cos_a,
                    self.position.y + inner_r * sin_a,
                    self.position.z,
                ),
                vec2(t, 0.0),
                color,
            ));

            outer_row.push(flat_vertex(
                vec3(
                    self.position.x + outer_r * cos_a,
                    self.position.y + outer_r * sin_a,
                    self.position.z,
                ),
                vec2(t, 1.0),
                color,
            ));
        }

        mb.strip(&inner_row, &outer_row);
    }
}

impl SceneObject for Ring {
    fn draw(&self) {
        if self.progress > 0.0 {
            let mut mb = MeshBuilder::new();
            self.build(&mut mb);
            mb.draw();
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        let outer_r = self.radius + self.thickness * 0.5;
        BoundingBox {
            min: vec3(self.position.x - outer_r, self.position.y - outer_r, self.position.z),
            max: vec3(self.position.x + outer_r, self.position.y + outer_r, self.position.z),
        }
    }
}

animatable!(Ring {
    position: Vec3,
    radius: Float,
    thickness: Float,
    progress: Float,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_circle() -> Ring {
        Ring::new(Vec3::ZERO, 1.0, WHITE, 1.0)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_circle());
    }

    #[test]
    fn default_progress_is_full() {
        let c = make_circle();
        assert_eq!(c.progress, 1.0);
    }

    #[test]
    fn default_thickness() {
        let c = make_circle();
        assert_eq!(c.thickness, 0.05);
    }

    fn build_meshes(c: &Ring) -> Vec<Mesh> {
        let mut mb = MeshBuilder::new();
        c.build(&mut mb);
        mb.build()
    }

    #[test]
    fn mesh_full_sweep() {
        let c = make_circle();
        let meshes = build_meshes(&c);
        // 64 segments → 64 quads of 4 vertices / 6 indices each
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), SEGMENTS * 4);
        assert_eq!(meshes[0].indices.len(), SEGMENTS * 6);
    }

    #[test]
    fn mesh_half_sweep() {
        let mut c = make_circle();
        c.progress = 0.5;
        let meshes = build_meshes(&c);
        let expected_segs = (SEGMENTS as f32 * 0.5).ceil() as usize;
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), expected_segs * 4);
        assert_eq!(meshes[0].indices.len(), expected_segs * 6);
    }

    #[test]
    fn zero_progress_skips_draw() {
        let mut c = make_circle();
        c.progress = 0.0;
        // draw() skips when progress is 0 — no mesh is rendered
        assert_eq!(c.progress, 0.0);
    }

    #[test]
    fn bounding_box_uses_outer_radius() {
        let mut c = make_circle();
        c.thickness = 0.2;
        let bb = c.bounding_box();
        let outer = 1.0 + 0.1; // radius + thickness/2
        assert_eq!(bb.min, vec3(-outer, -outer, 0.0));
        assert_eq!(bb.max, vec3(outer, outer, 0.0));
    }

    #[test]
    fn inner_radius_clamped_to_zero() {
        let mut c = make_circle();
        c.radius = 0.01;
        c.thickness = 1.0; // inner would be -0.49, clamped to 0
        let meshes = build_meshes(&c);
        // First vertex of the first quad is inner[0] (clamped to 0 radius)
        let inner_pos = meshes[0].vertices[0].position;
        assert_eq!(inner_pos, Vec3::ZERO);
    }
}
