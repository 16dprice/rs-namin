use inquire::{InquireError, Select};
use macroquad::prelude::Conf;

use rs_namin::registry::{self, SceneEntry};
use rs_namin::viewer;

fn main() {
    let choices: Vec<&'static SceneEntry> = registry::SCENES.iter().collect();

    let selection = Select::new("Choose a scene:", choices).prompt();

    let entry = match selection {
        Ok(entry) => entry,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            eprintln!("Cancelled.");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let conf = Conf {
        window_title: format!("rs-namin — {}", entry.name),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    };

    // Build inside the window future — scene builders may load textures,
    // which requires the GL context.
    macroquad::Window::from_config(conf, async move {
        let (scene, timeline, camera) = (entry.build)();
        viewer::run(scene, timeline, camera).await;
    });
}
