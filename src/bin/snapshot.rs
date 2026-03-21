use std::path::{Path, PathBuf};

use macroquad::prelude::*;
use macroquad::texture::{RenderTargetParams, render_target_ex};

use rs_namin::examples;
use rs_namin::my_scene;
use rs_namin::render_util::rgba_flipped;

struct SnapshotConfig {
    times: Vec<f32>,
    width: u32,
    height: u32,
    output: PathBuf,
    scene: Option<String>,
}

fn parse_args() -> SnapshotConfig {
    let args: Vec<String> = std::env::args().collect();
    let mut times: Option<Vec<f32>> = None;
    let mut width: u32 = 1280;
    let mut height: u32 = 720;
    let mut output = PathBuf::from("snapshot.png");
    let mut scene_name: Option<String> = None;

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
                    .map(|s| {
                        s.trim()
                            .parse()
                            .expect("--times requires comma-separated floats")
                    })
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
            "--scene" => {
                i += 1;
                scene_name = Some(args[i].clone());
            }
            other => {
                eprintln!("Unknown argument: {other}");
                eprintln!(
                    "Usage: snapshot [--scene NAME] [--time T | --times T1,T2,...] [--width W] [--height H] [--output PATH]"
                );
                eprintln!("Available scenes: {}", examples::names().join(", "));
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
        scene: scene_name,
    }
}

fn main() {
    let config = parse_args();

    // Resolve the build function but don't call it yet — texture creation
    // requires the GL context which isn't available until inside the window.
    let build_fn: fn() -> (
        rs_namin::scene::Scene,
        rs_namin::animation::timeline::Timeline,
        rs_namin::camera::Camera,
    ) = if let Some(ref name) = config.scene {
        let example = examples::find(name).unwrap_or_else(|| {
            eprintln!("Unknown scene: {name}");
            eprintln!("Available: {}", examples::names().join(", "));
            std::process::exit(1);
        });
        example.build
    } else {
        my_scene::build
    };

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
        snapshot_render(
            build_fn,
            config.times,
            config.width,
            config.height,
            config.output,
        ),
    );
}

fn save_png(rgba_data: &[u8], width: u32, height: u32, path: &std::path::Path) {
    image::save_buffer(path, rgba_data, width, height, image::ColorType::Rgba8)
        .unwrap_or_else(|e| panic!("Failed to save PNG to {}: {e}", path.display()));
}

async fn snapshot_render(
    build_fn: fn() -> (
        rs_namin::scene::Scene,
        rs_namin::animation::timeline::Timeline,
        rs_namin::camera::Camera,
    ),
    requested_times: Vec<f32>,
    width: u32,
    height: u32,
    output: PathBuf,
) {
    // Build the scene inside the GL context so texture creation works.
    let (mut scene, timeline, initial_camera) = build_fn();

    let duration = timeline.duration();
    let times: Vec<f32> = requested_times
        .iter()
        .map(|t| t.clamp(0.0, duration))
        .collect();

    let rt = render_target_ex(
        width,
        height,
        RenderTargetParams {
            depth: true,
            ..Default::default()
        },
    );
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
        cam3d.viewport = Some((0, 0, width as i32, height as i32));
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
