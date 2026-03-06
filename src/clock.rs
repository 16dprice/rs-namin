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
