use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

/// A circular arc (partial ring) on the XY plane.
/// Defined by center, inner/outer radius, start angle, and sweep angle.
pub struct Arc {
    pub position: Vec3,
    pub inner_radius: f32,
    pub outer_radius: f32,
    /// Start angle in radians (0 = +X direction).
    pub start_angle: f32,
    /// Sweep angle in radians (positive = counter-clockwise).
    pub sweep_angle: f32,
    pub color: Vec4,
}

const ARC_SEGMENTS: usize = 64;

impl Arc {
    const PROPERTY_NAMES: &[&str] = &["position", "inner_radius", "outer_radius", "start_angle", "sweep_angle", "color"];

    pub fn new(position: Vec3, inner_radius: f32, outer_radius: f32, start_angle: f32, sweep_angle: f32, color: Color) -> Self {
        Self {
            position,
            inner_radius,
            outer_radius,
            start_angle,
            sweep_angle,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Convenience: full-radius arc (disk sector) with inner_radius = 0.
    pub fn sector(position: Vec3, radius: f32, start_angle: f32, sweep_angle: f32, color: Color) -> Self {
        Self::new(position, 0.0, radius, start_angle, sweep_angle, color)
    }

    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);

        // Each segment has 4 vertices (inner/outer at two angles) and 2 triangles.
        let seg_count = ARC_SEGMENTS;
        let mut vertices = Vec::with_capacity(seg_count * 2 + 2);
        let mut indices = Vec::with_capacity(seg_count * 6);

        for i in 0..=seg_count {
            let t = i as f32 / seg_count as f32;
            let angle = self.start_angle + t * self.sweep_angle;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Outer vertex
            vertices.push(Vertex {
                position: vec3(
                    self.position.x + self.outer_radius * cos_a,
                    self.position.y + self.outer_radius * sin_a,
                    self.position.z,
                ),
                uv: vec2(t, 0.0),
                color,
                normal,
            });

            // Inner vertex
            vertices.push(Vertex {
                position: vec3(
                    self.position.x + self.inner_radius * cos_a,
                    self.position.y + self.inner_radius * sin_a,
                    self.position.z,
                ),
                uv: vec2(t, 1.0),
                color,
                normal,
            });
        }

        // Two triangles per segment: (outer_i, inner_i, outer_i+1) and (inner_i, inner_i+1, outer_i+1)
        for i in 0..seg_count {
            let base = (i * 2) as u16;
            indices.push(base); // outer i
            indices.push(base + 1); // inner i
            indices.push(base + 2); // outer i+1
            indices.push(base + 1); // inner i
            indices.push(base + 3); // inner i+1
            indices.push(base + 2); // outer i+1
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Arc {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        // Conservative: use outer_radius in all directions
        BoundingBox {
            min: vec3(
                self.position.x - self.outer_radius,
                self.position.y - self.outer_radius,
                self.position.z,
            ),
            max: vec3(
                self.position.x + self.outer_radius,
                self.position.y + self.outer_radius,
                self.position.z,
            ),
        }
    }
}

impl Animatable for Arc {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "inner_radius" => Some(AnimValue::Float(self.inner_radius)),
            "outer_radius" => Some(AnimValue::Float(self.outer_radius)),
            "start_angle" => Some(AnimValue::Float(self.start_angle)),
            "sweep_angle" => Some(AnimValue::Float(self.sweep_angle)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("inner_radius", AnimValue::Float(v)) => self.inner_radius = v,
            ("outer_radius", AnimValue::Float(v)) => self.outer_radius = v,
            ("start_angle", AnimValue::Float(v)) => self.start_angle = v,
            ("sweep_angle", AnimValue::Float(v)) => self.sweep_angle = v,
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

    fn make_arc() -> Arc {
        Arc::new(vec3(1.0, 2.0, 0.0), 0.5, 1.0, 0.0, std::f32::consts::PI, WHITE)
    }

    #[test]
    fn property_round_trip() {
        let mut arc = make_arc();
        for name in Arc::PROPERTY_NAMES {
            let val = arc.get(name).unwrap();
            arc.set(name, val.clone());
            assert_eq!(arc.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn set_sweep_angle() {
        let mut arc = make_arc();
        arc.set("sweep_angle", AnimValue::Float(1.5));
        assert_eq!(arc.sweep_angle, 1.5);
    }

    #[test]
    fn set_inner_radius() {
        let mut arc = make_arc();
        arc.set("inner_radius", AnimValue::Float(0.25));
        assert_eq!(arc.inner_radius, 0.25);
    }

    #[test]
    fn unknown_property_returns_none() {
        let arc = make_arc();
        assert!(arc.get("nonexistent").is_none());
    }

    #[test]
    fn bounding_box_uses_outer_radius() {
        let arc = make_arc();
        let bb = arc.bounding_box();
        assert_eq!(bb.min.x, 0.0); // 1.0 - 1.0
        assert_eq!(bb.max.x, 2.0); // 1.0 + 1.0
    }

    #[test]
    fn sector_convenience_sets_inner_radius_zero() {
        let arc = Arc::sector(vec3(0.0, 0.0, 0.0), 2.0, 0.0, std::f32::consts::PI, RED);
        assert_eq!(arc.inner_radius, 0.0);
        assert_eq!(arc.outer_radius, 2.0);
    }

    #[test]
    fn mesh_vertex_count() {
        let arc = make_arc();
        let mesh = arc.build_mesh();
        // (ARC_SEGMENTS + 1) * 2 vertices
        assert_eq!(mesh.vertices.len(), (ARC_SEGMENTS + 1) * 2);
        // ARC_SEGMENTS * 6 indices
        assert_eq!(mesh.indices.len(), ARC_SEGMENTS * 6);
    }

    #[test]
    fn property_names_complete() {
        let arc = make_arc();
        let names = arc.property_names();
        assert_eq!(names.len(), 6);
        assert!(names.contains(&"position"));
        assert!(names.contains(&"inner_radius"));
        assert!(names.contains(&"outer_radius"));
        assert!(names.contains(&"start_angle"));
        assert!(names.contains(&"sweep_angle"));
        assert!(names.contains(&"color"));
    }
}
