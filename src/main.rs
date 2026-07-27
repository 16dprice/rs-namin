use macroquad::prelude::Conf;

use rs_namin::app::{self, AppMode};
use rs_namin::registry;

fn window_conf() -> Conf {
    Conf {
        window_title: "rs-namin".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Open on the scene library. RS_NAMIN_SCENE=name jumps straight into the
    // viewer on that scene (dev iteration and frame-dump verification).
    let mode = match std::env::var("RS_NAMIN_SCENE") {
        Ok(name) => match registry::find(&name) {
            Some(entry) => AppMode::viewer(entry),
            None => {
                eprintln!("Unknown scene: {name}");
                std::process::exit(1);
            }
        },
        Err(_) => AppMode::Library,
    };
    app::run(mode).await;
}
