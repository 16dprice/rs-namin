use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const MAJOR_SEGMENTS: usize = 32;
const MINOR_SEGMENTS: usize = 16;

pub struct Torus {
    pub position: Vec3,
    pub major_radius: f32,
    pub minor_radius: f32,
    /// Full 3D orientation applied to the mesh. A `Mat4`, unlike the planar
    /// Z-axis `rotation` (Float) property on 2D objects.
    pub orientation: Mat4,
    pub color: Vec4,
}

impl Torus {
    pub fn new(position: Vec3, major_radius: f32, minor_radius: f32, color: Color) -> Self {
        Self {
            position,
            major_radius,
            minor_radius,
            orientation: Mat4::IDENTITY,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build the torus surface: one row of vertices per major segment,
    /// connected to the next row by quad strips.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);

        let rows: Vec<Vec<Vertex>> = (0..=MAJOR_SEGMENTS)
            .map(|i| {
                let theta = (i as f32 / MAJOR_SEGMENTS as f32) * TAU;
                let cos_theta = theta.cos();
                let sin_theta = theta.sin();

                (0..=MINOR_SEGMENTS)
                    .map(|j| {
                        let phi = (j as f32 / MINOR_SEGMENTS as f32) * TAU;
                        let cos_phi = phi.cos();
                        let sin_phi = phi.sin();

                        // Local position (torus in XY plane, tube cross-section has Z component)
                        let local_pos = vec3(
                            (self.major_radius + self.minor_radius * cos_phi) * cos_theta,
                            (self.major_radius + self.minor_radius * cos_phi) * sin_theta,
                            self.minor_radius * sin_phi,
                        );

                        // Local normal (points outward from tube surface)
                        let local_normal = vec3(cos_phi * cos_theta, cos_phi * sin_theta, sin_phi);

                        let rotated_pos = self.orientation.transform_point3(local_pos) + self.position;
                        let rotated_normal = self.orientation.transform_vector3(local_normal).normalize();

                        Vertex {
                            position: rotated_pos,
                            uv: vec2(i as f32 / MAJOR_SEGMENTS as f32, j as f32 / MINOR_SEGMENTS as f32),
                            color,
                            normal: vec4(rotated_normal.x, rotated_normal.y, rotated_normal.z, 0.0),
                        }
                    })
                    .collect()
            })
            .collect();

        for i in 0..MAJOR_SEGMENTS {
            mb.strip(&rows[i], &rows[i + 1]);
        }
    }
}

impl SceneObject for Torus {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        let r = self.major_radius + self.minor_radius;
        BoundingBox {
            min: self.position - Vec3::splat(r),
            max: self.position + Vec3::splat(r),
        }
    }
}

animatable!(Torus {
    position: Vec3,
    major_radius: Float,
    minor_radius: Float,
    orientation: Mat4,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_torus() -> Torus {
        Torus::new(vec3(1.0, 2.0, 0.0), 2.0, 0.5, WHITE)
    }

    fn build_meshes(torus: &Torus) -> Vec<Mesh> {
        let mut mb = MeshBuilder::new();
        torus.build(&mut mb);
        mb.build()
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_torus());
    }

    #[test]
    fn default_orientation_is_identity() {
        let torus = make_torus();
        assert_eq!(torus.orientation, Mat4::IDENTITY);
    }

    #[test]
    fn bounding_box_correct() {
        let torus = make_torus();
        let bb = torus.bounding_box();
        let r = 2.0 + 0.5; // major + minor
        assert_eq!(bb.min, vec3(1.0 - r, 2.0 - r, 0.0 - r));
        assert_eq!(bb.max, vec3(1.0 + r, 2.0 + r, 0.0 + r));
    }

    #[test]
    fn mesh_vertex_count() {
        let torus = make_torus();
        let total_verts: usize = build_meshes(&torus).iter().map(|m| m.vertices.len()).sum();
        // strip() duplicates vertices per quad: 4 per quad, one quad per segment pair.
        assert_eq!(total_verts, MAJOR_SEGMENTS * MINOR_SEGMENTS * 4);
    }

    #[test]
    fn mesh_index_count() {
        let torus = make_torus();
        let total_indices: usize = build_meshes(&torus).iter().map(|m| m.indices.len()).sum();
        assert_eq!(total_indices, MAJOR_SEGMENTS * MINOR_SEGMENTS * 6);
    }

    #[test]
    fn orientation_transforms_vertices() {
        let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE);
        // With identity orientation, vertices should have Z components from the tube
        let meshes_identity = build_meshes(&torus);

        // Rotate 90 degrees around X axis
        torus.orientation = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let meshes_rotated = build_meshes(&torus);

        // After 90-degree X rotation, what was Z should now be -Y (approximately)
        // Just verify the meshes are different
        let identity_positions: Vec<Vec3> = meshes_identity.iter().flat_map(|m| m.vertices.iter().map(|v| v.position)).collect();
        let rotated_positions: Vec<Vec3> = meshes_rotated.iter().flat_map(|m| m.vertices.iter().map(|v| v.position)).collect();

        let mut any_different = false;
        for (a, b) in identity_positions.iter().zip(rotated_positions.iter()) {
            if (*a - *b).length() > 1e-5 {
                any_different = true;
                break;
            }
        }
        assert!(any_different, "orientation should change vertex positions");
    }
}
