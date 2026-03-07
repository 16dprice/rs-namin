use std::io::Write;
use std::process::{Command, Stdio};

use macroquad::prelude::*;

use rs_namin::camera::Camera;
use rs_namin::my_scene;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FPS: f32 = 60.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "rs-namin export".to_owned(),
        window_width: WIDTH as i32,
        window_height: HEIGHT as i32,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0), // disable vsync for max speed
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Convert RGBA pixel data to RGB with Y-flip (OpenGL render targets are upside-down).
fn rgba_to_rgb_flipped(rgba: &[[u8; 4]], width: usize, height: usize, out: &mut Vec<u8>) {
    out.clear();
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba[y * width + x];
            out.push(pixel[0]);
            out.push(pixel[1]);
            out.push(pixel[2]);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let (mut scene, timeline) = my_scene::build();
    let duration = timeline.duration();
    let total_frames = (duration * FPS).ceil() as u32;

    let camera = Camera::new(vec3(0.0, 4.0, 15.0), vec3(0.0, 3.0, 0.0));
    let cam3d = camera.to_macroquad();

    let rt = render_target(WIDTH, HEIGHT);
    rt.texture.set_filter(FilterMode::Nearest);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    std::fs::create_dir_all("export_frames").ok();
    let output_path = format!("export_frames/{}.mp4", timestamp);

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "rawvideo",
            "-pixel_format", "rgb24",
            "-video_size", &format!("{}x{}", WIDTH, HEIGHT),
            "-framerate", &format!("{}", FPS),
            "-i", "-",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-preset", "fast",
            &output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ffmpeg. Is it installed?");

    let stdin = ffmpeg.stdin.as_mut().unwrap();
    let mut rgb_buf = Vec::with_capacity((WIDTH * HEIGHT * 3) as usize);

    eprintln!(
        "Exporting {} frames ({:.2}s at {}fps) to {}",
        total_frames, duration, FPS, output_path
    );

    // Frame 0 needs to be rendered first, then flushed via next_frame().
    // We render frame N, flush, then read back frame N on the next iteration.
    let mut pending_frame: Option<u32> = None;

    for frame in 0..total_frames {
        // Read back the previously rendered frame (now flushed)
        if pending_frame.is_some() {
            let image = rt.texture.get_texture_data();
            let data = image.get_image_data();
            rgba_to_rgb_flipped(data, WIDTH as usize, HEIGHT as usize, &mut rgb_buf);
            if stdin.write_all(&rgb_buf).is_err() {
                eprintln!("ffmpeg pipe broken, aborting");
                break;
            }
        }

        // Render this frame to the render target
        let t = frame as f32 / FPS;
        timeline.apply(t, &mut scene);

        let export_cam = Camera3D {
            render_target: Some(rt.clone()),
            ..cam3d
        };
        set_camera(&export_cam);
        clear_background(BLACK);
        scene.draw_world();

        // Screen-space pass: render text/overlay objects into the same render target.
        let screen_cam = Camera2D {
            zoom: vec2(2.0 / WIDTH as f32, -2.0 / HEIGHT as f32),
            target: vec2(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            render_target: Some(rt.clone()),
            ..Default::default()
        };
        set_camera(&screen_cam);
        scene.draw_screen();

        pending_frame = Some(frame);

        // Flush GPU commands
        next_frame().await;
    }

    // Read back the final frame
    if pending_frame.is_some() {
        let image = rt.texture.get_texture_data();
        let data = image.get_image_data();
        rgba_to_rgb_flipped(data, WIDTH as usize, HEIGHT as usize, &mut rgb_buf);
        let _ = stdin.write_all(&rgb_buf);
    }

    // Close stdin and wait for ffmpeg to finish
    drop(ffmpeg.stdin.take());
    let _ = ffmpeg.wait();

    eprintln!("Export complete: {}", output_path);
}
