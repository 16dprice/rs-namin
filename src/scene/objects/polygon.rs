use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Polygon {
    pub position: Vec3,
    pub radius: f32,
    /// Number of sides (3 = triangle, 5 = pentagon, 6 = hexagon, etc.).
    pub sides: u32,
    /// Rotation in radians. 0 = first vertex points along +X.
    pub rotation: f32,
    pub color: Vec4,
}

impl Polygon {
    const PROPERTY_NAMES: &[&str] = &["position", "radius", "sides", "rotation", "color"];

    pub fn new(position: Vec3, radius: f32, sides: u32, color: Color) -> Self {
        Self {
            position,
            radius,
            sides: sides.max(3),
            rotation: 0.0,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat regular polygon mesh on the XY plane centered at `position`.
    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] =
            Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let n = self.sides as usize;
        let mut vertices = Vec::with_capacity(n + 1);
        let mut indices = Vec::with_capacity(n * 3);

        // Center vertex
        vertices.push(Vertex {
            position: self.position,
            uv: vec2(0.5, 0.5),
            color,
            normal,
        });

        // Edge vertices
        for i in 0..n {
            let angle = self.rotation + (i as f32 / n as f32) * TAU;
            let x = self.position.x + self.radius * angle.cos();
            let y = self.position.y + self.radius * angle.sin();
            vertices.push(Vertex {
                position: vec3(x, y, self.position.z),
                uv: vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                color,
                normal,
            });
        }

        // Triangle fan
        for i in 0..n {
            let next = (i + 1) % n;
            indices.push(0);
            indices.push((i + 1) as u16);
            indices.push((next + 1) as u16);
        }

        Mesh {
            vertices,
            indices,
            texture: None,
        }
    }
}

impl SceneObject for Polygon {
    fn draw(&self) {
        draw_mesh(&self.build_mesh());
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: vec3(
                self.position.x - self.radius,
                self.position.y - self.radius,
                self.position.z,
            ),
            max: vec3(
                self.position.x + self.radius,
                self.position.y + self.radius,
                self.position.z,
            ),
        }
    }
}

impl Animatable for Polygon {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "radius" => Some(AnimValue::Float(self.radius)),
            "sides" => Some(AnimValue::Float(self.sides as f32)),
            "rotation" => Some(AnimValue::Float(self.rotation)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("radius", AnimValue::Float(v)) => self.radius = v,
            ("sides", AnimValue::Float(v)) => self.sides = (v as u32).max(3),
            ("rotation", AnimValue::Float(v)) => self.rotation = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}
