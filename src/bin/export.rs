use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

use indicatif::{ProgressBar, ProgressStyle};
use inquire::{CustomType, InquireError, Select, Text};
use macroquad::prelude::*;

use rs_namin::registry::{self, SceneEntry};
use rs_namin::render_util::OffscreenRenderer;

#[derive(Clone)]
struct ResolutionPreset {
    label: &'static str,
    width: u32,
    height: u32,
}

impl fmt::Display for ResolutionPreset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({}x{})", self.label, self.width, self.height)
    }
}

#[derive(Clone)]
struct FpsPreset {
    fps: u32,
}

impl fmt::Display for FpsPreset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} fps", self.fps)
    }
}

#[derive(Clone)]
enum EncodingMode {
    Crf { crf: u32 },
    Bitrate { kbps: u32 },
}

impl fmt::Display for EncodingMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncodingMode::Crf { crf } => write!(f, "CRF {crf} (constant quality)"),
            EncodingMode::Bitrate { kbps } => write!(f, "{} Mbps (target bitrate)", kbps / 1000),
        }
    }
}

fn prompt_cancelled(err: &InquireError) -> bool {
    matches!(err, InquireError::OperationCanceled | InquireError::OperationInterrupted)
}

struct ExportConfig {
    scene: SceneEntry,
    resolution: ResolutionPreset,
    fps: u32,
    encoding: EncodingMode,
    start_time: f32,
    end_time: f32,
    audio_path: Option<String>,
    /// Explicit output path; None = timestamped file under renders/.
    output: Option<String>,
}

/// YouTube recommended bitrate (kbps) for given resolution and frame rate.
fn recommended_bitrate(label: &str, fps: u32) -> u32 {
    match (label, fps) {
        ("720p", 30) => 5_000,
        ("720p", _) => 7_500,
        ("1080p", 30) => 8_000,
        ("1080p", _) => 16_000,
        ("1440p", 30) => 16_000,
        ("1440p", _) => 24_000,
        ("4K", 30) => 35_000,
        ("4K", _) => 53_000,
        _ => 16_000,
    }
}

fn prompt_scene() -> Option<SceneEntry> {
    match Select::new("Scene", registry::SCENES.to_vec()).prompt() {
        Ok(v) => Some(v),
        Err(e) if prompt_cancelled(&e) => None,
        Err(e) => panic!("{e}"),
    }
}

fn prompt_config(scene: SceneEntry, duration: f32) -> Option<ExportConfig> {
    // 1. Resolution
    let resolutions = vec![
        ResolutionPreset {
            label: "720p",
            width: 1280,
            height: 720,
        },
        ResolutionPreset {
            label: "1080p",
            width: 1920,
            height: 1080,
        },
        ResolutionPreset {
            label: "1440p",
            width: 2560,
            height: 1440,
        },
        ResolutionPreset {
            label: "4K",
            width: 3840,
            height: 2160,
        },
    ];

    let resolution = match Select::new("Resolution", resolutions).with_starting_cursor(1).prompt() {
        Ok(r) => r,
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 3. Frame rate
    let fps_presets = vec![FpsPreset { fps: 30 }, FpsPreset { fps: 60 }];

    let fps = match Select::new("Frame rate", fps_presets).with_starting_cursor(1).prompt() {
        Ok(f) => f.fps,
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 4. Encoding mode
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

    // 5. Start/end time
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

    // 6. Audio
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
        resolution,
        fps,
        encoding,
        start_time,
        end_time,
        audio_path,
        output: None,
    })
}

