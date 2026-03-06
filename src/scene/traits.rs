use macroquad::prelude::Vec3;

use super::value::AnimValue;

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

pub trait SceneObject {
    fn draw(&self);
    fn bounding_box(&self) -> BoundingBox;
}

pub trait Animatable {
    fn get(&self, property_name: &str) -> Option<AnimValue>;
    fn set(&mut self, property_name: &str, value: AnimValue);
    fn property_names(&self) -> &[&str];
}
