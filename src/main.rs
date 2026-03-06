use macroquad::prelude::*;

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
    loop {
        clear_background(BLACK);

        draw_circle(screen_width() / 2.0, screen_height() / 2.0, 100.0, BLUE);

        next_frame().await;
    }
}
