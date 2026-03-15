pub mod camera_log;
pub mod keybindings;
pub mod scrub_bar;
pub mod value_inspector;

use macroquad::prelude::*;

use crate::camera::orbit::OrbitController;
use crate::camera::Camera;
use crate::clock::{Clock, LoopMode, PlaybackState};
use crate::input::InputProvider;
use crate::scene::Scene;

use camera_log::CameraLog;
use keybindings::Keybindings;
use scrub_bar::ScrubBar;
use value_inspector::ValueInspector;

pub struct DebugOverlay {
    pub keybindings: Keybindings,
    pub hud_visible: bool,
    pub world_helpers_visible: bool,
    /// When true, the camera is driven by the timeline instead of the orbit controller.
    pub camera_follow_timeline: bool,
    pub scrub_bar: ScrubBar,
    pub value_inspector: ValueInspector,
    pub camera_log: CameraLog,
    pub bounding_boxes_visible: bool,
    pub mouse_coords_visible: bool,
}

/// Result of snap-to-view input handling. The caller uses this to set the orbit state.
pub enum SnapView {
    None,
    Front,
    Right,
    Top,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            keybindings: Keybindings::default(),
            hud_visible: true,
            world_helpers_visible: true,
            camera_follow_timeline: false,
            scrub_bar: ScrubBar::new(),
            value_inspector: ValueInspector::new(),
            camera_log: CameraLog::new(256),
            bounding_boxes_visible: false,
            mouse_coords_visible: false,
        }
    }

    /// Handle keybindings for toggling overlays and transport controls.
    /// Call this at the start of each frame, before clock.tick().
    /// Returns a `SnapView` if a snap-to-view key was pressed.
    pub fn handle_input(&mut self, clock: &mut Clock, input: &dyn InputProvider) -> SnapView {
        let kb = &self.keybindings;

        if input.is_key_pressed(kb.toggle_hud) {
            self.hud_visible = !self.hud_visible;
        }
        if input.is_key_pressed(kb.toggle_scrub_bar) {
            self.scrub_bar.visible = !self.scrub_bar.visible;
        }
        if input.is_key_pressed(kb.toggle_value_inspector) {
            self.value_inspector.visible = !self.value_inspector.visible;
        }
        if input.is_key_pressed(kb.toggle_world_helpers) {
            self.world_helpers_visible = !self.world_helpers_visible;
        }
        if input.is_key_pressed(kb.toggle_camera_follow) {
            self.camera_follow_timeline = !self.camera_follow_timeline;
        }
        if input.is_key_pressed(kb.toggle_mouse_coords) {
            self.mouse_coords_visible = !self.mouse_coords_visible;
        }

        if input.is_key_pressed(kb.play_pause) {
            clock.toggle();
        }
        if input.is_key_pressed(kb.step_forward) {
            clock.pause();
            clock.step_forward();
        }
        if input.is_key_pressed(kb.step_backward) {
            clock.pause();
            clock.step_backward();
        }
        if input.is_key_pressed(kb.speed_up) {
            clock.set_speed((clock.playback_speed * 2.0).min(8.0));
        }
        if input.is_key_pressed(kb.speed_down) {
            clock.set_speed((clock.playback_speed * 0.5).max(0.125));
        }

        // Snap-to-view
        if input.is_key_pressed(kb.snap_front) {
            return SnapView::Front;
        }
        if input.is_key_pressed(kb.snap_right) {
            return SnapView::Right;
        }
        if input.is_key_pressed(kb.snap_top) {
            return SnapView::Top;
        }

        SnapView::None
    }

    /// Update interactive elements (scrub bar dragging). Call after handle_input.
    pub fn update(&mut self, clock: &mut Clock) {
        self.scrub_bar.update(clock);
    }

    /// Record the current camera state in the log.
    pub fn record_camera(&mut self, camera: &Camera, time: f32) {
        self.camera_log.record(camera, time);
    }

    /// Draw world-space debug helpers (toggle: F4). Call while 3D camera is active.
    pub fn draw_world(&self, orbit: &OrbitController, scene: &Scene) {
        if !self.world_helpers_visible {
            return;
        }
        self.draw_grid(20, 1.0);
        self.draw_origin_axes(2.0);
        self.draw_orbit_crosshair(orbit);

        if self.bounding_boxes_visible {
            self.draw_bounding_boxes(scene);
        }
    }

    /// Draw all visible screen-space overlays. Call after set_default_camera().
    pub fn draw(&self, clock: &Clock, scene: &Scene, camera: &Camera) {
        if self.hud_visible {
            self.draw_hud(clock, scene, camera);
        }
        if self.mouse_coords_visible {
            self.draw_mouse_coords(camera);
        }
        self.scrub_bar.draw(clock);
        self.value_inspector.draw(scene);
    }

    fn draw_hud(&self, clock: &Clock, scene: &Scene, camera: &Camera) {
        let x = 10.0;
        let mut y = 30.0;
        let line_h = 20.0;
        let font_size = 16.0;
        let color = LIGHTGRAY;

        let state_str = match clock.playback_state {
            PlaybackState::Playing => "Playing",
            PlaybackState::Paused => "Paused",
        };

        let loop_str = match clock.loop_mode {
            LoopMode::Once => "Once",
            LoopMode::Loop => "Loop",
            LoopMode::PingPong => "PingPong",
        };

        let p = camera.position;
        let t = camera.target;
        let fwd = camera.forward();

        let cam_mode = if self.camera_follow_timeline {
            "Timeline"
        } else {
            "Orbit"
        };

        let hud_lines = [
            format!(
                "Time: {:.2} / {:.2}s",
                clock.current_time, clock.duration
            ),
            format!("State: {}  Speed: {:.2}x", state_str, clock.playback_speed),
            format!("Loop: {}  Camera: {} (F5)", loop_str, cam_mode),
            format!("Objects: {}", scene.len()),
            format!(
                "Cam: ({:.1}, {:.1}, {:.1})  Target: ({:.1}, {:.1}, {:.1})",
                p.x, p.y, p.z, t.x, t.y, t.z
            ),
            format!(
                "Fwd: ({:.2}, {:.2}, {:.2})  Dist: {:.1}  FOV: {:.0}",
                fwd.x, fwd.y, fwd.z, camera.distance(), camera.fov
            ),
        ];

        for line in &hud_lines {
            draw_text(line, x, y, font_size, color);
            y += line_h;
        }
    }

    fn draw_grid(&self, half_size: i32, spacing: f32) {
        let grid_color = Color::new(0.3, 0.3, 0.3, 0.5);
        let extent = half_size as f32 * spacing;

        for i in -half_size..=half_size {
            let pos = i as f32 * spacing;
            // Lines along Z axis
            draw_line_3d(vec3(pos, 0.0, -extent), vec3(pos, 0.0, extent), grid_color);
            // Lines along X axis
            draw_line_3d(vec3(-extent, 0.0, pos), vec3(extent, 0.0, pos), grid_color);
        }
    }

    fn draw_origin_axes(&self, length: f32) {
        draw_line_3d(Vec3::ZERO, vec3(length, 0.0, 0.0), RED);   // X
        draw_line_3d(Vec3::ZERO, vec3(0.0, length, 0.0), GREEN); // Y
        draw_line_3d(Vec3::ZERO, vec3(0.0, 0.0, length), BLUE);  // Z
    }

    /// Draw a small crosshair at the orbit controller's target point.
    fn draw_orbit_crosshair(&self, orbit: &OrbitController) {
        let t = orbit.target;
        let size = orbit.distance * 0.02; // Scale with distance so it stays visible
        let color = YELLOW;

        draw_line_3d(t - vec3(size, 0.0, 0.0), t + vec3(size, 0.0, 0.0), color);
        draw_line_3d(t - vec3(0.0, size, 0.0), t + vec3(0.0, size, 0.0), color);
        draw_line_3d(t - vec3(0.0, 0.0, size), t + vec3(0.0, 0.0, size), color);
    }

    /// Draw mouse cursor world coordinates (raycast onto Y=0 ground plane).
    fn draw_mouse_coords(&self, camera: &Camera) {
        let (mx, my) = mouse_position();
        let sw = screen_width();
        let sh = screen_height();

        // Convert mouse to NDC (-1..1)
        let ndc_x = (mx / sw) * 2.0 - 1.0;
        let ndc_y = 1.0 - (my / sh) * 2.0; // flip Y

        let mq_cam = camera.to_macroquad();
        let aspect = sw / sh;

        // Build view and projection matrices
        let view = Mat4::look_at_rh(mq_cam.position, mq_cam.target, mq_cam.up);
        let proj = Mat4::perspective_rh_gl(mq_cam.fovy, aspect, mq_cam.z_near, mq_cam.z_far);
        let inv_vp = (proj * view).inverse();

        // Unproject near and far points
        let near_pt = inv_vp.project_point3(vec3(ndc_x, ndc_y, -1.0));
        let far_pt = inv_vp.project_point3(vec3(ndc_x, ndc_y, 1.0));
        let ray_dir = (far_pt - near_pt).normalize();

        // Intersect with Z=0 plane (XY scene plane)
        let label = if ray_dir.z.abs() > 1e-6 {
            let t = -near_pt.z / ray_dir.z;
            if t > 0.0 {
                let hit = near_pt + ray_dir * t;
                format!("({:.2}, {:.2})", hit.x, hit.y)
            } else {
                "no hit".to_string()
            }
        } else {
            "parallel to plane".to_string()
        };

        // Draw near cursor with slight offset
        let font_size = 16.0;
        draw_text(&label, mx + 15.0, my - 10.0, font_size, YELLOW);
    }

    /// Draw wireframe bounding boxes for all scene objects.
    fn draw_bounding_boxes(&self, scene: &Scene) {
        let color = Color::new(0.0, 1.0, 0.0, 0.4);
        for (_id, obj) in scene.iter() {
            if obj.is_screen_space() {
                continue;
            }
            let bb = obj.bounding_box();
            draw_aabb(bb.min, bb.max, color);
        }
    }
}

