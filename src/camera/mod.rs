pub mod orbit;

use macroquad::prelude::*;

use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMode {
    Perspective,
    Orthographic,
}

#[derive(Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32, // degrees
    pub near: f32,
    pub far: f32,
    pub projection: ProjectionMode,
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            projection: ProjectionMode::Perspective,
        }
    }

    pub fn to_macroquad(&self) -> Camera3D {
        Camera3D {
            position: self.position,
            target: self.target,
            up: self.up,
            fovy: self.fov.to_radians(),
            aspect: None,
            projection: match self.projection {
                ProjectionMode::Perspective => Projection::Perspective,
                ProjectionMode::Orthographic => Projection::Orthographics,
            },
            render_target: None,
            viewport: None,
            z_near: self.near,
            z_far: self.far,
        }
    }

    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    pub fn distance(&self) -> f32 {
        (self.target - self.position).length()
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(vec3(0.0, 5.0, 10.0), Vec3::ZERO)
    }
}

impl Animatable for Camera {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "target" => Some(AnimValue::Vec3(self.target)),
            "up" => Some(AnimValue::Vec3(self.up)),
            "fov" => Some(AnimValue::Float(self.fov)),
            "near" => Some(AnimValue::Float(self.near)),
            "far" => Some(AnimValue::Float(self.far)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("target", AnimValue::Vec3(v)) => self.target = v,
            ("up", AnimValue::Vec3(v)) => self.up = v,
            ("fov", AnimValue::Float(v)) => self.fov = v,
            ("near", AnimValue::Float(v)) => self.near = v,
            ("far", AnimValue::Float(v)) => self.far = v,
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        &["position", "target", "up", "fov", "near", "far"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera() {
        let cam = Camera::default();
        assert_eq!(cam.projection, ProjectionMode::Perspective);
        assert!((cam.fov - 60.0).abs() < f32::EPSILON);
        assert_eq!(cam.up, Vec3::Y);
    }

    #[test]
    fn to_macroquad_converts_fov_to_radians() {
        let cam = Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO);
        let mq = cam.to_macroquad();
        let expected_fovy = 60.0_f32.to_radians();
        assert!((mq.fovy - expected_fovy).abs() < 1e-5);
    }

    #[test]
    fn to_macroquad_perspective() {
        let cam = Camera {
            projection: ProjectionMode::Perspective,
            ..Camera::default()
        };
        let mq = cam.to_macroquad();
        assert!(matches!(mq.projection, Projection::Perspective));
    }

    #[test]
    fn to_macroquad_orthographic() {
        let cam = Camera {
            projection: ProjectionMode::Orthographic,
            ..Camera::default()
        };
        let mq = cam.to_macroquad();
        assert!(matches!(mq.projection, Projection::Orthographics));
    }

    #[test]
    fn forward_vector() {
        let cam = Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO);
        let fwd = cam.forward();
        assert!((fwd - vec3(0.0, 0.0, -1.0)).length() < 1e-5);
    }

    #[test]
    fn distance() {
        let cam = Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO);
        assert!((cam.distance() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn property_roundtrip_position() {
        let mut cam = Camera::default();
        let new_pos = AnimValue::Vec3(vec3(1.0, 2.0, 3.0));
        cam.set("position", new_pos.clone());
        assert_eq!(cam.get("position"), Some(new_pos));
    }

    #[test]
    fn property_roundtrip_target() {
        let mut cam = Camera::default();
        let new_target = AnimValue::Vec3(vec3(10.0, 0.0, 0.0));
        cam.set("target", new_target.clone());
        assert_eq!(cam.get("target"), Some(new_target));
    }

    #[test]
    fn property_roundtrip_fov() {
        let mut cam = Camera::default();
        cam.set("fov", AnimValue::Float(90.0));
        assert_eq!(cam.get("fov"), Some(AnimValue::Float(90.0)));
    }

    #[test]
    fn property_names_all_gettable() {
        let cam = Camera::default();
        for name in cam.property_names() {
            assert!(cam.get(name).is_some(), "property '{}' returned None", name);
        }
    }

    #[test]
    fn unknown_property_returns_none() {
        let cam = Camera::default();
        assert_eq!(cam.get("nonexistent"), None);
    }
}
