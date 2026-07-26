use macroquad::prelude::Conf;

use rs_namin::app::{self, AppMode};

fn main() {
    let conf = Conf {
        window_title: "rs-namin — library".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        ..Default::default()
    };

    // Start in the library: the in-app scene list replaces the old
    // terminal picker.
    macroquad::Window::from_config(conf, app::run(AppMode::Library));
}