/// Draw an axis-aligned bounding box as 12 wireframe edges.
fn draw_aabb(min: Vec3, max: Vec3, color: Color) {
    // Bottom face (y = min.y)
    draw_line_3d(vec3(min.x, min.y, min.z), vec3(max.x, min.y, min.z), color);
    draw_line_3d(vec3(max.x, min.y, min.z), vec3(max.x, min.y, max.z), color);
    draw_line_3d(vec3(max.x, min.y, max.z), vec3(min.x, min.y, max.z), color);
    draw_line_3d(vec3(min.x, min.y, max.z), vec3(min.x, min.y, min.z), color);

    // Top face (y = max.y)
    draw_line_3d(vec3(min.x, max.y, min.z), vec3(max.x, max.y, min.z), color);
    draw_line_3d(vec3(max.x, max.y, min.z), vec3(max.x, max.y, max.z), color);
    draw_line_3d(vec3(max.x, max.y, max.z), vec3(min.x, max.y, max.z), color);
    draw_line_3d(vec3(min.x, max.y, max.z), vec3(min.x, max.y, min.z), color);

    // Vertical edges
    draw_line_3d(vec3(min.x, min.y, min.z), vec3(min.x, max.y, min.z), color);
    draw_line_3d(vec3(max.x, min.y, min.z), vec3(max.x, max.y, min.z), color);
    draw_line_3d(vec3(max.x, min.y, max.z), vec3(max.x, max.y, max.z), color);
    draw_line_3d(vec3(min.x, min.y, max.z), vec3(min.x, max.y, max.z), color);
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ScriptedInput;

    #[test]
    fn play_pause_toggle() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);
        clock.pause();

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.play_pause);

        overlay.handle_input(&mut clock, &input);
        assert!(matches!(clock.playback_state, PlaybackState::Playing));
    }

    #[test]
    fn toggle_hud() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);
        assert!(overlay.hud_visible);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.toggle_hud);

        overlay.handle_input(&mut clock, &input);
        assert!(!overlay.hud_visible);
    }

    #[test]
    fn toggle_camera_follow() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);
        assert!(!overlay.camera_follow_timeline);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.toggle_camera_follow);

        overlay.handle_input(&mut clock, &input);
        assert!(overlay.camera_follow_timeline);
    }

    #[test]
    fn speed_up() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.speed_up);

        overlay.handle_input(&mut clock, &input);
        assert!((clock.playback_speed - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn step_forward_pauses() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);
        clock.play();

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.step_forward);

        overlay.handle_input(&mut clock, &input);
        assert!(matches!(clock.playback_state, PlaybackState::Paused));
        assert!(clock.current_time > 0.0);
    }

    #[test]
    fn snap_front_returns_snap_view() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.snap_front);

        let result = overlay.handle_input(&mut clock, &input);
        assert!(matches!(result, SnapView::Front));
    }

    #[test]
    fn snap_right_returns_snap_view() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.snap_right);

        let result = overlay.handle_input(&mut clock, &input);
        assert!(matches!(result, SnapView::Right));
    }

    #[test]
    fn snap_top_returns_snap_view() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);

        let input = ScriptedInput::default()
            .with_key_pressed(overlay.keybindings.snap_top);

        let result = overlay.handle_input(&mut clock, &input);
        assert!(matches!(result, SnapView::Top));
    }

    #[test]
    fn no_snap_key_returns_none() {
        let mut overlay = DebugOverlay::new();
        let mut clock = Clock::new(10.0, 60.0);

        let input = ScriptedInput::default();
        let result = overlay.handle_input(&mut clock, &input);
        assert!(matches!(result, SnapView::None));
    }
}
