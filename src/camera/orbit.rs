use macroquad::prelude::*;

use super::Camera;

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
    pub fn update(&mut self, camera: &mut Camera) {
        let (_, scroll_y) = mouse_wheel();
        // mouse_delta_position returns normalized coords (-1..1), scale to pixels
        let raw_delta = mouse_delta_position();
        // mouse_delta_position() returns coords in -2..2 range (normalized * 2 - 1).
        // Multiply by screen/2 to get pixel deltas.
        let delta = vec2(
            raw_delta.x * screen_width() * 0.5,
            raw_delta.y * screen_height() * 0.5,
        );

        // Middle-click drag: orbit
        if is_mouse_button_down(MouseButton::Middle) {
            self.azimuth += delta.x * self.orbit_speed;
            self.elevation -= delta.y * self.orbit_speed;
            self.elevation = self.elevation.clamp(-1.5, 1.5); // ~86 degrees
        }

        // Right-click drag: pan (1:1 with mouse — point under cursor stays under cursor)
        if is_mouse_button_down(MouseButton::Right) {
            let right = self.right_vector();
            let up = camera.up;
            // Convert pixel movement to world units at the target's depth.
            // For perspective: world_per_pixel = 2 * distance * tan(fov/2) / screen_height
            let fov_rad = camera.fov.to_radians();
            let world_per_pixel = 2.0 * self.distance * (fov_rad / 2.0).tan() / screen_height();
            self.target += (delta.x * right - delta.y * up) * world_per_pixel;
        }

        // Scroll: zoom
        if scroll_y.abs() > 0.0 {
            self.distance *= 1.0 - scroll_y.signum() * self.zoom_speed;
            self.distance = self.distance.clamp(self.min_distance, self.max_distance);
        }

        // WASD + Q/E: move target (camera follows)
        let dt = get_frame_time();
        let speed = self.move_speed * dt;
        let forward = self.forward_vector();
        let right = self.right_vector();

        if is_key_down(KeyCode::W) {
            self.target += forward * speed;
        }
        if is_key_down(KeyCode::S) {
            self.target -= forward * speed;
        }
        if is_key_down(KeyCode::A) {
            self.target -= right * speed;
        }
        if is_key_down(KeyCode::D) {
            self.target += right * speed;
        }
        if is_key_down(KeyCode::Q) {
            self.target -= Vec3::Y * speed;
        }
        if is_key_down(KeyCode::E) {
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
    fn from_camera_distance() {
        let cam = Camera::new(vec3(3.0, 4.0, 0.0), Vec3::ZERO);
        let orbit = OrbitController::from_camera(&cam);
        assert!((orbit.distance - 5.0).abs() < 1e-5);
    }
}
