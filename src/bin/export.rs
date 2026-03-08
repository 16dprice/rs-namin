use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

use indicatif::{ProgressBar, ProgressStyle};
use inquire::{CustomType, InquireError, Select};
use macroquad::prelude::*;

use rs_namin::my_scene;

const FPS: f32 = 60.0;

#[derive(Clone)]
struct QualityPreset {
    label: &'static str,
    width: u32,
    height: u32,
}

impl fmt::Display for QualityPreset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({}×{})", self.label, self.width, self.height)
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "rs-namin export (see terminal)".to_owned(),
        window_width: 320,
        window_height: 180,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
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

fn prompt_cancelled(err: &InquireError) -> bool {
    matches!(
        err,
        InquireError::OperationCanceled | InquireError::OperationInterrupted
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let (mut scene, timeline, mut camera) = my_scene::build();
    let initial_camera = camera.clone();
    let duration = timeline.duration();

    // --- Interactive prompts (in the terminal) ---

    let presets = vec![
        QualityPreset {
            label: "720p",
            width: 1280,
            height: 720,
        },
        QualityPreset {
            label: "1080p",
            width: 1920,
            height: 1080,
        },
        QualityPreset {
            label: "1440p",
            width: 2560,
            height: 1440,
        },
        QualityPreset {
            label: "4K",
            width: 3840,
            height: 2160,
        },
    ];

    let quality = match Select::new("Quality", presets).with_starting_cursor(3).prompt() {
        Ok(q) => q,
        Err(e) if prompt_cancelled(&e) => return,
        Err(e) => panic!("{e}"),
    };

    let (width, height) = (quality.width, quality.height);

    println!("Scene duration: {duration:.2}s");

    let start_time = match CustomType::<f32>::new("Start time (seconds)")
        .with_default(0.0)
        .with_help_message(&format!("0.0 – {duration:.2}"))
        .prompt()
    {
        Ok(t) => t.clamp(0.0, duration),
        Err(e) if prompt_cancelled(&e) => return,
        Err(e) => panic!("{e}"),
    };

    let end_time = match CustomType::<f32>::new("End time (seconds)")
        .with_default(duration)
        .with_help_message(&format!("{start_time:.2} – {duration:.2}"))
        .prompt()
    {
        Ok(t) => t.clamp(start_time, duration),
        Err(e) if prompt_cancelled(&e) => return,
        Err(e) => panic!("{e}"),
    };

    // --- Compute frame range ---

    let start_frame = (start_time * FPS).floor() as u32;
    let end_frame = (end_time * FPS).ceil() as u32;
    let total_frames = end_frame - start_frame;

    if total_frames == 0 {
        println!("Nothing to render.");
        return;
    }

    // --- Set up render target & ffmpeg ---

    let rt = render_target(width, height);
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
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &format!("{width}x{height}"),
            "-framerate",
            &format!("{FPS}"),
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "18",
            "-preset",
            "slow",
            &output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ffmpeg. Is it installed?");

    let stdin = ffmpeg.stdin.as_mut().unwrap();
    let mut rgb_buf = Vec::with_capacity((width * height * 3) as usize);

    let render_duration = end_time - start_time;
    let pb = ProgressBar::new(total_frames as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta} remaining)",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );
    pb.set_message(format!(
        "{render_duration:.1}s @ {FPS}fps {} → {output_path}",
        quality.label,
    ));

    // --- Render loop ---

    let mut pending_frame = false;

    for frame in start_frame..end_frame {
        // Read back the previously rendered frame (now flushed)
        if pending_frame {
            let image = rt.texture.get_texture_data();
            let data = image.get_image_data();
            rgba_to_rgb_flipped(data, width as usize, height as usize, &mut rgb_buf);
            if stdin.write_all(&rgb_buf).is_err() {
                pb.abandon_with_message("ffmpeg pipe broken, aborting");
                break;
            }
            pb.inc(1);
        }

        // Render this frame to the render target
        let t = frame as f32 / FPS;
        camera = initial_camera.clone();
        timeline.apply(t, &mut scene, &mut camera);

        let mut cam3d = camera.to_macroquad();
        cam3d.render_target = Some(rt.clone());
        set_camera(&cam3d);
        clear_background(BLACK);
        scene.draw_world();

        // Screen-space pass
        let screen_cam = Camera2D {
            zoom: vec2(2.0 / width as f32, -2.0 / height as f32),
            target: vec2(width as f32 / 2.0, height as f32 / 2.0),
            render_target: Some(rt.clone()),
            ..Default::default()
        };
        set_camera(&screen_cam);
        scene.draw_screen();

        pending_frame = true;
        next_frame().await;
    }

    // Read back the final frame
    if pending_frame {
        let image = rt.texture.get_texture_data();
        let data = image.get_image_data();
        rgba_to_rgb_flipped(data, width as usize, height as usize, &mut rgb_buf);
        let _ = stdin.write_all(&rgb_buf);
    }
    pb.inc(1);

    // Close stdin and wait for ffmpeg to finish
    drop(ffmpeg.stdin.take());
    let _ = ffmpeg.wait();

    pb.finish_with_message(format!("Done: {output_path}"));
}
