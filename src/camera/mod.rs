pub mod orbit;

use macroquad::prelude::*;

use crate::scene::traits::animatable;

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
    pub rotation_x: f32, // radians — pitch around X axis
    pub rotation_y: f32, // radians — yaw around Y axis
    pub rotation_z: f32, // radians — roll around Z axis
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
            rotation_x: 0.0,
            rotation_y: 0.0,
            rotation_z: 0.0,
        }
    }

    /// Apply rotation fields to the base position offset from target.
    fn rotated_position(&self) -> Vec3 {
        let offset = self.position - self.target;
        let rot = Quat::from_euler(EulerRot::YXZ, self.rotation_y, self.rotation_x, self.rotation_z);
        self.target + rot.mul_vec3(offset)
    }

    pub fn to_macroquad(&self) -> Camera3D {
        let position = self.rotated_position();
        Camera3D {
            position,
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
        (self.target - self.rotated_position()).normalize()
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

animatable!(Camera {
    position: Vec3,
    target: Vec3,
    up: Vec3,
    fov: Float,
    near: Float,
    far: Float,
    rotation_x: Float,
    rotation_y: Float,
    rotation_z: Float,
});

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
    fn property_round_trip() {
        use crate::scene::traits::test_support::assert_property_roundtrip;
        assert_property_roundtrip(&mut Camera::default());
    }
}
