pub mod keybindings;
pub mod scrub_bar;
pub mod value_inspector;

use macroquad::prelude::*;

use crate::camera::Camera;
use crate::clock::{Clock, LoopMode, PlaybackState};
use crate::scene::Scene;

use keybindings::Keybindings;
use scrub_bar::ScrubBar;
use value_inspector::ValueInspector;

pub struct DebugOverlay {
    pub keybindings: Keybindings,
    pub hud_visible: bool,
    pub scrub_bar: ScrubBar,
    pub value_inspector: ValueInspector,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            keybindings: Keybindings::default(),
            hud_visible: true,
            scrub_bar: ScrubBar::new(),
            value_inspector: ValueInspector::new(),
        }
    }

    /// Handle keybindings for toggling overlays and transport controls.
    /// Call this at the start of each frame, before clock.tick().
    pub fn handle_input(&mut self, clock: &mut Clock) {
        let kb = &self.keybindings;

        if is_key_pressed(kb.toggle_hud) {
            self.hud_visible = !self.hud_visible;
        }
        if is_key_pressed(kb.toggle_scrub_bar) {
            self.scrub_bar.visible = !self.scrub_bar.visible;
        }
        if is_key_pressed(kb.toggle_value_inspector) {
            self.value_inspector.visible = !self.value_inspector.visible;
        }

        if is_key_pressed(kb.play_pause) {
            clock.toggle();
        }
        if is_key_pressed(kb.step_forward) {
            clock.pause();
            clock.step_forward();
        }
        if is_key_pressed(kb.step_backward) {
            clock.pause();
            clock.step_backward();
        }
        if is_key_pressed(kb.speed_up) {
            clock.set_speed((clock.playback_speed * 2.0).min(8.0));
        }
        if is_key_pressed(kb.speed_down) {
            clock.set_speed((clock.playback_speed * 0.5).max(0.125));
        }
    }

    /// Update interactive elements (scrub bar dragging). Call after handle_input.
    pub fn update(&mut self, clock: &mut Clock) {
        self.scrub_bar.update(clock);
    }

    /// Draw world-space debug helpers. Call while 3D camera is active.
    pub fn draw_world(&self) {
        self.draw_grid(20, 1.0);
        self.draw_origin_axes(2.0);
    }

    /// Draw all visible screen-space overlays. Call after set_default_camera().
    pub fn draw(&self, clock: &Clock, scene: &Scene, camera: &Camera) {
        if self.hud_visible {
            self.draw_hud(clock, scene, camera);
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

        let hud_lines = [
            format!(
                "Time: {:.2} / {:.2}s",
                clock.current_time, clock.duration
            ),
            format!("State: {}  Speed: {:.2}x", state_str, clock.playback_speed),
            format!("Loop: {}", loop_str),
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
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}
