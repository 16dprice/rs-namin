use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Sprite {
    pub position: Vec3,
    /// Rotation around the Z axis in radians.
    pub rotation: f32,
    /// Width (X) and height (Y) in world units.
    pub size: Vec2,
    /// Pivot point in local space (world units, relative to geometric center).
    /// The sprite rotates around this point, and this point is placed at `position`.
    /// Default `(0,0)` = geometric center.
    pub center: Vec2,
    /// Mirror the texture horizontally.
    pub flip_x: bool,
    /// Tint color — white (1,1,1,1) renders the texture unmodified.
    pub color: Vec4,
    pub(crate) texture: Option<Texture2D>,
    /// Image file resolved lazily at draw time through the texture cache.
    /// Scene-document sprites use this: constructing one must not touch the
    /// GL context or fail (specs spawn per-frame in the inspector and in
    /// headless tests).
    pub(crate) image_path: Option<String>,
}

impl Sprite {
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
            center: Vec2::ZERO,
            flip_x: false,
            color: vec4(color.r, color.g, color.b, color.a),
            texture: Some(texture),
            image_path: None,
        }
    }

    /// Create a sprite whose image loads lazily at first draw (see
    /// `scene::texture_cache`) — a missing or unreadable file draws a
    /// placeholder. `size` must be explicit: geometry can't depend on a
    /// texture that may never load.
    pub fn from_path(path: &str, position: Vec3, size: Vec2, color: Color) -> Self {
        Self {
            position,
            rotation: 0.0,
            size,
            center: Vec2::ZERO,
            flip_x: false,
            color: vec4(color.r, color.g, color.b, color.a),
            texture: None,
            image_path: Some(path.to_string()),
        }
    }

    /// Build a textured quad on the XY plane, rotated around `center`.
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;

        // Corner offsets relative to geometric center, shifted so that
        // `self.center` becomes the rotation pivot (maps to `position`).
        let c = self.center;
        let corners = [
            vec2(-hw - c.x, -hh - c.y),
            vec2(hw - c.x, -hh - c.y),
            vec2(hw - c.x, hh - c.y),
            vec2(-hw - c.x, hh - c.y),
        ];

        let (sin_r, cos_r) = self.rotation.sin_cos();
        let cx = self.position.x;
        let cy = self.position.y;
        let z = self.position.z;

        let (u0, u1) = if self.flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
        let uvs = [vec2(u0, 1.0), vec2(u1, 1.0), vec2(u1, 0.0), vec2(u0, 0.0)];

        let quad = std::array::from_fn(|i| {
            let c = corners[i];
            let rx = c.x * cos_r - c.y * sin_r;
            let ry = c.x * sin_r + c.y * cos_r;
            flat_vertex(vec3(cx + rx, cy + ry, z), uvs[i], color)
        });
        mb.quad(quad);
    }
}

impl SceneObject for Sprite {
    fn draw(&self) {
        let texture = self
            .texture
            .clone()
            .or_else(|| self.image_path.as_deref().map(crate::scene::texture_cache::get));
        let mut mb = match texture {
            Some(texture) => MeshBuilder::with_texture(texture),
            None => MeshBuilder::new(),
        };
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        let hw = self.size.x / 2.0;
        let hh = self.size.y / 2.0;
        let c = self.center;

        let corners = [
            vec2(-hw - c.x, -hh - c.y),
            vec2(hw - c.x, -hh - c.y),
            vec2(hw - c.x, hh - c.y),
            vec2(-hw - c.x, hh - c.y),
        ];

        let (sin_r, cos_r) = self.rotation.sin_cos();
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for p in &corners {
            let rx = p.x * cos_r - p.y * sin_r + self.position.x;
            let ry = p.x * sin_r + p.y * cos_r + self.position.y;
            min_x = min_x.min(rx);
            min_y = min_y.min(ry);
            max_x = max_x.max(rx);
            max_y = max_y.max(ry);
        }

        BoundingBox {
            min: vec3(min_x, min_y, self.position.z),
            max: vec3(max_x, max_y, self.position.z),
        }
    }
}

