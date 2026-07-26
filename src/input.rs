use std::collections::HashSet;

use macroquad::prelude::{KeyCode, MouseButton, Vec2, vec2};

pub trait InputProvider {
    fn mouse_position(&self) -> Vec2;
    fn mouse_delta(&self) -> Vec2;
    fn mouse_wheel(&self) -> (f32, f32);
    fn is_mouse_button_down(&self, button: MouseButton) -> bool;
    fn is_key_down(&self, key: KeyCode) -> bool;
    fn is_key_pressed(&self, key: KeyCode) -> bool;
    fn screen_width(&self) -> f32;
    fn screen_height(&self) -> f32;
    fn frame_time(&self) -> f32;
}

pub struct MacroquadInput;

impl InputProvider for MacroquadInput {
    fn mouse_position(&self) -> Vec2 {
        let (x, y) = macroquad::prelude::mouse_position();
        vec2(x, y)
    }

    fn mouse_delta(&self) -> Vec2 {
        macroquad::prelude::mouse_delta_position()
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        macroquad::prelude::mouse_wheel()
    }

    fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        macroquad::prelude::is_mouse_button_down(button)
    }

    fn is_key_down(&self, key: KeyCode) -> bool {
        macroquad::prelude::is_key_down(key)
    }

    fn is_key_pressed(&self, key: KeyCode) -> bool {
        macroquad::prelude::is_key_pressed(key)
    }

    fn screen_width(&self) -> f32 {
        macroquad::prelude::screen_width()
    }

    fn screen_height(&self) -> f32 {
        macroquad::prelude::screen_height()
    }

    fn frame_time(&self) -> f32 {
        macroquad::prelude::get_frame_time()
    }
}

pub struct ScriptedInput {
    pub mouse_position: Vec2,
    pub mouse_delta: Vec2,
    pub mouse_wheel: (f32, f32),
    pub mouse_buttons_down: HashSet<MouseButton>,
    pub keys_down: HashSet<KeyCode>,
    pub keys_pressed: HashSet<KeyCode>,
    pub screen_width: f32,
    pub screen_height: f32,
    pub frame_time: f32,
}

impl Default for ScriptedInput {
    fn default() -> Self {
        Self {
            mouse_position: vec2(0.0, 0.0),
            mouse_delta: vec2(0.0, 0.0),
            mouse_wheel: (0.0, 0.0),
            mouse_buttons_down: HashSet::new(),
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            screen_width: 1280.0,
            screen_height: 720.0,
            frame_time: 1.0 / 60.0,
        }
    }
}

impl ScriptedInput {
    pub fn with_mouse_position(mut self, position: Vec2) -> Self {
        self.mouse_position = position;
        self
    }

    pub fn with_mouse_delta(mut self, delta: Vec2) -> Self {
        self.mouse_delta = delta;
        self
    }

    pub fn with_mouse_wheel(mut self, scroll: f32) -> Self {
        self.mouse_wheel = (0.0, scroll);
        self
    }

    pub fn with_mouse_button(mut self, button: MouseButton) -> Self {
        self.mouse_buttons_down.insert(button);
        self
    }

    pub fn with_key_down(mut self, key: KeyCode) -> Self {
        self.keys_down.insert(key);
        self
    }

    pub fn with_key_pressed(mut self, key: KeyCode) -> Self {
        self.keys_pressed.insert(key);
        self
    }

    pub fn with_screen_size(mut self, width: f32, height: f32) -> Self {
        self.screen_width = width;
        self.screen_height = height;
        self
    }

    pub fn with_frame_time(mut self, dt: f32) -> Self {
        self.frame_time = dt;
        self
    }
}

impl InputProvider for ScriptedInput {
    fn mouse_position(&self) -> Vec2 {
        self.mouse_position
    }

    fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    fn mouse_wheel(&self) -> (f32, f32) {
        self.mouse_wheel
    }

    fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    fn screen_width(&self) -> f32 {
        self.screen_width
    }

    fn screen_height(&self) -> f32 {
        self.screen_height
    }

    fn frame_time(&self) -> f32 {
        self.frame_time
    }
}
