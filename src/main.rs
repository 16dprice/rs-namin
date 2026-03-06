#![allow(dead_code)]

mod scene;

use macroquad::prelude::*;
use scene::objects::{Circle, Line};
use scene::Scene;

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
    let mut scene = Scene::new();
    scene.add(Circle::new(
        vec3(screen_width() / 2.0, screen_height() / 2.0, 0.0),
        100.0,
        BLUE,
    ));
    scene.add(Line::new(
        vec3(100.0, 100.0, 0.0),
        vec3(400.0, 300.0, 0.0),
        3.0,
        RED,
    ));

    loop {
        clear_background(BLACK);
        scene.draw_all();
        next_frame().await;
    }
}
