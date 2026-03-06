use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Circle {
    pub position: Vec3,
    pub radius: f32,
    pub color: Vec4,
}

impl Circle {
    const PROPERTY_NAMES: &[&str] = &["position", "radius", "color"];

    pub fn new(position: Vec3, radius: f32, color: Color) -> Self {
        Self {
            position,
            radius,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }
}

impl SceneObject for Circle {
    fn draw(&self) {
        let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
        draw_circle(self.position.x, self.position.y, self.radius, color);
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

impl Animatable for Circle {
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
