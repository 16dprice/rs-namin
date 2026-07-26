use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec4,
}

impl Line {
    pub fn new(start: Vec3, end: Vec3, color: Color) -> Self {
        Self {
            start,
            end,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }
}

impl SceneObject for Line {
    fn draw(&self) {
        let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
        draw_line_3d(self.start, self.end, color);
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: vec3(
                self.start.x.min(self.end.x),
                self.start.y.min(self.end.y),
                self.start.z.min(self.end.z),
            ),
            max: vec3(
                self.start.x.max(self.end.x),
                self.start.y.max(self.end.y),
                self.start.z.max(self.end.z),
            ),
        }
    }
}

animatable!(Line {
    start: Vec3,
    end: Vec3,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Line::new(Vec3::ZERO, vec3(1.0, 2.0, 3.0), WHITE));
    }
}