animatable!(Sprite {
    position: Vec3,
    rotation: Float,
    size: Vec2,
    color: Vec4,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    /// Build a Sprite without a GPU context for property/bounding-box tests.
    fn test_sprite(position: Vec3, size: Vec2) -> Sprite {
        Sprite {
            position,
            rotation: 0.0,
            size,
            center: Vec2::ZERO,
            flip_x: false,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            texture: None,
            image_path: None,
        }
    }

    #[test]
    fn from_path_is_gl_free_and_sized_explicitly() {
        let sprite = Sprite::from_path("assets/nope.png", vec3(1.0, 2.0, 0.0), vec2(3.0, 1.0), WHITE);
        assert_eq!(sprite.image_path.as_deref(), Some("assets/nope.png"));
        assert!(sprite.texture.is_none());
        // Geometry comes from the explicit size, never the (unloaded) image.
        let bb = sprite.bounding_box();
        assert!((bb.max.x - 2.5).abs() < 1e-5 && (bb.max.y - 2.5).abs() < 1e-5);
    }

    fn build_mesh(sprite: &Sprite) -> Mesh {
        let mut mb = MeshBuilder::new();
        sprite.build(&mut mb);
        let mut meshes = mb.build();
        assert_eq!(meshes.len(), 1);
        meshes.remove(0)
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut test_sprite(Vec3::ZERO, vec2(2.0, 1.0)));
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
    fn center_shifts_mesh_vertices() {
        // With center=(0, 0.5) on a 1x1 sprite at origin, the pivot is 0.5
        // above geometric center. The quad should shift down so that point
        // lands at position. Without rotation, bottom-left should be at (-0.5, -1.0).
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        sprite.center = vec2(0.0, 0.5);
        let mesh = build_mesh(&sprite);
        let bl = mesh.vertices[0].position; // bottom-left corner
        assert!((bl.x - -0.5).abs() < 1e-5);
        assert!((bl.y - -1.0).abs() < 1e-5);
    }

    #[test]
    fn center_pivot_stays_at_position_after_rotation() {
        use std::f32::consts::FRAC_PI_2;

        // center=(0, 0.5) means the point 0.5 above geometric center is at position.
        // After 90° rotation, that point should still be at position (0,0).
        // The geometric center of the quad should be at (-0.5, 0) after rotation.
        let mut sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        sprite.center = vec2(0.0, 0.5);
        sprite.rotation = FRAC_PI_2;
        let mesh = build_mesh(&sprite);

        // Average of all 4 vertices = geometric center of the rotated quad.
        let avg_x: f32 = mesh.vertices.iter().map(|v| v.position.x).sum::<f32>() / 4.0;
        let avg_y: f32 = mesh.vertices.iter().map(|v| v.position.y).sum::<f32>() / 4.0;
        // Geometric center should have shifted away from position.
        assert!(avg_x.abs() > 0.1 || avg_y.abs() > 0.1);

        // But position (0,0) should be the pivot: the center point in local space
        // was (0, 0.5), which after 90° CCW rotation becomes (-0.5, 0).
        // Since we subtracted center before rotation, the pivot maps to origin.
        // Verify by checking that the bounding box is centered around the
        // geometric center, not the position.
        let bb = sprite.bounding_box();
        let bb_cx = (bb.min.x + bb.max.x) / 2.0;
        let bb_cy = (bb.min.y + bb.max.y) / 2.0;
        // BB center should NOT be at position (0,0) because of the offset.
        assert!(bb_cx.abs() > 0.1 || bb_cy.abs() > 0.1);
    }

    #[test]
    fn mesh_no_texture_still_builds() {
        let sprite = test_sprite(Vec3::ZERO, vec2(1.0, 1.0));
        let mesh = build_mesh(&sprite);
        assert!(mesh.texture.is_none());
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
    }
}
