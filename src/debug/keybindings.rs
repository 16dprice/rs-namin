use macroquad::prelude::KeyCode;

/// All debug keybindings in one place. Change keys here to remap controls.
pub struct Keybindings {
    pub toggle_hud: KeyCode,
    pub toggle_scrub_bar: KeyCode,
    pub toggle_value_inspector: KeyCode,
    pub toggle_world_helpers: KeyCode,
    pub toggle_camera_follow: KeyCode,
    pub play_pause: KeyCode,
    pub step_forward: KeyCode,
    pub step_backward: KeyCode,
    pub speed_up: KeyCode,
    pub speed_down: KeyCode,
    pub toggle_mouse_coords: KeyCode,
    // Snap-to-view keys (Blender-style numpad)
    pub snap_front: KeyCode,
    pub snap_right: KeyCode,
    pub snap_top: KeyCode,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            toggle_hud: KeyCode::F1,
            toggle_scrub_bar: KeyCode::F2,
            toggle_value_inspector: KeyCode::F3,
            toggle_world_helpers: KeyCode::F4,
            toggle_camera_follow: KeyCode::F5,
            toggle_mouse_coords: KeyCode::F6,
            play_pause: KeyCode::Space,
            step_forward: KeyCode::Right,
            step_backward: KeyCode::Left,
            speed_up: KeyCode::Up,
            speed_down: KeyCode::Down,
            snap_front: KeyCode::Kp1,
            snap_right: KeyCode::Kp3,
            snap_top: KeyCode::Kp7,
        }
    }
}
