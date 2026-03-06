use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Line {
    pub start: Vec3,
    pub end: Vec3,
    pub thickness: f32,
    pub color: Vec4,
}

impl Line {
    const PROPERTY_NAMES: &[&str] = &["start", "end", "thickness", "color"];

    pub fn new(start: Vec3, end: Vec3, thickness: f32, color: Color) -> Self {
        Self {
            start,
            end,
            thickness,
            color: vec4(color.r, color.g, color.b, color.a),
        }
    }
}

impl SceneObject for Line {
    fn draw(&self) {
        let color = Color::new(self.color.x, self.color.y, self.color.z, self.color.w);
        draw_line(
            self.start.x,
            self.start.y,
            self.end.x,
            self.end.y,
            self.thickness,
            color,
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: vec3(
                self.start.x.min(self.end.x),
                self.start.y.min(self.end.y),
                self.start.z.min(self.end.z),
            ),
            max: vec3(
                self.start.x.max(self.end.x),
                self.start.y.max(self.end.y),
                self.start.z.max(self.end.z),
            ),
        }
    }
}

impl Animatable for Line {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "start" => Some(AnimValue::Vec3(self.start)),
            "end" => Some(AnimValue::Vec3(self.end)),
            "thickness" => Some(AnimValue::Float(self.thickness)),
            "color" => Some(AnimValue::Vec4(self.color)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("start", AnimValue::Vec3(v)) => self.start = v,
            ("end", AnimValue::Vec3(v)) => self.end = v,
            ("thickness", AnimValue::Float(v)) => self.thickness = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}
