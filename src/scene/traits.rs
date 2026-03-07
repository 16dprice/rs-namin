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
    /// Returns true if this object should be drawn in screen space (after set_default_camera).
    /// Default is false (world-space, drawn during the 3D camera pass).
    fn is_screen_space(&self) -> bool {
        false
    }
}

pub trait Animatable {
    fn get(&self, property_name: &str) -> Option<AnimValue>;
    fn set(&mut self, property_name: &str, value: AnimValue);
    fn property_names(&self) -> &[&str];
}
