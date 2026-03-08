use macroquad::prelude::Conf;

use rs_namin::my_scene;
use rs_namin::viewer;

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
    let (scene, timeline, camera) = my_scene::build();
    viewer::run(scene, timeline, camera).await;
}
