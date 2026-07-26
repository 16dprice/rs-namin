use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const DOT_SEGMENTS: usize = 8;

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

    /// Build every dot as a closed fan; `MeshBuilder` chunks automatically.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);
        for i in 0..self.num_points {
            let r = self.delta_radius * i as f32;
            let theta = self.delta_theta * i as f32;
            let cx = self.position.x + r * theta.cos();
            let cy = self.position.y + r * theta.sin();
            let cz = self.position.z;

            let center = flat_vertex(vec3(cx, cy, cz), vec2(0.5, 0.5), color);
            let rim: Vec<_> = (0..DOT_SEGMENTS)
                .map(|j| {
                    let angle = (j as f32 / DOT_SEGMENTS as f32) * TAU;
                    flat_vertex(
                        vec3(cx + self.dot_radius * angle.cos(), cy + self.dot_radius * angle.sin(), cz),
                        vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                        color,
                    )
                })
                .collect();
            mb.fan(center, &rim, true);
        }
    }
}

impl SceneObject for Spiral {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
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
