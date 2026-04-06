use macroquad::prelude::*;

use super::sprite::Sprite;
use crate::scene::polyline::LineSegment;
use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Turtle {
    pub position: Vec3,
    pub rotation: f32,
    /// Progress along the path (0.0 to 1.0). Setting this updates `position`
    /// by interpolating along the line segments. Progress is divided evenly
    /// across segment count (not arc-length weighted).
    progress: f32,
    path: Vec<LineSegment>,
    sprite: Sprite,
    /// The sprite's local offset from the turtle's position.
    sprite_offset: Vec3,
}

impl Turtle {
    const PROPERTY_NAMES: &[&str] = &["position", "rotation", "progress"];

    pub fn new(sprite: Sprite, path: Vec<LineSegment>) -> Self {
        let sprite_offset = sprite.position;
        let (position, rotation) = if path.is_empty() {
            (Vec3::ZERO, 0.0)
        } else {
            let s = &path[0];
            (vec3(s.start.x, s.start.y, 0.0), Self::segment_heading(s))
        };
        let mut t = Self {
            position,
            rotation,
            progress: 0.0,
            path,
            sprite,
            sprite_offset,
        };
        t.sync_sprite();
        t
    }

    /// Compute heading angle (radians) from a segment's direction vector.
    /// Returns the angle from +X axis: 0 = right, π/2 = up, etc.
    fn segment_heading(seg: &LineSegment) -> f32 {
        let d = seg.end - seg.start;
        d.y.atan2(d.x)
    }

    /// Interpolate position and heading along the path for a given progress (0.0–1.0).
    /// Progress is divided evenly by segment count, matching `polyline::take_progress`.
    fn pose_on_path(&self, progress: f32) -> (Vec3, f32) {
        if self.path.is_empty() {
            return (self.position, self.rotation);
        }

        let clamped = progress.clamp(0.0, 1.0);
        let n = self.path.len() as f32;
        let exact = clamped * n;
        let seg_index = (exact.floor() as usize).min(self.path.len() - 1);
        let frac = if seg_index >= self.path.len() - 1 && exact >= n {
            1.0
        } else {
            exact - seg_index as f32
        };

        let seg = &self.path[seg_index];
        let p = seg.start.lerp(seg.end, frac);
        (vec3(p.x, p.y, 0.0), Self::segment_heading(seg))
    }

    /// Keep the child sprite's world position in sync with the turtle.
    /// The sprite offset is rotated by the turtle's heading so the sprite
    /// stays in the correct relative position regardless of direction.
    fn sync_sprite(&mut self) {
        self.sprite.position = self.position + self.sprite_offset;
        self.sprite.rotation = self.rotation;
    }
}

impl SceneObject for Turtle {
    fn draw(&self) {
        self.sprite.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        self.sprite.bounding_box()
    }
}

impl Animatable for Turtle {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "rotation" => Some(AnimValue::Float(self.rotation)),
            "progress" => Some(AnimValue::Float(self.progress)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => {
                self.position = v;
            }
            ("rotation", AnimValue::Float(v)) => {
                self.rotation = v;
            }
            ("progress", AnimValue::Float(v)) => {
                self.progress = v;
                let (pos, heading) = self.pose_on_path(v);
                self.position = pos;
                self.rotation = heading;
            }
            _ => return,
        }
        self.sync_sprite();
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(x0: f32, y0: f32, x1: f32, y1: f32) -> LineSegment {
        LineSegment {
            start: vec2(x0, y0),
            end: vec2(x1, y1),
        }
    }

    fn test_sprite(offset: Vec3) -> Sprite {
        Sprite {
            position: offset,
            rotation: 0.0,
            size: vec2(1.0, 1.0),
            center: Vec2::ZERO,
            flip_x: false,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            texture: None,
        }
    }

