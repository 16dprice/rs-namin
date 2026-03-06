pub mod keybindings;
pub mod scrub_bar;
pub mod value_inspector;

use macroquad::prelude::*;

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

    /// Draw all visible overlays. Call after scene.draw_all().
    pub fn draw(&self, clock: &Clock, scene: &Scene) {
        if self.hud_visible {
            self.draw_hud(clock, scene);
        }
        self.scrub_bar.draw(clock);
        self.value_inspector.draw(scene);
    }

    fn draw_hud(&self, clock: &Clock, scene: &Scene) {
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

        let lines = [
            format!(
                "Time: {:.2} / {:.2}s",
                clock.current_time, clock.duration
            ),
            format!("State: {}  Speed: {:.2}x", state_str, clock.playback_speed),
            format!("Loop: {}", loop_str),
            format!("Objects: {}", scene.len()),
        ];

        for line in &lines {
            draw_text(line, x, y, font_size, color);
            y += line_h;
        }
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}
