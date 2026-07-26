#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Once,
    Loop,
    PingPong,
}

pub struct Clock {
    pub current_time: f32,
    pub playback_state: PlaybackState,
    pub playback_speed: f32,
    pub loop_mode: LoopMode,
    pub duration: f32,
    pub fps: f32,
    direction: f32, // 1.0 or -1.0, used for PingPong
}

impl Clock {
    pub fn new(duration: f32, fps: f32) -> Self {
        Self {
            current_time: 0.0,
            playback_state: PlaybackState::Paused,
            playback_speed: 1.0,
            loop_mode: LoopMode::Once,
            duration,
            fps,
            direction: 1.0,
        }
    }

    pub fn play(&mut self) {
        self.playback_state = PlaybackState::Playing;
    }

    pub fn pause(&mut self) {
        self.playback_state = PlaybackState::Paused;
    }

    pub fn toggle(&mut self) {
        self.playback_state = match self.playback_state {
            PlaybackState::Playing => PlaybackState::Paused,
            PlaybackState::Paused => PlaybackState::Playing,
        };
    }

    pub fn tick(&mut self, dt: f32) {
        if self.playback_state == PlaybackState::Paused {
            return;
        }

        let advance = dt * self.playback_speed * self.direction;
        self.current_time += advance;
        self.apply_loop_mode();
    }

    pub fn step_forward(&mut self) {
        self.current_time += 1.0 / self.fps;
        self.current_time = self.current_time.min(self.duration);
    }

    pub fn step_backward(&mut self) {
        self.current_time -= 1.0 / self.fps;
        self.current_time = self.current_time.max(0.0);
    }

    pub fn scrub(&mut self, time: f32) {
        self.current_time = time.clamp(0.0, self.duration);
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed;
    }

    fn apply_loop_mode(&mut self) {
        // Empty timelines (duration 0) must pin time to 0: the Loop arm's
        // modulo would be x % 0.0 = NaN, which then poisons every keyframe
        // comparison downstream.
        if self.duration <= 0.0 {
            self.current_time = 0.0;
            return;
        }
        match self.loop_mode {
            LoopMode::Once => {
                self.current_time = self.current_time.clamp(0.0, self.duration);
                if self.current_time >= self.duration {
                    self.playback_state = PlaybackState::Paused;
                }
            }
            LoopMode::Loop => {
                if self.current_time > self.duration {
                    self.current_time %= self.duration;
                } else if self.current_time < 0.0 {
                    self.current_time = self.duration + (self.current_time % self.duration);
                }
            }
            LoopMode::PingPong => {
                if self.current_time >= self.duration {
                    self.current_time = self.duration;
                    self.direction = -1.0;
                } else if self.current_time <= 0.0 {
                    self.current_time = 0.0;
                    self.direction = 1.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_paused() {
        let clock = Clock::new(10.0, 60.0);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
        assert_eq!(clock.current_time, 0.0);
    }

    #[test]
    fn tick_does_not_advance_when_paused() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.tick(1.0);
        assert_eq!(clock.current_time, 0.0);
    }

    #[test]
    fn tick_advances_when_playing() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.play();
        clock.tick(0.5);
        assert!((clock.current_time - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn tick_respects_playback_speed() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.play();
        clock.set_speed(2.0);
        clock.tick(0.5);
        assert!((clock.current_time - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn toggle_play_pause() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.toggle();
        assert_eq!(clock.playback_state, PlaybackState::Playing);
        clock.toggle();
        assert_eq!(clock.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn once_mode_clamps_and_pauses_at_end() {
        let mut clock = Clock::new(2.0, 60.0);
        clock.loop_mode = LoopMode::Once;
        clock.play();
        clock.tick(3.0);
        assert!((clock.current_time - 2.0).abs() < f32::EPSILON);
        assert_eq!(clock.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn loop_mode_wraps() {
        let mut clock = Clock::new(2.0, 60.0);
        clock.loop_mode = LoopMode::Loop;
        clock.play();
        clock.tick(3.0);
        assert!((clock.current_time - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ping_pong_reverses_at_end() {
        let mut clock = Clock::new(2.0, 60.0);
        clock.loop_mode = LoopMode::PingPong;
        clock.play();
        clock.tick(2.0);
        assert!((clock.current_time - 2.0).abs() < f32::EPSILON);
        clock.tick(0.5);
        assert!((clock.current_time - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn ping_pong_reverses_at_start() {
        let mut clock = Clock::new(2.0, 60.0);
        clock.loop_mode = LoopMode::PingPong;
        clock.play();
        clock.tick(2.0);
        clock.tick(2.0);
        assert!((clock.current_time).abs() < f32::EPSILON);
        clock.tick(0.5);
        assert!((clock.current_time - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn step_forward() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.step_forward();
        let expected = 1.0 / 60.0;
        assert!((clock.current_time - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn step_forward_clamps_at_duration() {
        let mut clock = Clock::new(0.01, 60.0);
        clock.step_forward();
        assert!((clock.current_time - 0.01).abs() < f32::EPSILON);
    }

    #[test]
    fn step_backward() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.scrub(1.0);
        clock.step_backward();
        let expected = 1.0 - 1.0 / 60.0;
        assert!((clock.current_time - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn step_backward_clamps_at_zero() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.step_backward();
        assert_eq!(clock.current_time, 0.0);
    }

    #[test]
    fn zero_duration_loop_pins_time_instead_of_nan() {
        // Regression: playing a Loop-mode clock over an empty timeline
        // (duration 0, e.g. a freshly created scene document) computed
        // current_time %= 0.0 = NaN, which poisoned every downstream
        // keyframe comparison and crashed Track::evaluate.
        let mut clock = Clock::new(0.0, 60.0);
        clock.loop_mode = LoopMode::Loop;
        clock.play();
        for _ in 0..10 {
            clock.tick(0.5);
        }
        assert!(clock.current_time.is_finite());
        assert_eq!(clock.current_time, 0.0);
    }

    #[test]
    fn zero_duration_ping_pong_pins_time() {
        let mut clock = Clock::new(0.0, 60.0);
        clock.loop_mode = LoopMode::PingPong;
        clock.play();
        clock.tick(0.5);
        assert_eq!(clock.current_time, 0.0);
    }

    #[test]
    fn scrub_clamps_to_bounds() {
        let mut clock = Clock::new(5.0, 60.0);
        clock.scrub(-1.0);
        assert_eq!(clock.current_time, 0.0);
        clock.scrub(100.0);
        assert!((clock.current_time - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scrub_sets_exact_value() {
        let mut clock = Clock::new(10.0, 60.0);
        clock.scrub(3.5);
        assert!((clock.current_time - 3.5).abs() < f32::EPSILON);
    }
}
