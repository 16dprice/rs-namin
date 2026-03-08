use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use rs_namin::my_scene;
use rs_namin::render_util::rgba_flipped;

struct SnapshotConfig {
    times: Vec<f32>,
    width: u32,
    height: u32,
    output: PathBuf,
}

fn parse_args() -> SnapshotConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut times: Option<Vec<f32>> = None;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut output = PathBuf::from("snapshot.png");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--time" => {
                i += 1;
                let t: f32 = args[i].parse().expect("--time requires a float value");
                times = Some(vec![t]);
            }
            "--times" => {
                i += 1;
                let ts: Vec<f32> = args[i]
                    .split(',')
                    .map(|s| s.trim().parse().expect("--times requires comma-separated floats"))
                    .collect();
                times = Some(ts);
            }
            "--width" => {
                i += 1;
                width = args[i].parse().expect("--width requires an integer");
            }
            "--height" => {
                i += 1;
                height = args[i].parse().expect("--height requires an integer");
            }
            "--output" => {
                i += 1;
                output = PathBuf::from(&args[i]);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                eprintln!("Usage: snapshot [--time T | --times T1,T2,...] [--width W] [--height H] [--output PATH]");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    SnapshotConfig {
        times: times.unwrap_or_else(|| vec![0.0]),
        width,
        height,
        output,
    }
}

fn main() {
    let (scene, timeline, camera) = my_scene::build();
    let config = parse_args();

    // Clamp times to scene duration
    let duration = timeline.duration();
    let times: Vec<f32> = config.times.iter().map(|t| t.clamp(0.0, duration)).collect();

    let conf = Conf {
        window_title: "rs-namin snapshot".to_owned(),
        window_width: config.width.min(1280) as i32,
        window_height: config.height.min(720) as i32,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    macroquad::Window::from_config(
        conf,
        snapshot_render(scene, timeline, camera, times, config.width, config.height, config.output),
    );
}

fn save_png(rgba_data: &[u8], width: u32, height: u32, path: &std::path::Path) {
    image::save_buffer(path, rgba_data, width, height, image::ColorType::Rgba8)
        .unwrap_or_else(|e| panic!("Failed to save PNG to {}: {e}", path.display()));
}

async fn snapshot_render(
    mut scene: rs_namin::scene::Scene,
    timeline: rs_namin::animation::timeline::Timeline,
    initial_camera: rs_namin::camera::Camera,
    times: Vec<f32>,
    width: u32,
    height: u32,
    output: PathBuf,
) {
    let rt = render_target(width, height);
    rt.texture.set_filter(FilterMode::Nearest);

    let multiple = times.len() > 1;

    // If multiple times and output doesn't look like a directory, treat it as a directory
    if multiple {
        std::fs::create_dir_all(&output).ok();
    }

    let mut rgba_buf = Vec::with_capacity((width * height * 4) as usize);
    let mut pending: Option<(f32, usize)> = None;

    for (idx, &t) in times.iter().enumerate() {
        // Read back the previously rendered frame (now flushed by next_frame)
        if let Some((prev_t, prev_idx)) = pending {
            let image = rt.texture.get_texture_data();
            let data = image.get_image_data();
            rgba_flipped(data, width as usize, height as usize, &mut rgba_buf);

            let path = output_path(&output, multiple, prev_t, prev_idx);
            save_png(&rgba_buf, width, height, &path);
            eprintln!("Saved: {} (t={prev_t:.3}s)", path.display());
        }

        // Render this frame
        let mut camera = initial_camera.clone();
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

        pending = Some((t, idx));
        next_frame().await;
    }

    // Read back the final frame
    if let Some((prev_t, prev_idx)) = pending {
        let image = rt.texture.get_texture_data();
        let data = image.get_image_data();
        rgba_flipped(data, width as usize, height as usize, &mut rgba_buf);

        let path = output_path(&output, multiple, prev_t, prev_idx);
        save_png(&rgba_buf, width, height, &path);
        eprintln!("Saved: {} (t={prev_t:.3}s)", path.display());
    }
}

fn output_path(base: &Path, multiple: bool, time: f32, _idx: usize) -> PathBuf {
    if multiple {
        // output is a directory — name files by time
        base.join(format!("t{time:.3}.png"))
    } else {
        // Single file output
        let p = base.to_path_buf();
        if p.extension().is_none() {
            p.with_extension("png")
        } else {
            p
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_path_single() {
        let base = PathBuf::from("frame.png");
        let result = output_path(&base, false, 1.5, 0);
        assert_eq!(result, PathBuf::from("frame.png"));
    }

    #[test]
    fn output_path_single_no_extension() {
        let base = PathBuf::from("frame");
        let result = output_path(&base, false, 1.5, 0);
        assert_eq!(result, PathBuf::from("frame.png"));
    }

    #[test]
    fn output_path_multiple() {
        let base = PathBuf::from("frames");
        let result = output_path(&base, true, 1.5, 0);
        assert_eq!(result, PathBuf::from("frames/t1.500.png"));
    }

    #[test]
    fn output_path_multiple_zero_time() {
        let base = PathBuf::from("out");
        let result = output_path(&base, true, 0.0, 0);
        assert_eq!(result, PathBuf::from("out/t0.000.png"));
    }

    #[test]
    fn parse_args_defaults() {
        // Can't easily test parse_args since it reads std::env::args,
        // but we test the output_path logic which is the tricky part.
    }
}
