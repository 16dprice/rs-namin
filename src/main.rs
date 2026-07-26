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
    // Start in the viewer on the scratch scene (the iteration workflow) or
    // on RS_NAMIN_SCENE; the library is one click or Esc away.
    let name = std::env::var("RS_NAMIN_SCENE").unwrap_or_else(|_| "my_scene".to_string());
    let entry = registry::find(&name).unwrap_or_else(|| {
        eprintln!("Unknown scene: {name}");
        std::process::exit(1);
    });
    app::run(AppMode::viewer(entry)).await;
}
