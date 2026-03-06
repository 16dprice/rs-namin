use macroquad::prelude::KeyCode;

/// All debug keybindings in one place. Change keys here to remap controls.
pub struct Keybindings {
    pub toggle_hud: KeyCode,
    pub toggle_scrub_bar: KeyCode,
    pub toggle_value_inspector: KeyCode,
    pub play_pause: KeyCode,
    pub step_forward: KeyCode,
    pub step_backward: KeyCode,
    pub speed_up: KeyCode,
    pub speed_down: KeyCode,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            toggle_hud: KeyCode::F1,
            toggle_scrub_bar: KeyCode::F2,
            toggle_value_inspector: KeyCode::F3,
            play_pause: KeyCode::Space,
            step_forward: KeyCode::Right,
            step_backward: KeyCode::Left,
            speed_up: KeyCode::Up,
            speed_down: KeyCode::Down,
        }
    }
}
