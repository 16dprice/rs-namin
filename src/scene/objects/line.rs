use macroquad::prelude::*;

use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec4,
    /// 0-1 reveal fraction from `start` toward `end`.
    pub progress: f32,
}

impl Line {
    pub fn new(start: Vec3, end: Vec3, color: Color) -> Self {
        Self {
            start,
            end,
            color: vec4(color.r, color.g, color.b, color.a),
            progress: 1.0,
        }
    }

    fn visible_end(&self) -> Vec3 {
        self.start.lerp(self.end, self.progress.clamp(0.0, 1.0))
    }

    /// The revealed tip (read-only output — a binding source).
    pub fn pen_position(&self) -> Vec3 {
        self.visible_end()
    }

    /// XY heading of the line in radians (constant along it; the shared
    /// `atan2` convention of every `pen_angle` output).
    pub fn pen_angle(&self) -> f32 {
        let d = self.end - self.start;
        d.y.atan2(d.x)
    }
}

impl SceneObject for Line {
    fn draw(&self) {
        if self.progress <= 0.0 {
            return;
        }
        let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
        draw_line_3d(self.start, self.visible_end(), color);
    }

    fn bounding_box(&self) -> BoundingBox {
        let end = self.visible_end();
        BoundingBox {
            min: vec3(self.start.x.min(end.x), self.start.y.min(end.y), self.start.z.min(end.z)),
            max: vec3(self.start.x.max(end.x), self.start.y.max(end.y), self.start.z.max(end.z)),
        }
    }
}

animatable!(Line {
    start: Vec3,
    end: Vec3,
    color: Vec4,
    progress: Float,
} outputs {
    pen_position: Vec3,
    pen_angle: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut Line::new(Vec3::ZERO, vec3(1.0, 2.0, 3.0), WHITE));
    }

    #[test]
    fn pen_tracks_the_reveal() {
        let mut line = Line::new(vec3(1.0, 0.0, 0.0), vec3(5.0, 0.0, 2.0), WHITE);
        assert_eq!(line.pen_position(), vec3(5.0, 0.0, 2.0));
        line.progress = 0.5;
        assert_eq!(line.pen_position(), vec3(3.0, 0.0, 1.0));
        line.progress = 0.0;
        assert_eq!(line.pen_position(), vec3(1.0, 0.0, 0.0));
        // Heading is constant regardless of progress.
        let flat = Line::new(Vec3::ZERO, vec3(0.0, 2.0, 0.0), WHITE);
        assert!((flat.pen_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn bounding_box_follows_progress() {
        let mut line = Line::new(Vec3::ZERO, vec3(10.0, 0.0, 0.0), WHITE);
        line.progress = 0.3;
        assert!((line.bounding_box().max.x - 3.0).abs() < 1e-5);
    }
}
