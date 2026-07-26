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
    // Start in the viewer on the scratch scene (the iteration workflow);
    // the library is one click or Esc away.
    let entry = registry::find("my_scene").expect("my_scene is registered");
    app::run(AppMode::viewer(entry)).await;
}
