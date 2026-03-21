use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Sprite {
    pub position: Vec3,
    /// Rotation around the Z axis in radians.
    pub rotation: f32,
    /// Width (X) and height (Y) in world units.
    pub size: Vec2,
    /// Tint color — white (1,1,1,1) renders the texture unmodified.
    pub color: Vec4,
    pub(crate) texture: Option<Texture2D>,
}

impl Sprite {
    const PROPERTY_NAMES: &[&str] = &["position", "rotation", "size", "color"];

    /// Create a sprite from a pre-loaded texture.
    ///
    /// `size` sets the dimensions in world units. Pass `None` to derive size
    /// from the texture's pixel dimensions (1 pixel = 1 world unit).
    pub fn new(texture: Texture2D, position: Vec3, size: Option<Vec2>, color: Color) -> Self {
        let size = size.unwrap_or_else(|| vec2(texture.width(), texture.height()));
        Self {
            position,
            rotation: 0.0,
            size,
            color: vec4(color.r, color.g, color.b, color.a),
            texture: Some(texture),
        }
    }

    /// Build a textured quad mesh on the XY plane, rotated around its center.
    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] =
            Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;

        // Corner offsets relative to center.
        let corners = [vec2(-hw, -hh), vec2(hw, -hh), vec2(hw, hh), vec2(-hw, hh)];

        let (sin_r, cos_r) = self.rotation.sin_cos();
        let cx = self.position.x;
        let cy = self.position.y;
        let z = self.position.z;

        let uvs = [
            vec2(0.0, 1.0),
            vec2(1.0, 1.0),
            vec2(1.0, 0.0),
            vec2(0.0, 0.0),
        ];

        let vertices: Vec<Vertex> = corners
            .iter()
            .zip(uvs.iter())
            .map(|(c, &uv)| {
                let rx = c.x * cos_r - c.y * sin_r;
                let ry = c.x * sin_r + c.y * cos_r;
                Vertex {
                    position: vec3(cx + rx, cy + ry, z),
                    uv,
                    color,
                    normal,
                }
            })
            .collect();

        Mesh {
            vertices,
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: self.texture.clone(),
        }
    }
}

impl SceneObject for Sprite {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;

        // For a rotated rectangle, the AABB encloses all four rotated corners.
        let (sin_r, cos_r) = self.rotation.sin_cos();
        let abs_cos = cos_r.abs();
        let abs_sin = sin_r.abs();
        let half_w = hw * abs_cos + hh * abs_sin;
        let half_h = hw * abs_sin + hh * abs_cos;

        BoundingBox {
            min: vec3(
                self.position.x - half_w,
                self.position.y - half_h,
                self.position.z,
            ),
            max: vec3(
                self.position.x + half_w,
                self.position.y + half_h,
                self.position.z,
            ),
        }
    }
}

impl Animatable for Sprite {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "rotation" => Some(AnimValue::Float(self.rotation)),
            "size" => Some(AnimValue::Vec2(self.size)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("rotation", AnimValue::Float(v)) => self.rotation = v,
            ("size", AnimValue::Vec2(v)) => self.size = v,
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
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    /// Build a Sprite without a GPU context for property/bounding-box tests.
    fn test_sprite(position: Vec3, size: Vec2) -> Sprite {
        Sprite {
            position,
            rotation: 0.0,
            size,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            texture: None,
        }
    }

    #[test]
    fn property_names_match_getters() {
        let sprite = test_sprite(Vec3::ZERO, vec2(2.0, 1.0));
        for name in sprite.property_names() {
            assert!(sprite.get(name).is_some(), "get({name}) returned None");
        }
    }

    #[test]
    fn property_roundtrip_position() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let val = AnimValue::Vec3(vec3(3.0, 4.0, 5.0));
        sprite.set("position", val.clone());
        assert_eq!(sprite.get("position"), Some(val));
    }

    #[test]
    fn property_roundtrip_rotation() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let val = AnimValue::Float(1.5);
        sprite.set("rotation", val.clone());
        assert_eq!(sprite.get("rotation"), Some(val));
    }

    #[test]
    fn property_roundtrip_size() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let val = AnimValue::Vec2(vec2(10.0, 20.0));
        sprite.set("size", val.clone());
        assert_eq!(sprite.get("size"), Some(val));
    }

    #[test]
    fn property_roundtrip_color() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let val = AnimValue::Vec4(vec4(0.5, 0.6, 0.7, 0.8));
        sprite.set("color", val.clone());
        assert_eq!(sprite.get("color"), Some(val));
    }

    #[test]
    fn bounding_box_no_rotation() {
        let sprite = test_sprite(vec3(1.0, 2.0, 0.0), vec2(4.0, 2.0));
        let bb = sprite.bounding_box();
        assert!((bb.min.x - -1.0).abs() < 1e-5);
        assert!((bb.min.y - 1.0).abs() < 1e-5);
        assert!((bb.max.x - 3.0).abs() < 1e-5);
        assert!((bb.max.y - 3.0).abs() < 1e-5);
    }

    #[test]
    fn bounding_box_grows_with_rotation() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(4.0, 2.0));
        let bb_0 = sprite.bounding_box();
        sprite.rotation = FRAC_PI_4;
        let bb_45 = sprite.bounding_box();
        assert!(bb_45.max.x > bb_0.max.x);
        assert!(bb_45.max.y > bb_0.max.y);
    }

    #[test]
    fn bounding_box_90_degree_rotation_swaps_axes() {
        let mut sprite = test_sprite(Vec3::ZERO, vec2(4.0, 2.0));
        let bb_0 = sprite.bounding_box();
        sprite.rotation = FRAC_PI_2;
        let bb_90 = sprite.bounding_box();
        assert!((bb_90.max.x - bb_0.max.y).abs() < 1e-5);
        assert!((bb_90.max.y - bb_0.max.x).abs() < 1e-5);
    }

    #[test]
    fn mesh_no_texture_still_builds() {
        let sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let mesh = sprite.build_mesh();
        assert!(mesh.texture.is_none());
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
    }
}
