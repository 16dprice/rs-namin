use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const DOT_SEGMENTS: usize = 8;

// macroquad's default draw call buffer is 10,000 vertices / 5,000 indices.
// Each dot uses DOT_SEGMENTS+1 vertices and DOT_SEGMENTS*3 indices.
// Chunk size is the min of both limits.
const MAX_DOTS_PER_MESH: usize = {
    let by_verts = 10_000 / (DOT_SEGMENTS + 1);
    let by_indices = 5_000 / (DOT_SEGMENTS * 3);
    if by_verts < by_indices { by_verts } else { by_indices }
};

pub struct Spiral {
    pub position: Vec3,
    pub delta_radius: f32,
    pub delta_theta: f32,
    pub color: Vec4,
    pub num_points: usize,
    pub dot_radius: f32,
}

impl Spiral {
    pub fn new(position: Vec3, delta_radius: f32, delta_theta: f32, color: Color, num_points: usize, dot_radius: f32) -> Self {
        Self {
            position,
            delta_radius,
            delta_theta,
            color: vec4(color.r, color.g, color.b, color.a),
            num_points,
            dot_radius,
        }
    }

    fn build_dot(&self, i: usize, vertices: &mut Vec<Vertex>, indices: &mut Vec<u16>) {
        let color_bytes: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);

        let r = self.delta_radius * i as f32;
        let theta = self.delta_theta * i as f32;
        let cx = self.position.x + r * theta.cos();
        let cy = self.position.y + r * theta.sin();
        let cz = self.position.z;

        let base = vertices.len() as u16;

        // Center vertex
        vertices.push(Vertex {
            position: vec3(cx, cy, cz),
            uv: vec2(0.5, 0.5),
            color: color_bytes,
            normal,
        });

        // Edge vertices
        for j in 0..DOT_SEGMENTS {
            let angle = (j as f32 / DOT_SEGMENTS as f32) * std::f32::consts::TAU;
            vertices.push(Vertex {
                position: vec3(cx + self.dot_radius * angle.cos(), cy + self.dot_radius * angle.sin(), cz),
                uv: vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                color: color_bytes,
                normal,
            });
        }

        // Triangle fan indices
        for j in 0..DOT_SEGMENTS {
            let next = (j + 1) % DOT_SEGMENTS;
            indices.push(base);
            indices.push(base + j as u16 + 1);
            indices.push(base + next as u16 + 1);
        }
    }
}

impl SceneObject for Spiral {
    fn draw(&self) {
        for chunk_start in (0..self.num_points).step_by(MAX_DOTS_PER_MESH) {
            let chunk_end = (chunk_start + MAX_DOTS_PER_MESH).min(self.num_points);
            let chunk_len = chunk_end - chunk_start;
            let verts_per_dot = DOT_SEGMENTS + 1;

            let mut vertices = Vec::with_capacity(chunk_len * verts_per_dot);
            let mut indices = Vec::with_capacity(chunk_len * DOT_SEGMENTS * 3);

            for i in chunk_start..chunk_end {
                self.build_dot(i, &mut vertices, &mut indices);
            }

            draw_mesh(&Mesh {
                vertices,
                indices,
                texture: None,
            });
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        let max_r = self.delta_radius * (self.num_points.saturating_sub(1)) as f32 + self.dot_radius;
        BoundingBox {
            min: vec3(self.position.x - max_r, self.position.y - max_r, self.position.z),
            max: vec3(self.position.x + max_r, self.position.y + max_r, self.position.z),
        }
    }
}

animatable!(Spiral {
    position: Vec3,
    delta_radius: Float,
    delta_theta: Float,
    color: Vec4,
    dot_radius: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Spiral::new(Vec3::ZERO, 0.1, 0.5, WHITE, 100, 0.05));
    }
}
