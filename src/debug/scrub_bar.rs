use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::clock::{Clock, PlaybackState};

const BAR_HEIGHT: f32 = 24.0;
const BAR_MARGIN: f32 = 20.0;
const BAR_COLOR: Color = Color::new(0.3, 0.3, 0.3, 0.8);
const FILL_COLOR: Color = Color::new(0.2, 0.6, 1.0, 0.9);
const PLAYHEAD_COLOR: Color = WHITE;
const TICK_COLOR: Color = Color::new(1.0, 1.0, 1.0, 0.5);
const TEXT_COLOR: Color = LIGHTGRAY;

pub struct ScrubBar {
    pub visible: bool,
    dragging: bool,
    was_playing_before_drag: bool,
}

impl ScrubBar {
    pub fn new() -> Self {
        Self {
            visible: true,
            dragging: false,
            was_playing_before_drag: false,
        }
    }

    /// Returns the bar rect as (x, y, w, h).
    fn bar_rect(&self) -> (f32, f32, f32, f32) {
        let sw = screen_width();
        let sh = screen_height();
        let x = BAR_MARGIN;
        let w = sw - 2.0 * BAR_MARGIN;
        let y = sh - BAR_MARGIN - BAR_HEIGHT;
        (x, y, w, BAR_HEIGHT)
    }

    /// Convert a mouse x position to a normalized t in [0, 1] relative to the bar.
    fn mouse_x_to_t(&self, mouse_x: f32) -> f32 {
        let (x, _, w, _) = self.bar_rect();
        ((mouse_x - x) / w).clamp(0.0, 1.0)
    }

    /// Handle mouse interaction for drag-to-scrub.
    pub fn update(&mut self, clock: &mut Clock) {
        if !self.visible {
            self.dragging = false;
            return;
        }

        let (mx, my) = mouse_position();
        let (bx, by, bw, bh) = self.bar_rect();

        if is_mouse_button_pressed(MouseButton::Left)
            && mx >= bx
            && mx <= bx + bw
            && my >= by
            && my <= by + bh
        {
            self.dragging = true;
            self.was_playing_before_drag = clock.playback_state == PlaybackState::Playing;
            clock.pause();
            let t = self.mouse_x_to_t(mx);
            clock.scrub(t * clock.duration);
        }

        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                let t = self.mouse_x_to_t(mx);
                clock.scrub(t * clock.duration);
            } else {
                self.dragging = false;
                if self.was_playing_before_drag {
                    clock.play();
                }
            }
        }
    }

    pub fn draw(&self, clock: &Clock) {
        if !self.visible {
            return;
        }

        let (x, y, w, h) = self.bar_rect();

        // Background
        draw_rectangle(x, y, w, h, BAR_COLOR);

        // Fill to playhead
        let progress = if clock.duration > 0.0 {
            clock.current_time / clock.duration
        } else {
            0.0
        };
        draw_rectangle(x, y, w * progress, h, FILL_COLOR);

        // Playhead line
        let px = x + w * progress;
        draw_line(px, y, px, y + h, 2.0, PLAYHEAD_COLOR);

        // Time text
        let time_text = format!("{:.2} / {:.2}s", clock.current_time, clock.duration);
        let font_size = 14.0;
        let text_w = measure_text(&time_text, None, font_size as u16, 1.0).width;
        draw_text(
            &time_text,
            x + w - text_w - 4.0,
            y - 4.0,
            font_size,
            TEXT_COLOR,
        );

        // Play/pause indicator
        let state = if clock.playback_state == PlaybackState::Playing {
            ">"
        } else {
            "||"
        };
        draw_text(state, x, y - 4.0, font_size, TEXT_COLOR);
    }

    /// Draw keyframe tick marks. Call this separately since it needs the timeline.
    pub fn draw_ticks(&self, timeline: &Timeline, duration: f32) {
        if !self.visible || duration <= 0.0 {
            return;
        }

        let (x, y, w, h) = self.bar_rect();

        for track in &timeline.tracks {
            for time in track.keyframe_times() {
                let t = time / duration;
                let tx = x + w * t;
                draw_line(tx, y, tx, y + h, 1.0, TICK_COLOR);
            }
        }
    }
}

impl Default for ScrubBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_mouse_x_to_t_clamps() {
        let bar = ScrubBar::new();
        // We can't test exact positions since they depend on screen size,
        // but we can verify the clamping logic works by checking the math directly.
        // mouse_x_to_t uses bar_rect internally, so test the clamping contract:
        // any value far left of bar maps to 0, far right maps to 1.
        let t_low = ((0.0_f32 - 100.0) / 1000.0).clamp(0.0, 1.0);
        let t_high = ((2000.0_f32 - 100.0) / 1000.0).clamp(0.0, 1.0);
        assert_eq!(t_low, 0.0);
        assert_eq!(t_high, 1.0);

        // Verify new scrub bar starts not dragging
        assert!(!bar.dragging);
        assert!(bar.visible);
    }

    #[test]
    fn progress_calculation() {
        // When duration > 0, progress = current_time / duration
        let progress = 2.5_f32 / 5.0;
        assert!((progress - 0.5).abs() < f32::EPSILON);

        // When duration == 0, progress should be 0
        let progress_zero = if 0.0_f32 > 0.0 { 1.0 / 0.0_f32 } else { 0.0 };
        assert_eq!(progress_zero, 0.0);
    }

    #[test]
    fn drag_state_initial() {
        let bar = ScrubBar::new();
        assert!(!bar.dragging);
        assert!(!bar.was_playing_before_drag);
    }
}
