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

    /// The combined view-projection matrix, matching what macroquad renders
    /// with (orthographic reinterprets `fovy` as vertical world-unit extent).
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let mq = self.to_macroquad();
        let view = Mat4::look_at_rh(mq.position, mq.target, mq.up);
        let proj = match self.projection {
            ProjectionMode::Perspective => Mat4::perspective_rh_gl(mq.fovy, aspect, mq.z_near, mq.z_far),
            ProjectionMode::Orthographic => {
                let half_h = mq.fovy / 2.0;
                let half_w = half_h * aspect;
                Mat4::orthographic_rh_gl(-half_w, half_w, -half_h, half_h, mq.z_near, mq.z_far)
            }
        };
        proj * view
    }

    /// A world-space ray (origin, normalized direction) through a screen
    /// pixel. `screen` is in window pixels with the origin top-left.
    pub fn screen_ray(&self, screen: Vec2, screen_size: Vec2) -> (Vec3, Vec3) {
        let ndc_x = (screen.x / screen_size.x) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen.y / screen_size.y) * 2.0; // flip Y
        let inv_vp = self.view_projection(screen_size.x / screen_size.y).inverse();
        let near = inv_vp.project_point3(vec3(ndc_x, ndc_y, -1.0));
        let far = inv_vp.project_point3(vec3(ndc_x, ndc_y, 1.0));
        (near, (far - near).normalize())
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
    fn screen_ray_through_center_points_forward() {
        let cam = Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO);
        let (origin, dir) = cam.screen_ray(vec2(640.0, 360.0), vec2(1280.0, 720.0));
        assert!((dir - vec3(0.0, 0.0, -1.0)).length() < 1e-4);
        assert!(origin.z < 10.0 && origin.z > 9.0); // starts at the near plane
    }

    #[test]
    fn screen_ray_center_orthographic() {
        let mut cam = Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO);
        cam.projection = ProjectionMode::Orthographic;
        let (_, dir) = cam.screen_ray(vec2(640.0, 360.0), vec2(1280.0, 720.0));
        assert!((dir - vec3(0.0, 0.0, -1.0)).length() < 1e-4);
    }

    #[test]
    fn property_round_trip() {
        use crate::scene::traits::test_support::assert_property_roundtrip;
        assert_property_roundtrip(&mut Camera::default());
    }
}
