use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

const DISK_SEGMENTS: usize = 32;

pub struct Disk {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec4,
}

impl Disk {
    const PROPERTY_NAMES: &[&str] = &["position", "radius", "color"];

    pub fn new(position: Vec3, radius: f32, color: Color) -> Self {
        Self {
            position,
            radius,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }

    /// Build a flat disk mesh on the XY plane centered at `position`.
    fn build_mesh(&self) -> Mesh {
        let color: [u8; 4] =
            Color::new(self.color.x, self.color.y, self.color.z, self.color.w).into();
        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let mut vertices = Vec::with_capacity(DISK_SEGMENTS + 1);
        let mut indices = Vec::with_capacity(DISK_SEGMENTS * 3);

        // Center vertex
        vertices.push(Vertex {
            position: self.position,
            uv: vec2(0.5, 0.5),
            color,
            normal,
        });

        // Edge vertices on the XY plane (normal along +Z)
        for i in 0..DISK_SEGMENTS {
            let angle = (i as f32 / DISK_SEGMENTS as f32) * TAU;
            let x = self.position.x + self.radius * angle.cos();
            let y = self.position.y + self.radius * angle.sin();
            vertices.push(Vertex {
                position: vec3(x, y, self.position.z),
                uv: vec2(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()),
                color,
                normal,
            });
        }

        // Triangle fan: center → edge[i] → edge[i+1]
        for i in 0..DISK_SEGMENTS {
            let next = (i + 1) % DISK_SEGMENTS;
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

impl SceneObject for Disk {
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

impl Animatable for Disk {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "radius" => Some(AnimValue::Float(self.radius)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("radius", AnimValue::Float(v)) => self.radius = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}
