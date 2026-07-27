use std::io::Write;

use indicatif::{ProgressBar, ProgressStyle};
use inquire::{CustomType, InquireError, Select, Text};
use macroquad::prelude::*;

use rs_namin::export::{
    EncodeSettings, EncodingMode, RESOLUTION_PRESETS, ResolutionPreset, build_ffmpeg_args, doc_export_defaults, frame_range,
    preset_by_label, recommended_bitrate, spawn_ffmpeg, timestamped_output_path,
};
use rs_namin::registry::{self, SceneEntry};
use rs_namin::render_util::OffscreenRenderer;

struct FpsPreset {
    fps: u32,
}

impl std::fmt::Display for FpsPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} fps", self.fps)
    }
}

fn prompt_cancelled(err: &InquireError) -> bool {
    matches!(err, InquireError::OperationCanceled | InquireError::OperationInterrupted)
}

struct ExportConfig {
    scene: SceneEntry,
    settings: EncodeSettings,
    start_time: f32,
    end_time: f32,
    /// Explicit output path; None = timestamped file under renders/.
    output: Option<String>,
}

fn prompt_scene() -> Option<SceneEntry> {
    match Select::new("Scene", registry::scenes().to_vec()).prompt() {
        Ok(v) => Some(v),
        Err(e) if prompt_cancelled(&e) => None,
        Err(e) => panic!("{e}"),
    }
}

