use std::fmt;

use inquire::{InquireError, Select};
use macroquad::prelude::Conf;

use rs_namin::examples;
use rs_namin::viewer;

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

    macroquad::Window::from_config(conf, viewer::run(scene, timeline, camera));
}
