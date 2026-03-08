use macroquad::prelude::Conf;

use rs_namin::videos;
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
    let (scene, timeline, camera) = videos::bouncing_ball::build();
    viewer::run(scene, timeline, camera).await;
}
