#[cfg(test)]
mod tests {
    use crate::clock::{Clock, LoopMode, PlaybackState};

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
        clock.tick(2.0); // reaches end
        assert!((clock.current_time - 2.0).abs() < f32::EPSILON);
        clock.tick(0.5); // should go backward
        assert!((clock.current_time - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn ping_pong_reverses_at_start() {
        let mut clock = Clock::new(2.0, 60.0);
        clock.loop_mode = LoopMode::PingPong;
        clock.play();
        // Go to end, reverse, go back to start
        clock.tick(2.0);
        clock.tick(2.0); // should hit 0
        assert!((clock.current_time).abs() < f32::EPSILON);
        clock.tick(0.5); // should go forward again
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
