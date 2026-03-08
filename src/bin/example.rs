use std::fmt;

use inquire::{InquireError, Select};
use macroquad::prelude::*;

use rs_namin::camera::orbit::OrbitController;
use rs_namin::clock::{self, Clock};
use rs_namin::debug::{DebugOverlay, SnapView};
use rs_namin::examples;
use rs_namin::input::MacroquadInput;

struct ExampleChoice {
    name: &'static str,
    description: &'static str,
}

impl fmt::Display for ExampleChoice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

fn main() {
    let choices: Vec<ExampleChoice> = examples::EXAMPLES
        .iter()
        .map(|e| ExampleChoice {
            name: e.name,
            description: e.description,
        })
        .collect();

    let selection = Select::new("Choose an example:", choices).prompt();

    let name = match selection {
        Ok(choice) => choice.name,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            eprintln!("Cancelled.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let example = examples::find(name).expect("selected example not found");
    let (scene, timeline, camera) = (example.build)();

    let conf = Conf {
        window_title: format!("rs-namin — {}", name),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    };

    macroquad::Window::from_config(conf, run(scene, timeline, camera));
}

async fn run(
    mut scene: rs_namin::scene::Scene,
    timeline: rs_namin::animation::timeline::Timeline,
    mut camera: rs_namin::camera::Camera,
) {
    let initial_camera = camera.clone();

    let mut clock = Clock::new(timeline.duration(), 60.0);
    clock.loop_mode = clock::LoopMode::Loop;
    clock.play();

    let mut debug_overlay = DebugOverlay::new();
    let mut orbit = OrbitController::from_camera(&camera);

    loop {
        clear_background(BLACK);

        let input = MacroquadInput;
        let snap = debug_overlay.handle_input(&mut clock, &input);
        debug_overlay.update(&mut clock);

        match snap {
            SnapView::Front => orbit.snap_front(),
            SnapView::Right => orbit.snap_right(),
            SnapView::Top => orbit.snap_top(),
            SnapView::None => {}
        }

        clock.tick(get_frame_time());

        if debug_overlay.camera_follow_timeline {
            camera = initial_camera.clone();
            timeline.apply(clock.current_time, &mut scene, &mut camera);
        } else {
            timeline.apply_scene_only(clock.current_time, &mut scene);
        }

        set_camera(&camera.to_macroquad());
        debug_overlay.draw_world(&orbit, &scene);
        scene.draw_world();

        set_default_camera();
        scene.draw_screen();
        debug_overlay.draw(&clock, &scene, &camera);
        debug_overlay
            .scrub_bar
            .draw_ticks(&timeline, clock.duration);

        if debug_overlay.camera_follow_timeline {
            orbit = OrbitController::from_camera(&camera);
        } else {
            orbit.update(&mut camera, &input);
        }

        debug_overlay.record_camera(&camera, clock.current_time);

        next_frame().await;
    }
}