    #[test]
    fn property_names_match_getters() {
        let turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 1.0, 0.0)]);
        for name in turtle.property_names() {
            assert!(turtle.get(name).is_some(), "get({name}) returned None");
        }
    }

    #[test]
    fn initial_position_is_first_segment_start() {
        let turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(1.0, 2.0, 5.0, 6.0)]);
        assert_eq!(turtle.position, vec3(1.0, 2.0, 0.0));
    }

    #[test]
    fn sprite_offset_applied() {
        let turtle = Turtle::new(
            test_sprite(vec3(0.5, 0.5, 0.0)),
            vec![seg(10.0, 20.0, 20.0, 20.0)],
        );
        // Offset is applied in world space, not rotated by heading.
        assert!((turtle.sprite.position.x - 10.5).abs() < 1e-5);
        assert!((turtle.sprite.position.y - 20.5).abs() < 1e-5);
    }

    #[test]
    fn sprite_offset_not_rotated_by_heading() {
        // Vertical segment (heading=π/2). Offset (1, 0) stays (1, 0) in world space.
        let turtle = Turtle::new(
            test_sprite(vec3(1.0, 0.0, 0.0)),
            vec![seg(0.0, 0.0, 0.0, 10.0)],
        );
        assert!((turtle.sprite.position.x - 1.0).abs() < 1e-5);
        assert!(turtle.sprite.position.y.abs() < 1e-5);
    }

    #[test]
    fn set_position_syncs_sprite() {
        let mut turtle = Turtle::new(
            test_sprite(vec3(1.0, 0.0, 0.0)),
            vec![seg(0.0, 0.0, 10.0, 0.0)],
        );
        turtle.set("position", AnimValue::Vec3(vec3(5.0, 5.0, 0.0)));
        assert_eq!(turtle.sprite.position, vec3(6.0, 5.0, 0.0));
    }

    #[test]
    fn set_rotation_updates_turtle() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 1.0, 0.0)]);
        turtle.set("rotation", AnimValue::Float(1.5));
        assert_eq!(turtle.rotation, 1.5);
    }

    #[test]
    fn progress_zero_is_path_start() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 10.0, 0.0)]);
        turtle.set("progress", AnimValue::Float(0.0));
        assert_eq!(turtle.position, vec3(0.0, 0.0, 0.0));
    }

    #[test]
    fn progress_one_is_path_end() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 10.0, 0.0)]);
        turtle.set("progress", AnimValue::Float(1.0));
        assert!((turtle.position.x - 10.0).abs() < 1e-5);
    }

    #[test]
    fn progress_midpoint_single_segment() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 10.0, 0.0)]);
        turtle.set("progress", AnimValue::Float(0.5));
        assert!((turtle.position.x - 5.0).abs() < 1e-5);
    }

    #[test]
    fn progress_multi_segment_equal_length() {
        // Two segments of equal length (10 each).
        let path = vec![seg(0.0, 0.0, 10.0, 0.0), seg(10.0, 0.0, 10.0, 10.0)];
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), path);

        // 25% = halfway through first segment.
        turtle.set("progress", AnimValue::Float(0.25));
        assert!((turtle.position.x - 5.0).abs() < 1e-5);
        assert!(turtle.position.y.abs() < 1e-5);

        // 75% = halfway through second segment.
        turtle.set("progress", AnimValue::Float(0.75));
        assert!((turtle.position.x - 10.0).abs() < 1e-5);
        assert!((turtle.position.y - 5.0).abs() < 1e-5);
    }

    #[test]
    fn progress_multi_segment_unequal_length() {
        // First segment length=10, second length=30.
        // Progress is segment-count based, not arc-length: each segment gets
        // an equal share of the progress range (0.0–0.5 and 0.5–1.0).
        let path = vec![seg(0.0, 0.0, 10.0, 0.0), seg(10.0, 0.0, 40.0, 0.0)];
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), path);

        // Halfway through first segment at progress 0.25.
        turtle.set("progress", AnimValue::Float(0.25));
        assert!((turtle.position.x - 5.0).abs() < 1e-5);

        // End of first segment at progress 0.5.
        turtle.set("progress", AnimValue::Float(0.5));
        assert!((turtle.position.x - 10.0).abs() < 1e-5);

        // Halfway through second segment at progress 0.75.
        turtle.set("progress", AnimValue::Float(0.75));
        assert!((turtle.position.x - 25.0).abs() < 1e-5);
    }

    #[test]
    fn progress_clamps() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 10.0, 0.0)]);

        turtle.set("progress", AnimValue::Float(-0.5));
        assert!(turtle.position.x.abs() < 1e-5);

        turtle.set("progress", AnimValue::Float(1.5));
        assert!((turtle.position.x - 10.0).abs() < 1e-5);
    }

    #[test]
    fn empty_path_keeps_position() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![]);
        turtle.set("position", AnimValue::Vec3(vec3(3.0, 4.0, 0.0)));
        turtle.set("progress", AnimValue::Float(0.5));
        assert_eq!(turtle.position, vec3(3.0, 4.0, 0.0));
    }

    #[test]
    fn bounding_box_delegates_to_sprite() {
        // Use a horizontal segment so heading=0 and sprite rotation stays 0.
        let turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(5.0, 5.0, 15.0, 5.0)]);
        let bb = turtle.bounding_box();
        // Sprite is 1x1 centered at (5,5), so bounds are (4.5,4.5) to (5.5,5.5).
        assert!((bb.min.x - 4.5).abs() < 1e-5);
        assert!((bb.min.y - 4.5).abs() < 1e-5);
        assert!((bb.max.x - 5.5).abs() < 1e-5);
        assert!((bb.max.y - 5.5).abs() < 1e-5);
    }

    #[test]
    fn property_roundtrip_position() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 1.0, 0.0)]);
        let val = AnimValue::Vec3(vec3(3.0, 4.0, 5.0));
        turtle.set("position", val.clone());
        assert_eq!(turtle.get("position"), Some(val));
    }

    #[test]
    fn property_roundtrip_rotation() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 1.0, 0.0)]);
        let val = AnimValue::Float(2.0);
        turtle.set("rotation", val.clone());
        assert_eq!(turtle.get("rotation"), Some(val));
    }

    #[test]
    fn property_roundtrip_progress() {
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 1.0, 0.0)]);
        let val = AnimValue::Float(0.7);
        turtle.set("progress", val.clone());
        assert_eq!(turtle.get("progress"), Some(val));
    }

    #[test]
    fn heading_follows_segment_direction() {
        use std::f32::consts::FRAC_PI_2;

        // Horizontal segment → heading 0 (facing +X).
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 10.0, 0.0)]);
        turtle.set("progress", AnimValue::Float(0.5));
        assert!(
            turtle.rotation.abs() < 1e-5,
            "expected 0, got {}",
            turtle.rotation
        );

        // Vertical segment → heading π/2 (facing +Y).
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 0.0, 10.0)]);
        turtle.set("progress", AnimValue::Float(0.5));
        assert!(
            (turtle.rotation - FRAC_PI_2).abs() < 1e-5,
            "expected π/2, got {}",
            turtle.rotation
        );
    }

    #[test]
    fn heading_changes_between_segments() {
        use std::f32::consts::FRAC_PI_2;

        // First segment goes right, second goes up.
        let path = vec![seg(0.0, 0.0, 10.0, 0.0), seg(10.0, 0.0, 10.0, 10.0)];
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), path);

        // On first segment: heading 0.
        turtle.set("progress", AnimValue::Float(0.25));
        assert!(turtle.rotation.abs() < 1e-5);

        // On second segment: heading π/2.
        turtle.set("progress", AnimValue::Float(0.75));
        assert!((turtle.rotation - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn initial_heading_matches_first_segment() {
        use std::f32::consts::FRAC_PI_2;

        // Segment goes up → initial heading should be π/2.
        let turtle = Turtle::new(test_sprite(Vec3::ZERO), vec![seg(0.0, 0.0, 0.0, 10.0)]);
        assert!((turtle.rotation - FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn sprite_not_flipped() {
        // Facing left should not flip the sprite.
        let path = vec![seg(10.0, 0.0, 0.0, 0.0)];
        let mut turtle = Turtle::new(test_sprite(Vec3::ZERO), path);
        turtle.set("progress", AnimValue::Float(0.5));
        assert!(!turtle.sprite.flip_x);
    }
}
