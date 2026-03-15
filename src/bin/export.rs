use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

use indicatif::{ProgressBar, ProgressStyle};
use inquire::{CustomType, InquireError, Select, Text};
use macroquad::prelude::*;
use macroquad::texture::{render_target_ex, RenderTargetParams};

use rs_namin::render_util::rgba_to_rgb_flipped;
use rs_namin::videos::{self, Video};

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
    matches!(
        err,
        InquireError::OperationCanceled | InquireError::OperationInterrupted
    )
}

struct ExportConfig {
    video: Video,
    resolution: ResolutionPreset,
    fps: u32,
    encoding: EncodingMode,
    start_time: f32,
    end_time: f32,
    audio_path: Option<String>,
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

fn prompt_video() -> Option<Video> {
    match Select::new("Video", videos::VIDEOS.to_vec()).prompt() {
        Ok(v) => Some(v),
        Err(e) if prompt_cancelled(&e) => None,
        Err(e) => panic!("{e}"),
    }
}

fn prompt_config(video: Video, duration: f32) -> Option<ExportConfig> {
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

    let resolution = match Select::new("Resolution", resolutions)
        .with_starting_cursor(1)
        .prompt()
    {
        Ok(r) => r,
        Err(e) if prompt_cancelled(&e) => return None,
        Err(e) => panic!("{e}"),
    };

    // 3. Frame rate
    let fps_presets = vec![FpsPreset { fps: 30 }, FpsPreset { fps: 60 }];

    let fps = match Select::new("Frame rate", fps_presets)
        .with_starting_cursor(1)
        .prompt()
    {
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
    let default_audio = video.audio.unwrap_or("");
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
        video,
        resolution,
        fps,
        encoding,
        start_time,
        end_time,
        audio_path,
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
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
    ]);

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

fn main() {
    // Prompt for video selection first, then build scene to get duration.
    let video = match prompt_video() {
        Some(v) => v,
        None => return,
    };

    let (scene, timeline, camera) = (video.build)();
    let duration = timeline.duration();

    let config = match prompt_config(video, duration) {
        Some(c) => c,
        None => return,
    };

    let fps = config.fps;
    let (width, height) = (config.resolution.width, config.resolution.height);

    let start_frame = (config.start_time * fps as f32).floor() as u32;
    let end_time = config.end_time.min(duration);
    let end_frame = (end_time * fps as f32).ceil() as u32;
    let total_frames = end_frame - start_frame;

    if total_frames == 0 {
        println!("Nothing to render.");
        return;
    }

    let conf = Conf {
        window_title: "rs-namin export".to_owned(),
        window_width: width.min(1280) as i32,
        window_height: height.min(720) as i32,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    };

    macroquad::Window::from_config(
        conf,
        export_render(scene, timeline, camera, config, start_frame, end_frame, total_frames),
    );
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
        let video = rs_namin::videos::find("bouncing_ball").unwrap().clone();
        let config = ExportConfig {
            video,
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
        };
        let args = build_ffmpeg_args(&config, "out.mp4");
        assert!(args.contains(&"-crf".to_string()));
        assert!(args.contains(&"18".to_string()));
        assert!(!args.contains(&"-c:a".to_string()));
        assert!(!args.contains(&"-shortest".to_string()));
    }

    #[test]
    fn ffmpeg_args_bitrate_with_audio() {
        let video = rs_namin::videos::find("bouncing_ball").unwrap().clone();
        let config = ExportConfig {
            video,
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

    let rt = render_target_ex(
        width,
        height,
        RenderTargetParams {
            depth: true,
            ..Default::default()
        },
    );
    rt.texture.set_filter(FilterMode::Nearest);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    std::fs::create_dir_all("renders").ok();
    let output_path = format!(
        "renders/{}_{}_{}fps_{}.mp4",
        config.video.name, config.resolution.label, fps, timestamp
    );

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
        ProgressStyle::with_template(
            "{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta} remaining)",
        )
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
        let t = frame as f32 / fps as f32;
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
