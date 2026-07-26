use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::input::ScriptedInput;
use crate::scene::Scene;

pub struct Scenario {
    steps: Vec<Step>,
}

enum Step {
    RunFrames { count: u32, input: ScriptedInput },
    AssertCamera(Box<dyn Fn(&Camera) -> bool>, String),
    AssertOrbit(Box<dyn Fn(&OrbitController) -> bool>, String),
    AssertScene(Box<dyn Fn(&Scene) -> bool>, String),
}

impl Scenario {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Run `count` frames with the given input repeated each frame.
    pub fn run_frames(mut self, count: u32, input: ScriptedInput) -> Self {
        self.steps.push(Step::RunFrames { count, input });
        self
    }

    /// Run `count` frames with no input (idle).
    pub fn idle(mut self, count: u32) -> Self {
        self.steps.push(Step::RunFrames {
            count,
            input: ScriptedInput::default(),
        });
        self
    }

    /// Assert a condition on the camera. Panics with `msg` if the closure returns false.
    pub fn assert_camera(mut self, msg: &str, check: impl Fn(&Camera) -> bool + 'static) -> Self {
        self.steps.push(Step::AssertCamera(Box::new(check), msg.to_string()));
        self
    }

    /// Assert a condition on the orbit controller state.
    pub fn assert_orbit(mut self, msg: &str, check: impl Fn(&OrbitController) -> bool + 'static) -> Self {
        self.steps.push(Step::AssertOrbit(Box::new(check), msg.to_string()));
        self
    }

    /// Assert a condition on the scene.
    pub fn assert_scene(mut self, msg: &str, check: impl Fn(&Scene) -> bool + 'static) -> Self {
        self.steps.push(Step::AssertScene(Box::new(check), msg.to_string()));
        self
    }

    /// Execute the scenario. Panics on assertion failure.
    pub fn run(self, scene: &mut Scene, timeline: &Timeline, camera: &mut Camera) {
        let mut orbit = OrbitController::from_camera(camera);
        let mut time = 0.0_f32;

        for step in self.steps {
            match step {
                Step::RunFrames { count, input } => {
                    let dt = input.frame_time;
                    for _ in 0..count {
                        time += dt;
                        timeline.apply(time, scene, camera);
                        orbit.update(camera, &input);
                    }
                }
                Step::AssertCamera(check, msg) => {
                    assert!(check(camera), "Camera assertion failed: {msg}");
                }
                Step::AssertOrbit(check, msg) => {
                    assert!(check(&orbit), "Orbit assertion failed: {msg}");
                }
                Step::AssertScene(check, msg) => {
                    assert!(check(scene), "Scene assertion failed: {msg}");
                }
            }
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::timeline::Timeline;
    use macroquad::prelude::Vec3;

    #[test]
    fn empty_scenario_runs() {
        let mut scene = Scene::new();
        let timeline = Timeline::new();
        let mut camera = Camera::default();
        Scenario::new().run(&mut scene, &timeline, &mut camera);
    }

    #[test]
    fn idle_advances_time() {
        let mut scene = Scene::new();
        let timeline = Timeline::new();
        let mut camera = Camera::default();

        // Idle shouldn't change camera position (no input, no timeline tracks)
        let initial_pos = camera.position;
        Scenario::new()
            .idle(60)
            .assert_camera("position unchanged after idle", move |c| (c.position - initial_pos).length() < 1e-5)
            .run(&mut scene, &timeline, &mut camera);
    }

    #[test]
    fn assert_camera_passes_on_true() {
        let mut scene = Scene::new();
        let timeline = Timeline::new();
        let mut camera = Camera::default();

        Scenario::new()
            .assert_camera("default camera has Y up", |c| c.up == Vec3::Y)
            .run(&mut scene, &timeline, &mut camera);
    }

    #[test]
    #[should_panic(expected = "Camera assertion failed")]
    fn assert_camera_panics_on_false() {
        let mut scene = Scene::new();
        let timeline = Timeline::new();
        let mut camera = Camera::default();

        Scenario::new()
            .assert_camera("impossible condition", |_c| false)
            .run(&mut scene, &timeline, &mut camera);
    }

    #[test]
    fn run_frames_with_input_changes_state() {
        use macroquad::prelude::{MouseButton, vec2};

        let mut scene = Scene::new();
        let timeline = Timeline::new();
        let mut camera = Camera::default();

        let drag_right = ScriptedInput::default()
            .with_mouse_button(MouseButton::Middle)
            .with_mouse_delta(vec2(0.01, 0.0));

        Scenario::new()
            .run_frames(10, drag_right)
            .assert_orbit("azimuth should have increased", |o| o.azimuth > 0.0)
            .run(&mut scene, &timeline, &mut camera);
    }
}
