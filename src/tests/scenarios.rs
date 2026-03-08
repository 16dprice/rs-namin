use macroquad::prelude::{vec2, KeyCode, MouseButton};

use crate::input::ScriptedInput;
use crate::my_scene;
use crate::scenario::Scenario;

#[test]
fn orbit_accumulation_rightward() {
    let (mut scene, timeline, mut camera) = my_scene::build();

    // mouse_delta is in -2..2 range; pixel delta = 0.008 * 1280 * 0.5 = 5.12 px
    // azimuth per frame = 5.12 * orbit_speed(0.005) = 0.0256 rad
    // after 60 frames: 0.0256 * 60 ≈ 1.536 rad ≈ π/2
    let drag_right = ScriptedInput::default()
        .with_mouse_button(MouseButton::Middle)
        .with_mouse_delta(vec2(0.008, 0.0));

    Scenario::new()
        .run_frames(60, drag_right)
        .assert_orbit("azimuth should be roughly π/2", |o| {
            (o.azimuth - std::f32::consts::FRAC_PI_2).abs() < 0.1
        })
        .assert_camera("camera X should be positive", |c| c.position.x > 0.0)
        .run(&mut scene, &timeline, &mut camera);
}

#[test]
fn zoom_to_min_clamp() {
    let (mut scene, timeline, mut camera) = my_scene::build();

    let scroll_in = ScriptedInput::default().with_mouse_wheel(5.0);

    Scenario::new()
        .run_frames(100, scroll_in)
        .assert_orbit("distance should be at minimum", |o| {
            (o.distance - o.min_distance).abs() < 0.01
        })
        .run(&mut scene, &timeline, &mut camera);
}

#[test]
fn pan_then_orbit() {
    let (mut scene, timeline, mut camera) = my_scene::build();

    let pan_right = ScriptedInput::default()
        .with_mouse_button(MouseButton::Right)
        .with_mouse_delta(vec2(0.01, 0.0));

    let orbit_right = ScriptedInput::default()
        .with_mouse_button(MouseButton::Middle)
        .with_mouse_delta(vec2(0.01, 0.0));

    Scenario::new()
        .run_frames(30, pan_right)
        .assert_orbit("target should have moved right", |o| o.target.x > 0.0)
        .run_frames(30, orbit_right)
        .assert_orbit("azimuth should have increased", |o| o.azimuth > 0.0)
        .run(&mut scene, &timeline, &mut camera);
}

#[test]
fn idle_full_timeline() {
    let (mut scene, timeline, mut camera) = my_scene::build();
    let duration = timeline.duration();
    let total_frames = (duration * 60.0).ceil() as u32;

    Scenario::new()
        .idle(total_frames)
        .assert_scene("scene should still have objects", |s| !s.is_empty())
        .run(&mut scene, &timeline, &mut camera);
}

#[test]
fn no_input_camera_stable() {
    let (mut scene, timeline, mut camera) = my_scene::build();
    let initial_pos = camera.position;

    Scenario::new()
        .idle(60)
        .assert_camera("camera position should not drift", move |c| {
            (c.position - initial_pos).length() < 1e-4
        })
        .run(&mut scene, &timeline, &mut camera);
}

#[test]
fn wasd_movement_accumulates() {
    let (mut scene, timeline, mut camera) = my_scene::build();

    let move_forward = ScriptedInput::default().with_key_down(KeyCode::W);

    Scenario::new()
        .run_frames(60, move_forward)
        .assert_orbit("target should have moved forward (-Z)", |o| o.target.z < 0.0)
        .run(&mut scene, &timeline, &mut camera);
}
