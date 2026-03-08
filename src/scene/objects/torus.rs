use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

const MAJOR_SEGMENTS: usize = 32;
const MINOR_SEGMENTS: usize = 16;

pub struct Torus {
    pub position: Vec3,
    pub major_radius: f32,
    pub minor_radius: f32,
    pub rotation: Mat4,
    pub color: Vec4,
}

impl Torus {
    const PROPERTY_NAMES: &[&str] = &["position", "major_radius", "minor_radius", "rotation", "color"];

    pub fn new(position: Vec3, major_radius: f32, minor_radius: f32, color: Color) -> Self {
        Self {
            position,
            major_radius,
            minor_radius,
            rotation: Mat4::IDENTITY,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] =
            Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();

        let vert_count = (MAJOR_SEGMENTS + 1) * (MINOR_SEGMENTS + 1);
        let idx_count = MAJOR_SEGMENTS * MINOR_SEGMENTS * 6;
        let mut vertices = Vec::with_capacity(vert_count);
        let mut indices = Vec::with_capacity(idx_count);

        for i in 0..=MAJOR_SEGMENTS {
            let theta = (i as f32 / MAJOR_SEGMENTS as f32) * TAU;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            for j in 0..=MINOR_SEGMENTS {
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
                let local_normal = vec3(
                    cos_phi * cos_theta,
                    cos_phi * sin_theta,
                    sin_phi,
                );

                let rotated_pos = self.rotation.transform_point3(local_pos) + self.position;
                let rotated_normal = self.rotation.transform_vector3(local_normal).normalize();

                vertices.push(Vertex {
                    position: rotated_pos,
                    uv: vec2(i as f32 / MAJOR_SEGMENTS as f32, j as f32 / MINOR_SEGMENTS as f32),
                    color,
                    normal: vec4(rotated_normal.x, rotated_normal.y, rotated_normal.z, 0.0),
                });
            }
        }

        // Generate indices: two triangles per quad
        for i in 0..MAJOR_SEGMENTS {
            for j in 0..MINOR_SEGMENTS {
                let row_len = (MINOR_SEGMENTS + 1) as u16;
                let cur = (i as u16) * row_len + (j as u16);
                let next_row = cur + row_len;

                indices.push(cur);
                indices.push(next_row);
                indices.push(cur + 1);

                indices.push(cur + 1);
                indices.push(next_row);
                indices.push(next_row + 1);
            }
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Torus {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        let r = self.major_radius + self.minor_radius;
        BoundingBox {
            min: self.position - Vec3::splat(r),
            max: self.position + Vec3::splat(r),
        }
    }
}

impl Animatable for Torus {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "major_radius" => Some(AnimValue::Float(self.major_radius)),
            "minor_radius" => Some(AnimValue::Float(self.minor_radius)),
            "rotation" => Some(AnimValue::Mat4(self.rotation)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("major_radius", AnimValue::Float(v)) => self.major_radius = v,
            ("minor_radius", AnimValue::Float(v)) => self.minor_radius = v,
            ("rotation", AnimValue::Mat4(v)) => self.rotation = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
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

    fn make_torus() -> Torus {
        Torus::new(vec3(1.0, 2.0, 0.0), 2.0, 0.5, WHITE)
    }

    #[test]
    fn property_round_trip() {
        let mut torus = make_torus();
        for name in Torus::PROPERTY_NAMES {
            let val = torus.get(name).unwrap();
            torus.set(name, val.clone());
            assert_eq!(torus.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn set_major_radius() {
        let mut torus = make_torus();
        torus.set("major_radius", AnimValue::Float(5.0));
        assert_eq!(torus.major_radius, 5.0);
    }

    #[test]
    fn set_minor_radius() {
        let mut torus = make_torus();
        torus.set("minor_radius", AnimValue::Float(1.0));
        assert_eq!(torus.minor_radius, 1.0);
    }

    #[test]
    fn unknown_property_returns_none() {
        let torus = make_torus();
        assert!(torus.get("nonexistent").is_none());
    }

    #[test]
    fn default_rotation_is_identity() {
        let torus = make_torus();
        assert_eq!(torus.rotation, Mat4::IDENTITY);
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
        let mesh = torus.build_mesh();
        assert_eq!(mesh.vertices.len(), (MAJOR_SEGMENTS + 1) * (MINOR_SEGMENTS + 1));
    }

    #[test]
    fn mesh_index_count() {
        let torus = make_torus();
        let mesh = torus.build_mesh();
        assert_eq!(mesh.indices.len(), MAJOR_SEGMENTS * MINOR_SEGMENTS * 6);
    }

    #[test]
    fn rotation_transforms_vertices() {
        let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE);
        // With identity rotation, vertices should have Z components from the tube
        let mesh_identity = torus.build_mesh();

        // Rotate 90 degrees around X axis
        torus.rotation = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let mesh_rotated = torus.build_mesh();

        // After 90-degree X rotation, what was Z should now be -Y (approximately)
        // Just verify the meshes are different
        let identity_positions: Vec<Vec3> = mesh_identity.vertices.iter().map(|v| v.position).collect();
        let rotated_positions: Vec<Vec3> = mesh_rotated.vertices.iter().map(|v| v.position).collect();

        let mut any_different = false;
        for (a, b) in identity_positions.iter().zip(rotated_positions.iter()) {
            if (*a - *b).length() > 1e-5 {
                any_different = true;
                break;
            }
        }
        assert!(any_different, "rotation should change vertex positions");
    }

    #[test]
    fn property_names_complete() {
        let torus = make_torus();
        let names = torus.property_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"position"));
        assert!(names.contains(&"major_radius"));
        assert!(names.contains(&"minor_radius"));
        assert!(names.contains(&"rotation"));
        assert!(names.contains(&"color"));
    }
}