fn build_ffmpeg_args(config: &ExportConfig, output_path: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let width = config.resolution.width;
    let height = config.resolution.height;

    // Global
    args.extend(["-y".into()]);

    // Video input (rawvideo from stdin)
    args.extend([
        "-f".into(),
        "rawvideo".into(),
        "-pixel_format".into(),
        "rgb24".into(),
        "-video_size".into(),
        format!("{width}x{height}"),
        "-framerate".into(),
        format!("{}", config.fps),
        "-i".into(),
        "-".into(),
    ]);

    // Audio input (optional)
    if let Some(ref audio) = config.audio_path {
        args.extend(["-i".into(), audio.clone()]);
    }

    // Video encoding
    args.extend(["-c:v".into(), "libx264".into(), "-pix_fmt".into(), "yuv420p".into()]);

    match &config.encoding {
        EncodingMode::Crf { crf } => {
            args.extend(["-crf".into(), format!("{crf}")]);
        }
        EncodingMode::Bitrate { kbps } => {
            let maxrate = kbps * 3 / 2;
            let bufsize = kbps * 2;
            args.extend([
                "-b:v".into(),
                format!("{kbps}k"),
                "-maxrate".into(),
                format!("{maxrate}k"),
                "-bufsize".into(),
                format!("{bufsize}k"),
            ]);
        }
    }

    args.extend(["-preset".into(), "slow".into()]);

    // Audio encoding (if audio input provided)
    if config.audio_path.is_some() {
        args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "384k".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            "-shortest".into(),
        ]);
    }

    args.push(output_path.into());
    args
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
    match s {
        "720p" => ResolutionPreset {
            label: "720p",
            width: 1280,
            height: 720,
        },
        "1080p" => ResolutionPreset {
            label: "1080p",
            width: 1920,
            height: 1080,
        },
        "1440p" => ResolutionPreset {
            label: "1440p",
            width: 2560,
            height: 1440,
        },
        "4K" | "4k" => ResolutionPreset {
            label: "4K",
            width: 3840,
            height: 2160,
        },
        custom => {
            let parse_dims = || -> Option<(u32, u32)> {
                let (w, h) = custom.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            };
            match parse_dims() {
                Some((width, height)) => ResolutionPreset {
                    label: "custom",
                    width,
                    height,
                },
                None => {
                    eprintln!("Invalid resolution {custom:?} (expected a preset or WxH, e.g. 1920x1080)");
                    usage_exit(1);
                }
            }
        }
    }
}