fn prompt_config(scene: SceneEntry, duration: f32) -> Option<ExportConfig> {
    // 1. Resolution
    let resolution = match Select::new("Resolution", RESOLUTION_PRESETS.to_vec())
        .with_starting_cursor(1)
        .prompt()
    {
        Ok(r) => r,
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 2. Frame rate
    let fps_presets = vec![FpsPreset { fps: 30 }, FpsPreset { fps: 60 }];

    let fps = match Select::new("Frame rate", fps_presets).with_starting_cursor(1).prompt() {
        Ok(f) => f.fps,
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 3. Encoding mode
    let encoding_choices = vec!["CRF (constant quality)", "Bitrate (YouTube recommended)"];
    let encoding = match Select::new("Encoding mode", encoding_choices).prompt() {
        Ok("CRF (constant quality)") => {
            let crf = match CustomType::<u32>::new("CRF value")
                .with_default(18)
                .with_help_message("Lower = better quality, 0 = lossless. 18 is visually lossless.")
                .prompt()
            {
                Ok(c) => c.min(51),
                Err(e) if prompt_cancelled(&e) => return None,
                Err(e) => panic!("{e}"),
            };
            EncodingMode::Crf { crf }
        }
        Ok(_) => {
            let recommended = recommended_bitrate(resolution.label, fps);
            let kbps = match CustomType::<u32>::new("Bitrate (kbps)")
                .with_default(recommended)
                .with_help_message(&format!(
                    "YouTube recommends {} Mbps for {}@{}fps",
                    recommended / 1000,
                    resolution.label,
                    fps
                ))
                .prompt()
            {
                Ok(k) => k,
                Err(e) if prompt_cancelled(&e) => return None,
                Err(e) => panic!("{e}"),
            };
            EncodingMode::Bitrate { kbps }
        }
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 4. Start/end time
    println!("Scene duration: {duration:.2}s");

    let start_time = match CustomType::<f32>::new("Start time (seconds)")
        .with_default(0.0)
        .with_help_message(&format!("0.0 – {duration:.2}"))
        .prompt()
    {
        Ok(t) => t.clamp(0.0, duration),
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    let end_time = match CustomType::<f32>::new("End time (seconds)")
        .with_default(duration)
        .with_help_message(&format!("{start_time:.2} – {duration:.2}"))
        .prompt()
    {
        Ok(t) => t.clamp(start_time, duration),
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 5. Audio
    let default_audio = scene.audio.unwrap_or("");
    let audio_path = match Text::new("Audio file path (empty for none)")
        .with_default(default_audio)
        .with_help_message("Path to an audio file (mp3, wav, etc.)")
        .prompt()
    {
        Ok(s) if s.trim().is_empty() => None,
        Ok(s) => {
            let path = std::path::Path::new(s.trim());
            if !path.exists() {
                eprintln!("Warning: audio file not found: {}", s.trim());
                eprintln!("Continuing without audio.");
                None
            } else {
                Some(s.trim().to_string())
            }
        }
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    Some(ExportConfig {
        scene,
        settings: EncodeSettings {
            resolution,
            fps,
            encoding,
            audio_path,
        },
        start_time,
        end_time,
        output: None,
    })
}

const USAGE: &str = "Usage: export [--scene NAME] [--resolution 720p|1080p|1440p|4K|WxH] [--fps N] [--crf N | --bitrate KBPS] \
                     [--start S] [--end S] [--audio PATH] [--output PATH]\n\
                     With no arguments, prompts interactively. --scene enables non-interactive mode \
                     (defaults: 1080p, 60 fps, CRF 18, full scene range).";

/// Flags for non-interactive export. `None` from `parse_cli` means no flags
/// were given and the interactive prompts should run instead.
struct CliArgs {
    scene: String,
    resolution: Option<String>,
    fps: Option<u32>,
    crf: Option<u32>,
    bitrate: Option<u32>,
    start: Option<f32>,
    end: Option<f32>,
    audio: Option<String>,
    output: Option<String>,
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

fn parse_cli() -> Option<CliArgs> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        return None;
    }

    let mut scene: Option<String> = None;
    let mut cli = CliArgs {
        scene: String::new(),
        resolution: None,
        fps: None,
        crf: None,
        bitrate: None,
        start: None,
        end: None,
        audio: None,
        output: None,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => usage_exit(0),
            "--scene" => scene = Some(flag_value(&args, &mut i, "--scene").to_string()),
            "--resolution" => cli.resolution = Some(flag_value(&args, &mut i, "--resolution").to_string()),
            "--fps" => cli.fps = Some(flag_value(&args, &mut i, "--fps").parse().expect("--fps requires an integer")),
            "--crf" => cli.crf = Some(flag_value(&args, &mut i, "--crf").parse().expect("--crf requires an integer")),
            "--bitrate" => {
                cli.bitrate = Some(
                    flag_value(&args, &mut i, "--bitrate")
                        .parse()
                        .expect("--bitrate requires an integer (kbps)"),
                )
            }
            "--start" => cli.start = Some(flag_value(&args, &mut i, "--start").parse().expect("--start requires a float")),
            "--end" => cli.end = Some(flag_value(&args, &mut i, "--end").parse().expect("--end requires a float")),
            "--audio" => cli.audio = Some(flag_value(&args, &mut i, "--audio").to_string()),
            "--output" => cli.output = Some(flag_value(&args, &mut i, "--output").to_string()),
            other => {
                eprintln!("Unknown argument: {other}");
                usage_exit(1);
            }
        }
        i += 1;
    }

    match scene {
        Some(name) => {
            cli.scene = name;
            Some(cli)
        }
        None => {
            eprintln!("--scene is required when passing flags (omit all flags for interactive mode)");
            usage_exit(1);
        }
    }
}

fn parse_resolution(s: &str) -> ResolutionPreset {
    if let Some(preset) = preset_by_label(s) {
        return preset;
    }
    let parse_dims = || -> Option<(u32, u32)> {
        let (w, h) = s.split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    };
    match parse_dims() {
        Some((width, height)) => ResolutionPreset {
            label: "custom",
            width,
            height,
        },
        None => {
            eprintln!("Invalid resolution {s:?} (expected a preset or WxH, e.g. 1920x1080)");
            usage_exit(1);
        }
    }
}

/// Build an ExportConfig from CLI flags without prompting.
fn config_from_cli(scene: SceneEntry, cli: CliArgs, duration: f32) -> ExportConfig {
    // Flags win; then the scene document's stored defaults; then app defaults.
    let doc_defaults = doc_export_defaults(&scene).unwrap_or_default();
    let resolution_arg = cli.resolution.or(doc_defaults.resolution);
    let resolution = parse_resolution(resolution_arg.as_deref().unwrap_or("1080p"));
    let fps = cli.fps.or(doc_defaults.fps).unwrap_or(60);
    let encoding = match (cli.crf, cli.bitrate) {
        (Some(crf), _) => EncodingMode::Crf { crf: crf.min(51) },
        (None, Some(kbps)) => EncodingMode::Bitrate { kbps },
        (None, None) => EncodingMode::Crf { crf: 18 },
    };
    let start_time = cli.start.unwrap_or(0.0).clamp(0.0, duration);
    let end_time = cli.end.unwrap_or(duration).clamp(start_time, duration);
    let audio_path = cli.audio.or_else(|| scene.audio.map(String::from));

    ExportConfig {
        scene,
        settings: EncodeSettings {
            resolution,
            fps,
            encoding,
            audio_path,
        },
        start_time,
        end_time,
        output: cli.output.or(doc_defaults.output),
    }
}

fn main() {
    let cli = parse_cli();

    // Resolve the scene entry (prompting if no flags were given) before
    // creating the window.
    let (entry, cli) = match cli {
        Some(cli) => {
            let entry = registry::find(&cli.scene).unwrap_or_else(|| {
                eprintln!("Unknown scene: {}", cli.scene);
                usage_exit(1);
            });
            (entry.clone(), Some(cli))
        }
        None => match prompt_scene() {
            Some(entry) => (entry, None),
            None => return,
        },
    };

    // Create window first so the GL context is available for scene building
    // (some scenes load textures which require it).
    let conf = Conf {
        window_title: "rs-namin export".to_owned(),
        window_width: 1280,
        window_height: 720,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    macroquad::Window::from_config(conf, export_main(entry, cli));
}

async fn export_main(entry: SceneEntry, cli: Option<CliArgs>) {
    // Build scene inside async context where GL is available.
    let (scene, timeline, camera) = match entry.build_scene() {
        Ok(built) => built,
        Err(error) => {
            eprintln!("Failed to build scene: {error}");
            return;
        }
    };
    let duration = timeline.duration();

    let config = match cli {
        Some(cli) => config_from_cli(entry, cli, duration),
        None => match prompt_config(entry, duration) {
            Some(c) => c,
            None => return,
        },
    };

    let fps = config.settings.fps;
    let end_time = config.end_time.min(duration);
    let (start_frame, end_frame) = frame_range(config.start_time, end_time, fps);

    if end_frame <= start_frame {
        println!("Nothing to render.");
        return;
    }

    export_render(scene, timeline, camera, config, start_frame, end_frame).await;
}

async fn export_render(
    mut scene: rs_namin::scene::Scene,
    timeline: rs_namin::animation::timeline::Timeline,
    camera: rs_namin::camera::Camera,
    config: ExportConfig,
    start_frame: u32,
    end_frame: u32,
) {
    let initial_camera = camera;
    let (width, height) = (config.settings.resolution.width, config.settings.resolution.height);
    let fps = config.settings.fps;
    let total_frames = end_frame - start_frame;

    let renderer = OffscreenRenderer::new(width, height);

    let output_path = match &config.output {
        Some(path) => {
            if let Some(parent) = std::path::Path::new(path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).ok();
            }
            path.clone()
        }
        None => timestamped_output_path(config.scene.name, config.settings.resolution.label, fps),
    };

    let ffmpeg_args = build_ffmpeg_args(&config.settings, &output_path);

    let mut ffmpeg = spawn_ffmpeg(&ffmpeg_args).expect("Failed to spawn ffmpeg. Is it installed?");

    let stdin = ffmpeg.stdin.as_mut().unwrap();
    let mut rgb_buf = Vec::with_capacity((width * height * 3) as usize);

    let render_duration = config.end_time - config.start_time;
    let pb = ProgressBar::new(total_frames as u64);
    pb.set_style(
        ProgressStyle::with_template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta} remaining)")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message(format!(
        "{render_duration:.1}s @ {fps}fps {} → {output_path}",
        config.settings.resolution.label,
    ));

    // --- Render loop ---

    let mut pending_frame = false;

    for frame in start_frame..end_frame {
        // Read back the previously rendered frame (now flushed)
        if pending_frame {
            renderer.read_rgb(&mut rgb_buf);
            if stdin.write_all(&rgb_buf).is_err() {
                pb.abandon_with_message("ffmpeg pipe broken, aborting");
                break;
            }
            pb.inc(1);
        }

        // Render this frame to the render target
        let t = frame as f32 / fps as f32;
        let mut camera = initial_camera.clone();
        timeline.apply(t, &mut scene, &mut camera);
        renderer.render_frame(&scene, &camera);

        pending_frame = true;
        next_frame().await;
    }

    // Read back the final frame
    if pending_frame {
        renderer.read_rgb(&mut rgb_buf);
        let _ = stdin.write_all(&rgb_buf);
    }
    pb.inc(1);

    // Close stdin and wait for ffmpeg to finish
    drop(ffmpeg.stdin.take());
    let _ = ffmpeg.wait();

    pb.finish_with_message(format!("Done: {output_path}"));
}
