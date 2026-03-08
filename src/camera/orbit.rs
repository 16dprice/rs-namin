use macroquad::prelude::{KeyCode, MouseButton, Vec3, vec2, vec3};

use super::Camera;
use crate::input::InputProvider;

pub struct OrbitController {
    pub target: Vec3,
    pub distance: f32,
    /// Horizontal angle in radians (around Y axis).
    pub azimuth: f32,
    /// Vertical angle in radians from the horizontal plane. Clamped to avoid gimbal lock.
    pub elevation: f32,
    pub orbit_speed: f32,
    pub zoom_speed: f32,
    pub move_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl OrbitController {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance,
            azimuth: 0.0,
            elevation: 0.3, // ~17 degrees above horizontal
            orbit_speed: 0.005,
            zoom_speed: 0.1,
            move_speed: 10.0,
            min_distance: 0.5,
            max_distance: 500.0,
        }
    }

    /// Derive the orbit controller state from an existing camera.
    pub fn from_camera(camera: &Camera) -> Self {
        let offset = camera.position - camera.target;
        let distance = offset.length();
        let azimuth = offset.x.atan2(offset.z);
        let elevation = (offset.y / distance).asin();

        Self {
            target: camera.target,
            distance,
            azimuth,
            elevation,
            ..Self::new(Vec3::ZERO, 1.0)
        }
    }

    /// Process mouse input and update the camera.
    pub fn update(&mut self, camera: &mut Camera, input: &dyn InputProvider) {
        let (_, scroll_y) = input.mouse_wheel();
        let raw_delta = input.mouse_delta();
        // mouse_delta_position() returns coords in -2..2 range (normalized * 2 - 1).
        // Multiply by screen/2 to get pixel deltas.
        let delta = vec2(
            raw_delta.x * input.screen_width() * 0.5,
            raw_delta.y * input.screen_height() * 0.5,
        );

        // Middle-click drag: orbit
        if input.is_mouse_button_down(MouseButton::Middle) {
            self.azimuth += delta.x * self.orbit_speed;
            self.elevation -= delta.y * self.orbit_speed;
            self.elevation = self.elevation.clamp(-1.5, 1.5); // ~86 degrees
        }

        // Right-click drag: pan (1:1 with mouse — point under cursor stays under cursor)
        if input.is_mouse_button_down(MouseButton::Right) {
            let right = self.right_vector();
            let up = camera.up;
            // Convert pixel movement to world units at the target's depth.
            // For perspective: world_per_pixel = 2 * distance * tan(fov/2) / screen_height
            let fov_rad = camera.fov.to_radians();
            let world_per_pixel =
                2.0 * self.distance * (fov_rad / 2.0).tan() / input.screen_height();
            self.target += (delta.x * right - delta.y * up) * world_per_pixel;
        }

        // Scroll: zoom
        if scroll_y.abs() > 0.0 {
            self.distance *= 1.0 - scroll_y.signum() * self.zoom_speed;
            self.distance = self.distance.clamp(self.min_distance, self.max_distance);
        }

        // WASD + Q/E: move target (camera follows)
        let dt = input.frame_time();
        let speed = self.move_speed * dt;
        let forward = self.forward_vector();
        let right = self.right_vector();

        if input.is_key_down(KeyCode::W) {
            self.target += forward * speed;
        }
        if input.is_key_down(KeyCode::S) {
            self.target -= forward * speed;
        }
        if input.is_key_down(KeyCode::A) {
            self.target -= right * speed;
        }
        if input.is_key_down(KeyCode::D) {
            self.target += right * speed;
        }
        if input.is_key_down(KeyCode::Q) {
            self.target -= Vec3::Y * speed;
        }
        if input.is_key_down(KeyCode::E) {
            self.target += Vec3::Y * speed;
        }

        self.apply_to_camera(camera);
    }

    /// Compute camera position from spherical coordinates and apply.
    pub fn apply_to_camera(&self, camera: &mut Camera) {
        let position = self.compute_position();
        camera.position = position;
        camera.target = self.target;
    }

    /// Compute camera position from spherical coordinates relative to target.
    pub fn compute_position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.cos();
        self.target + vec3(x, y, z)
    }

    /// Snap to a standard view while preserving target and distance.
    pub fn snap_front(&mut self) {
        self.azimuth = 0.0;
        self.elevation = 0.0;
    }

    /// Snap to right view (looking from +X toward origin).
    pub fn snap_right(&mut self) {
        self.azimuth = std::f32::consts::FRAC_PI_2;
        self.elevation = 0.0;
    }

    /// Snap to top view (looking from +Y down).
    pub fn snap_top(&mut self) {
        self.azimuth = 0.0;
        self.elevation = std::f32::consts::FRAC_PI_2 - 0.001; // Near-vertical, avoid gimbal lock
    }

    /// Forward direction projected onto the XZ ground plane (based on azimuth).
    fn forward_vector(&self) -> Vec3 {
        vec3(-self.azimuth.sin(), 0.0, -self.azimuth.cos())
    }

    fn right_vector(&self) -> Vec3 {
        // Right is perpendicular to forward in the XZ plane
        vec3(self.azimuth.cos(), 0.0, -self.azimuth.sin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ScriptedInput;
    use macroquad::prelude::vec2;

    #[test]
    fn middle_drag_right_increases_azimuth() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_azimuth = orbit.azimuth;

        let input = ScriptedInput::default()
            .with_mouse_button(MouseButton::Middle)
            .with_mouse_delta(vec2(0.01, 0.0));

        orbit.update(&mut cam, &input);
        assert!(orbit.azimuth > initial_azimuth);
    }

    #[test]
    fn middle_drag_up_increases_elevation() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_elevation = orbit.elevation;

        // Negative Y delta = upward (screen coords are Y-down)
        let input = ScriptedInput::default()
            .with_mouse_button(MouseButton::Middle)
            .with_mouse_delta(vec2(0.0, -0.01));

        orbit.update(&mut cam, &input);
        assert!(orbit.elevation > initial_elevation);
    }

    #[test]
    fn elevation_clamps_at_extremes() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        orbit.elevation = 1.4;

        let input = ScriptedInput::default()
            .with_mouse_button(MouseButton::Middle)
            .with_mouse_delta(vec2(0.0, -1.0));

        orbit.update(&mut cam, &input);
        assert!(orbit.elevation <= 1.5);
    }

    #[test]
    fn scroll_up_decreases_distance() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_distance = orbit.distance;

        let input = ScriptedInput::default().with_mouse_wheel(1.0);

        orbit.update(&mut cam, &input);
        assert!(orbit.distance < initial_distance);
    }

    #[test]
    fn zoom_clamps_to_min_distance() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        orbit.distance = 0.6;

        let input = ScriptedInput::default().with_mouse_wheel(100.0);

        orbit.update(&mut cam, &input);
        assert!(orbit.distance >= orbit.min_distance);
    }

    #[test]
    fn right_drag_right_moves_target_right() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_target_x = orbit.target.x;

        let input = ScriptedInput::default()
            .with_mouse_button(MouseButton::Right)
            .with_mouse_delta(vec2(0.01, 0.0));

        orbit.update(&mut cam, &input);
        assert!(orbit.target.x > initial_target_x);
    }

    #[test]
    fn wasd_w_moves_target_forward() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_z = orbit.target.z;

        let input = ScriptedInput::default().with_key_down(KeyCode::W);

        orbit.update(&mut cam, &input);
        // Forward at azimuth=0 is -Z
        assert!(orbit.target.z < initial_z);
    }

    #[test]
    fn no_input_does_not_change_orbit_state() {
        let mut cam = Camera::default();
        let mut orbit = OrbitController::from_camera(&cam);
        let initial_azimuth = orbit.azimuth;
        let initial_elevation = orbit.elevation;
        let initial_distance = orbit.distance;
        let initial_target = orbit.target;

        let input = ScriptedInput::default();
        orbit.update(&mut cam, &input);

        assert_eq!(orbit.azimuth, initial_azimuth);
        assert_eq!(orbit.elevation, initial_elevation);
        assert_eq!(orbit.distance, initial_distance);
        assert_eq!(orbit.target, initial_target);
    }

    #[test]
    fn compute_position_at_zero_angles() {
        let orbit = OrbitController::new(Vec3::ZERO, 10.0);
        // azimuth=0, elevation=0.3 → camera at (0, sin(0.3)*10, cos(0.3)*10) roughly
        let pos = OrbitController {
            elevation: 0.0,
            ..orbit
        }
        .compute_position();
        // With azimuth=0, elevation=0: position should be (0, 0, distance)
        assert!((pos.x).abs() < 1e-5);
        assert!((pos.y).abs() < 1e-5);
        assert!((pos.z - 10.0).abs() < 1e-5);
    }

    #[test]
    fn compute_position_with_azimuth() {
        let orbit = OrbitController {
            target: Vec3::ZERO,
            distance: 10.0,
            azimuth: std::f32::consts::FRAC_PI_2, // 90 degrees
            elevation: 0.0,
            ..OrbitController::new(Vec3::ZERO, 10.0)
        };
        let pos = orbit.compute_position();
        // azimuth=PI/2 → camera at (10, 0, 0)
        assert!((pos.x - 10.0).abs() < 1e-4);
        assert!((pos.y).abs() < 1e-5);
        assert!((pos.z).abs() < 1e-4);
    }

    #[test]
    fn compute_position_with_elevation() {
        let orbit = OrbitController {
            target: Vec3::ZERO,
            distance: 10.0,
            azimuth: 0.0,
            elevation: std::f32::consts::FRAC_PI_4, // 45 degrees
            ..OrbitController::new(Vec3::ZERO, 10.0)
        };
        let pos = orbit.compute_position();
        // elevation=PI/4 → y = 10*sin(PI/4), z = 10*cos(PI/4)
        let expected_y = 10.0 * std::f32::consts::FRAC_PI_4.sin();
        let expected_z = 10.0 * std::f32::consts::FRAC_PI_4.cos();
        assert!((pos.y - expected_y).abs() < 1e-4);
        assert!((pos.z - expected_z).abs() < 1e-4);
    }

    #[test]
    fn compute_position_with_offset_target() {
        let orbit = OrbitController {
            target: vec3(5.0, 5.0, 5.0),
            distance: 10.0,
            azimuth: 0.0,
            elevation: 0.0,
            ..OrbitController::new(Vec3::ZERO, 10.0)
        };
        let pos = orbit.compute_position();
        assert!((pos.x - 5.0).abs() < 1e-5);
        assert!((pos.y - 5.0).abs() < 1e-5);
        assert!((pos.z - 15.0).abs() < 1e-5);
    }

    #[test]
    fn apply_to_camera_sets_position_and_target() {
        let orbit = OrbitController {
            target: Vec3::ZERO,
            distance: 10.0,
            azimuth: 0.0,
            elevation: 0.0,
            ..OrbitController::new(Vec3::ZERO, 10.0)
        };
        let mut cam = Camera::default();
        orbit.apply_to_camera(&mut cam);
        assert_eq!(cam.target, Vec3::ZERO);
        assert!((cam.position.z - 10.0).abs() < 1e-5);
    }

    #[test]
    fn from_camera_roundtrip() {
        let cam = Camera::new(vec3(0.0, 5.0, 10.0), Vec3::ZERO);
        let orbit = OrbitController::from_camera(&cam);
        let pos = orbit.compute_position();
        assert!((pos - cam.position).length() < 1e-4);
        assert_eq!(orbit.target, cam.target);
    }

    #[test]
    fn snap_front_sets_azimuth_and_elevation() {
        let mut orbit = OrbitController::new(Vec3::ZERO, 10.0);
        orbit.azimuth = 1.0;
        orbit.elevation = 0.5;
        orbit.snap_front();
        assert_eq!(orbit.azimuth, 0.0);
        assert_eq!(orbit.elevation, 0.0);
    }

    #[test]
    fn snap_right_sets_azimuth_pi_over_2() {
        let mut orbit = OrbitController::new(Vec3::ZERO, 10.0);
        orbit.snap_right();
        assert!((orbit.azimuth - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(orbit.elevation, 0.0);
    }

    #[test]
    fn snap_top_sets_near_vertical_elevation() {
        let mut orbit = OrbitController::new(Vec3::ZERO, 10.0);
        orbit.snap_top();
        assert_eq!(orbit.azimuth, 0.0);
        assert!(orbit.elevation > 1.5);
    }

    #[test]
    fn snap_preserves_target_and_distance() {
        let mut orbit = OrbitController::new(vec3(5.0, 3.0, 1.0), 20.0);
        orbit.snap_front();
        assert_eq!(orbit.target, vec3(5.0, 3.0, 1.0));
        assert_eq!(orbit.distance, 20.0);
    }

    #[test]
    fn snap_front_camera_position() {
        let mut orbit = OrbitController::new(Vec3::ZERO, 10.0);
        orbit.snap_front();
        let pos = orbit.compute_position();
        // Front view: camera at (0, 0, 10)
        assert!((pos.x).abs() < 1e-5);
        assert!((pos.y).abs() < 1e-5);
        assert!((pos.z - 10.0).abs() < 1e-5);
    }

    #[test]
    fn from_camera_distance() {
        let cam = Camera::new(vec3(3.0, 4.0, 0.0), Vec3::ZERO);
        let orbit = OrbitController::from_camera(&cam);
        assert!((orbit.distance - 5.0).abs() < 1e-5);
    }
}