/// Build an ExportConfig from CLI flags without prompting.
fn config_from_cli(scene: SceneEntry, cli: CliArgs, duration: f32) -> ExportConfig {
    let resolution = parse_resolution(cli.resolution.as_deref().unwrap_or("1080p"));
    let fps = cli.fps.unwrap_or(60);
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
        resolution,
        fps,
        encoding,
        start_time,
        end_time,
        audio_path,
        output: cli.output,
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
    let (scene, timeline, camera) = (entry.build)();
    let duration = timeline.duration();

    let config = match cli {
        Some(cli) => config_from_cli(entry, cli, duration),
        None => match prompt_config(entry, duration) {
            Some(c) => c,
            None => return,
        },
    };

    let fps = config.fps;

    let start_frame = (config.start_time * fps as f32).floor() as u32;
    let end_time = config.end_time.min(duration);
    let end_frame = (end_time * fps as f32).ceil() as u32;
    let total_frames = end_frame - start_frame;

    if total_frames == 0 {
        println!("Nothing to render.");
        return;
    }

    export_render(scene, timeline, camera, config, start_frame, end_frame, total_frames).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_count_whole_duration() {
        let fps: u32 = 60;
        let start_time: f32 = 0.0;
        let end_time: f32 = 1.0;
        let start_frame = (start_time * fps as f32).floor() as u32;
        let end_frame = (end_time * fps as f32).ceil() as u32;
        assert_eq!(start_frame, 0);
        assert_eq!(end_frame, 60);
        assert_eq!(end_frame - start_frame, 60);
    }

    #[test]
    fn frame_count_fractional_times() {
        let fps: u32 = 60;
        let start_time: f32 = 0.5;
        let end_time: f32 = 2.5;
        let start_frame = (start_time * fps as f32).floor() as u32;
        let end_frame = (end_time * fps as f32).ceil() as u32;
        assert_eq!(start_frame, 30);
        assert_eq!(end_frame, 150);
        assert_eq!(end_frame - start_frame, 120);
    }

    #[test]
    fn frame_count_non_aligned_end() {
        let fps: u32 = 60;
        let start_time: f32 = 0.0;
        let end_time: f32 = 1.01;
        let _start_frame = (start_time * fps as f32).floor() as u32;
        let end_frame = (end_time * fps as f32).ceil() as u32;
        assert_eq!(end_frame, 61);
    }

    #[test]
    fn frame_count_30fps() {
        let fps: u32 = 30;
        let start_time: f32 = 0.0;
        let end_time: f32 = 2.0;
        let start_frame = (start_time * fps as f32).floor() as u32;
        let end_frame = (end_time * fps as f32).ceil() as u32;
        assert_eq!(end_frame - start_frame, 60);
    }

    #[test]
    fn recommended_bitrate_youtube_specs() {
        assert_eq!(recommended_bitrate("1080p", 30), 8_000);
        assert_eq!(recommended_bitrate("1080p", 60), 16_000);
        assert_eq!(recommended_bitrate("4K", 30), 35_000);
        assert_eq!(recommended_bitrate("4K", 60), 53_000);
    }

    #[test]
    fn ffmpeg_args_crf_no_audio() {
        let scene = rs_namin::registry::find("bouncing_ball_long").unwrap().clone();
        let config = ExportConfig {
            scene,
            resolution: ResolutionPreset {
                label: "1080p",
                width: 1920,
                height: 1080,
            },
            fps: 60,
            encoding: EncodingMode::Crf { crf: 18 },
            start_time: 0.0,
            end_time: 1.0,
            audio_path: None,
            output: None,
        };
        let args = build_ffmpeg_args(&config, "out.mp4");
        assert!(args.contains(&"-crf".to_string()));
        assert!(args.contains(&"18".to_string()));
        assert!(!args.contains(&"-c:a".to_string()));
        assert!(!args.contains(&"-shortest".to_string()));
    }

    #[test]
    fn ffmpeg_args_bitrate_with_audio() {
        let scene = rs_namin::registry::find("bouncing_ball_long").unwrap().clone();
        let config = ExportConfig {
            scene,
            resolution: ResolutionPreset {
                label: "1080p",
                width: 1920,
                height: 1080,
            },
            fps: 60,
            encoding: EncodingMode::Bitrate { kbps: 16_000 },
            start_time: 0.0,
            end_time: 1.0,
            audio_path: Some("/tmp/test.mp3".to_string()),
            output: None,
        };
        let args = build_ffmpeg_args(&config, "out.mp4");
        assert!(args.contains(&"-b:v".to_string()));
        assert!(args.contains(&"16000k".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"384k".to_string()));
        assert!(args.contains(&"48000".to_string()));
        assert!(args.contains(&"-shortest".to_string()));
        assert!(args.contains(&"/tmp/test.mp3".to_string()));
    }
}

async fn export_render(
    mut scene: rs_namin::scene::Scene,
    timeline: rs_namin::animation::timeline::Timeline,
    camera: rs_namin::camera::Camera,
    config: ExportConfig,
    start_frame: u32,
    end_frame: u32,
    total_frames: u32,
) {
    let initial_camera = camera;
    let (width, height) = (config.resolution.width, config.resolution.height);
    let fps = config.fps;

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
        None => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            std::fs::create_dir_all("renders").ok();
            format!(
                "renders/{}_{}_{}fps_{}.mp4",
                config.scene.name, config.resolution.label, fps, timestamp
            )
        }
    };

    let ffmpeg_args = build_ffmpeg_args(&config, &output_path);

    let mut ffmpeg = Command::new("ffmpeg")
        .args(&ffmpeg_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ffmpeg. Is it installed?");

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
        config.resolution.label,
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
