use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Rectangle {
    pub position: Vec3,
    /// Width (X) and height (Y) of the rectangle.
    pub size: Vec2,
    pub color: Vec4,
}

impl Rectangle {
    pub fn new(position: Vec3, size: Vec2, color: Color) -> Self {
        Self {
            position,
            size,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat quad mesh on the XY plane centered at `position`.
    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;
        let z = self.position.z;
        let cx = self.position.x;
        let cy = self.position.y;

        let vertices = vec![
            Vertex {
                position: vec3(cx - hw, cy - hh, z),
                uv: vec2(0.0, 0.0),
                color,
                normal,
            },
            Vertex {
                position: vec3(cx + hw, cy - hh, z),
                uv: vec2(1.0, 0.0),
                color,
                normal,
            },
            Vertex {
                position: vec3(cx + hw, cy + hh, z),
                uv: vec2(1.0, 1.0),
                color,
                normal,
            },
            Vertex {
                position: vec3(cx - hw, cy + hh, z),
                uv: vec2(0.0, 1.0),
                color,
                normal,
            },
        ];

        // Two triangles: 0-1-2 and 0-2-3
        let indices = vec![0, 1, 2, 0, 2, 3];

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Rectangle {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;
        BoundingBox {
            min: vec3(self.position.x - hw, self.position.y - hh, self.position.z),
            max: vec3(self.position.x + hw, self.position.y + hh, self.position.z),
        }
    }
}

animatable!(Rectangle {
    position: Vec3,
    size: Vec2,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Rectangle::new(Vec3::ZERO, vec2(2.0, 1.0), WHITE));
    }
}
