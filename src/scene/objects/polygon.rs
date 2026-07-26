use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Polygon {
    pub position: Vec3,
    pub radius: f32,
    /// Number of sides (3 = triangle, 5 = pentagon, 6 = hexagon, etc.).
    /// Stored as f32 so it can be keyframed; floored and clamped to >= 3 at draw time.
    pub sides: f32,
    /// Rotation in radians. 0 = first vertex points along +X.
    pub rotation: f32,
    pub color: Vec4,
}

impl Polygon {
    pub fn new(position: Vec3, radius: f32, sides: u32, color: Color) -> Self {
        Self {
            position,
            radius,
            sides: sides as f32,
            rotation: 0.0,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat regular polygon mesh on the XY plane centered at `position`.
    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let n = (self.sides.floor() as usize).max(3);
        let mut vertices = Vec::with_capacity(n + 1);
        let mut indices = Vec::with_capacity(n * 3);

        // Center vertex
        vertices.push(Vertex {
            position: self.position,
            uv: vec2(0.5, 0.5),
            color,
            normal,
        });

        // Edge vertices
        for i in 0..n {
            let angle = self.rotation + (i as f32 / n as f32) * TAU;
            let x = self.position.x + self.radius * angle.cos();
            let y = self.position.y + self.radius * angle.sin();
            vertices.push(Vertex {
                position: vec3(x, y, self.position.z),
                uv: vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                color,
                normal,
            });
        }

        // Triangle fan
        for i in 0..n {
            let next = (i + 1) % n;
            indices.push(0);
            indices.push((i + 1) as u16);
            indices.push((next + 1) as u16);
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Polygon {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: vec3(self.position.x - self.radius, self.position.y - self.radius, self.position.z),
            max: vec3(self.position.x + self.radius, self.position.y + self.radius, self.position.z),
        }
    }
}

animatable!(Polygon {
    position: Vec3,
    radius: Float,
    sides: Float,
    rotation: Float,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Polygon::new(Vec3::ZERO, 1.0, 6, WHITE));
    }

    #[test]
    fn mesh_clamps_sides_to_minimum_three() {
        let mut poly = Polygon::new(Vec3::ZERO, 1.0, 6, WHITE);
        poly.sides = 1.7;
        // 3 sides → center + 3 edge vertices, 3 triangles
        let mesh = poly.build_mesh();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 9);
    }
}
