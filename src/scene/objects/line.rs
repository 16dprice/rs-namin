use macroquad::prelude::*;

use crate::scene::mesh::{MeshBuilder, color_bytes, flat_vertex};
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Vec4,
    /// 0-1 reveal fraction from `start` toward `end`.
    pub progress: f32,
    /// World-space width. 0 (the default) draws a hairline via
    /// `draw_line_3d` — the historical look, so existing scenes are
    /// unchanged; above 0 draws a ribbon quad on the XY plane (endpoint z
    /// interpolates across it).
    pub thickness: f32,
}

impl Line {
    pub fn new(start: Vec3, end: Vec3, color: Color) -> Self {
        Self {
            start,
            end,
            color: vec4(color.r, color.g, color.b, color.a),
            progress: 1.0,
            thickness: 0.0,
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

    /// Half-thickness offset perpendicular to the line in the XY plane.
    fn half_perp(&self) -> Vec2 {
        let d = self.end - self.start;
        let dxy = vec2(d.x, d.y);
        let len = dxy.length();
        let half = self.thickness / 2.0;
        if len > 1e-8 {
            vec2(-dxy.y, dxy.x) / len * half
        } else {
            vec2(half, 0.0)
        }
    }

    /// Build the thick-line ribbon quad (thickness > 0 only).
    fn build(&self, mb: &mut MeshBuilder) {
        let color = color_bytes(self.color);
        let end = self.visible_end();
        let p = self.half_perp();
        mb.quad([
            flat_vertex(vec3(self.start.x - p.x, self.start.y - p.y, self.start.z), vec2(0.0, 0.0), color),
            flat_vertex(vec3(self.start.x + p.x, self.start.y + p.y, self.start.z), vec2(0.0, 1.0), color),
            flat_vertex(vec3(end.x + p.x, end.y + p.y, end.z), vec2(1.0, 1.0), color),
            flat_vertex(vec3(end.x - p.x, end.y - p.y, end.z), vec2(1.0, 0.0), color),
        ]);
    }
}

impl SceneObject for Line {
    fn draw(&self) {
        if self.progress <= 0.0 {
            return;
        }
        if self.thickness <= 0.0 {
            let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
            draw_line_3d(self.start, self.visible_end(), color);
            return;
        }
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
    }

    fn bounding_box(&self) -> BoundingBox {
        let end = self.visible_end();
        let p = self.half_perp();
        let (px, py) = (p.x.abs(), p.y.abs());
        BoundingBox {
            min: vec3(self.start.x.min(end.x) - px, self.start.y.min(end.y) - py, self.start.z.min(end.z)),
            max: vec3(self.start.x.max(end.x) + px, self.start.y.max(end.y) + py, self.start.z.max(end.z)),
        }
    }
}

animatable!(Line {
    start: Vec3,
    end: Vec3,
    color: Vec4,
    progress: Float,
    thickness: Float,
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

    #[test]
    fn thickness_builds_a_ribbon_quad() {
        let mut line = Line::new(Vec3::ZERO, vec3(4.0, 0.0, 1.0), WHITE);
        line.thickness = 0.5;
        let mut mb = MeshBuilder::new();
        line.build(&mut mb);
        let meshes = mb.build();
        assert_eq!(meshes.len(), 1);
        let verts = &meshes[0].vertices;
        assert_eq!(verts.len(), 4);
        // Horizontal line: perp is +/-Y by half thickness.
        assert!((verts[0].position.y + 0.25).abs() < 1e-5);
        assert!((verts[1].position.y - 0.25).abs() < 1e-5);
        // Endpoint z carries across the ribbon.
        assert!((verts[2].position.z - 1.0).abs() < 1e-5);
        assert!((verts[0].position.z - 0.0).abs() < 1e-5);
    }

    #[test]
    fn thickness_expands_the_bounding_box() {
        let mut line = Line::new(Vec3::ZERO, vec3(10.0, 0.0, 0.0), WHITE);
        line.thickness = 1.0;
        let bb = line.bounding_box();
        assert!((bb.min.y + 0.5).abs() < 1e-5);
        assert!((bb.max.y - 0.5).abs() < 1e-5);
    }
}
