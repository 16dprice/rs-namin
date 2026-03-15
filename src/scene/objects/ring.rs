use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

const SEGMENTS: usize = 64;

pub struct Ring {
    pub position: Vec3,
    pub radius: f32,
    pub thickness: f32,
    pub sweep: f32, // 0.0 to 1.0 — fraction of the full arc to draw
    pub color: Vec4,
}

impl Ring {
    const PROPERTY_NAMES: &[&str] = &["position", "radius", "thickness", "sweep", "color"];

    pub fn new(position: Vec3, radius: f32, color: Color, sweep: f32) -> Self {
        Self {
            position,
            radius,
            thickness: 0.05,
            sweep,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat ring mesh on the XY plane. The ring is a triangle strip
    /// between an inner circle (radius - thickness/2) and outer circle
    /// (radius + thickness/2), sweeping `sweep` fraction of the full arc.
    fn build_mesh(&self) -> Mesh {
        let sweep_angle = self.sweep.clamp(0.0, 1.0) * TAU;
        let color: [u8; 4] =
            Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);

        let inner_r = (self.radius - self.thickness * 0.5).max(0.0);
        let outer_r = self.radius + self.thickness * 0.5;

        // Number of segments proportional to sweep
        let num_segs = ((SEGMENTS as f32 * self.sweep.clamp(0.0, 1.0)).ceil() as usize).max(1);

        let mut vertices = Vec::with_capacity((num_segs + 1) * 2);
        let mut indices = Vec::with_capacity(num_segs * 6);

        for i in 0..=num_segs {
            let angle = (i as f32 / num_segs as f32) * sweep_angle;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Inner vertex
            vertices.push(Vertex {
                position: vec3(
                    self.position.x + inner_r * cos_a,
                    self.position.y + inner_r * sin_a,
                    self.position.z,
                ),
                uv: vec2(i as f32 / num_segs as f32, 0.0),
                color,
                normal,
            });

            // Outer vertex
            vertices.push(Vertex {
                position: vec3(
                    self.position.x + outer_r * cos_a,
                    self.position.y + outer_r * sin_a,
                    self.position.z,
                ),
                uv: vec2(i as f32 / num_segs as f32, 1.0),
                color,
                normal,
            });
        }

        for i in 0..num_segs {
            let base = (i * 2) as u16;
            // Triangle 1: inner[i], outer[i], inner[i+1]
            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            // Triangle 2: inner[i+1], outer[i], outer[i+1]
            indices.push(base + 2);
            indices.push(base + 1);
            indices.push(base + 3);
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Ring {
    fn draw(&self) {
        if self.sweep > 0.0 {
            draw_mesh(&self.build_mesh());
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        let outer_r = self.radius + self.thickness * 0.5;
        BoundingBox {
            min: vec3(
                self.position.x - outer_r,
                self.position.y - outer_r,
                self.position.z,
            ),
            max: vec3(
                self.position.x + outer_r,
                self.position.y + outer_r,
                self.position.z,
            ),
        }
    }
}

impl Animatable for Ring {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "radius" => Some(AnimValue::Float(self.radius)),
            "thickness" => Some(AnimValue::Float(self.thickness)),
            "sweep" => Some(AnimValue::Float(self.sweep)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("radius", AnimValue::Float(v)) => self.radius = v,
            ("thickness", AnimValue::Float(v)) => self.thickness = v,
            ("sweep", AnimValue::Float(v)) => self.sweep = v,
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

    fn make_circle() -> Ring {
        Ring::new(Vec3::ZERO, 1.0, WHITE, 1.0)
    }

    #[test]
    fn property_round_trip() {
        let mut c = make_circle();
        for name in Ring::PROPERTY_NAMES {
            let val = c.get(name).unwrap();
            c.set(name, val.clone());
            assert_eq!(c.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn default_sweep_is_full() {
        let c = make_circle();
        assert_eq!(c.sweep, 1.0);
    }

    #[test]
    fn default_thickness() {
        let c = make_circle();
        assert_eq!(c.thickness, 0.05);
    }

    #[test]
    fn mesh_full_sweep() {
        let c = make_circle();
        let mesh = c.build_mesh();
        // 64 segments → 65 pairs of inner/outer = 130 vertices
        assert_eq!(mesh.vertices.len(), (SEGMENTS + 1) * 2);
        assert_eq!(mesh.indices.len(), SEGMENTS * 6);
    }

    #[test]
    fn mesh_half_sweep() {
        let mut c = make_circle();
        c.sweep = 0.5;
        let mesh = c.build_mesh();
        let expected_segs = (SEGMENTS as f32 * 0.5).ceil() as usize;
        assert_eq!(mesh.vertices.len(), (expected_segs + 1) * 2);
        assert_eq!(mesh.indices.len(), expected_segs * 6);
    }

    #[test]
    fn zero_sweep_skips_draw() {
        let mut c = make_circle();
        c.sweep = 0.0;
        // draw() skips when sweep is 0 — no mesh is rendered
        assert_eq!(c.sweep, 0.0);
    }

    #[test]
    fn bounding_box_uses_outer_radius() {
        let mut c = make_circle();
        c.thickness = 0.2;
        let bb = c.bounding_box();
        let outer = 1.0 + 0.1; // radius + thickness/2
        assert_eq!(bb.min, vec3(-outer, -outer, 0.0));
        assert_eq!(bb.max, vec3(outer, outer, 0.0));
    }

    #[test]
    fn inner_radius_clamped_to_zero() {
        let mut c = make_circle();
        c.radius = 0.01;
        c.thickness = 1.0; // inner would be -0.49, clamped to 0
        let mesh = c.build_mesh();
        // First vertex (inner) should be at position (clamped to 0 radius)
        let inner_pos = mesh.vertices[0].position;
        assert_eq!(inner_pos, Vec3::ZERO);
    }

    #[test]
    fn unknown_property_returns_none() {
        let c = make_circle();
        assert!(c.get("nonexistent").is_none());
    }

    #[test]
    fn set_sweep() {
        let mut c = make_circle();
        c.set("sweep", AnimValue::Float(0.75));
        assert_eq!(c.sweep, 0.75);
    }
}
