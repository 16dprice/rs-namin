use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const DISK_SEGMENTS: usize = 32;

pub struct Disk {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec4,
}

impl Disk {
    pub fn new(position: Vec3, radius: f32, color: Color) -> Self {
        Self {
            position,
            radius,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat disk on the XY plane centered at `position`.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);
        let center = flat_vertex(self.position, vec2(0.5, 0.5), color);
        let rim: Vec<_> = (0..DISK_SEGMENTS)
            .map(|i| {
                let angle = (i as f32 / DISK_SEGMENTS as f32) * TAU;
                let x = self.position.x + self.radius * angle.cos();
                let y = self.position.y + self.radius * angle.sin();
                flat_vertex(
                    vec3(x, y, self.position.z),
                    vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                    color,
                )
            })
            .collect();
        mb.fan(center, &rim, true);
    }
}

impl SceneObject for Disk {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: vec3(self.position.x - self.radius, self.position.y - self.radius, self.position.z),
            max: vec3(self.position.x + self.radius, self.position.y + self.radius, self.position.z),
        }
    }
}

animatable!(Disk {
    position: Vec3,
    radius: Float,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Disk::new(Vec3::ZERO, 1.0, WHITE));
    }
}
