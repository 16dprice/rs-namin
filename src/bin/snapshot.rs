use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use rs_namin::registry;
use rs_namin::render_util::OffscreenRenderer;

const USAGE: &str = "Usage: snapshot [--scene NAME] [--time T | --times T1,T2,...] [--width W] [--height H] [--output PATH]";

struct SnapshotConfig {
    times: Vec<f32>,
    width: u32,
    height: u32,
    output: PathBuf,
    scene: Option<String>,
}

fn usage_exit(code: i32) -> ! {
    eprintln!("{USAGE}");
    eprintln!("Available scenes: {}", registry::names().join(", "));
    std::process::exit(code);
}

/// Return the value following a flag, or exit with usage if it is missing.
fn flag_value<'a>(args: &'a [String], i: &mut usize, flag: &str) -> &'a str {
    *i += 1;
    match args.get(*i) {
        Some(v) => v,
        None => {
            eprintln!("{flag} requires a value");
            usage_exit(1);
        }
    }
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
            "--help" | "-h" => usage_exit(0),
            "--time" => {
                let t: f32 = flag_value(&args, &mut i, "--time").parse().expect("--time requires a float value");
                times = Some(vec![t]);
            }
            "--times" => {
                let ts: Vec<f32> = flag_value(&args, &mut i, "--times")
                    .split(',')
                    .map(|s| s.trim().parse().expect("--times requires comma-separated floats"))
                    .collect();
                times = Some(ts);
            }
            "--width" => {
                width = flag_value(&args, &mut i, "--width").parse().expect("--width requires an integer");
            }
            "--height" => {
                height = flag_value(&args, &mut i, "--height").parse().expect("--height requires an integer");
            }
            "--output" => {
                output = PathBuf::from(flag_value(&args, &mut i, "--output"));
            }
            "--scene" => {
                scene_name = Some(flag_value(&args, &mut i, "--scene").to_string());
            }
            other => {
                eprintln!("Unknown argument: {other}");
                usage_exit(1);
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
    let name = config.scene.as_deref().unwrap_or("my_scene");
    let entry = registry::find(name).unwrap_or_else(|| {
        eprintln!("Unknown scene: {name}");
        usage_exit(1);
    });
    let build_fn = entry.build;

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
        snapshot_render(build_fn, config.times, config.width, config.height, config.output),
    );
}

fn save_png(rgba_data: &[u8], width: u32, height: u32, path: &std::path::Path) {
    image::save_buffer(path, rgba_data, width, height, image::ColorType::Rgba8)
        .unwrap_or_else(|e| panic!("Failed to save PNG to {}: {e}", path.display()));
}

async fn snapshot_render(build_fn: registry::BuildFn, requested_times: Vec<f32>, width: u32, height: u32, output: PathBuf) {
    // Build the scene inside the GL context so texture creation works.
    let (mut scene, timeline, initial_camera) = build_fn();

    let duration = timeline.duration();
    let times: Vec<f32> = requested_times.iter().map(|t| t.clamp(0.0, duration)).collect();

    let renderer = OffscreenRenderer::new(width, height);

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
            renderer.read_rgba(&mut rgba_buf);
            let path = output_path(&output, multiple, prev_t, prev_idx);
            save_png(&rgba_buf, width, height, &path);
            eprintln!("Saved: {} (t={prev_t:.3}s)", path.display());
        }

        // Render this frame
        let mut camera = initial_camera.clone();
        timeline.apply(t, &mut scene, &mut camera);
        renderer.render_frame(&scene, &camera);

        pending = Some((t, idx));
        next_frame().await;
    }

    // Read back the final frame
    if let Some((prev_t, prev_idx)) = pending {
        renderer.read_rgba(&mut rgba_buf);
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
        if p.extension().is_none() { p.with_extension("png") } else { p }
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
}
