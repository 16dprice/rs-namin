//! Export core shared by the CLI `export` binary and the in-app export mode:
//! encoding settings, ffmpeg invocation, and frame math. The in-app
//! [`ExportMode`] renders incrementally — one export frame per UI frame —
//! so the interface stays live with progress and cancel.

use std::io::Write as IoWrite;
use std::process::{Child, Command, Stdio};

use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::doc::{ExportDefaults, SceneDoc};
use crate::input::{InputProvider, MacroquadInput, UiGatedInput};
use crate::registry::{SceneEntry, SceneSource};
use crate::render_util::{DESIGN_HEIGHT, DESIGN_WIDTH, OffscreenRenderer, draw_fitted_texture};
use crate::scene::Scene;
use crate::ui::{self, UiRequest};

#[derive(Clone)]
pub struct ResolutionPreset {
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
}

impl std::fmt::Display for ResolutionPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({}x{})", self.label, self.width, self.height)
    }
}

pub const RESOLUTION_PRESETS: [ResolutionPreset; 4] = [
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

pub fn preset_by_label(label: &str) -> Option<ResolutionPreset> {
    let label = if label.eq_ignore_ascii_case("4k") { "4K" } else { label };
    RESOLUTION_PRESETS.iter().find(|p| p.label == label).cloned()
}

#[derive(Clone)]
pub enum EncodingMode {
    Crf { crf: u32 },
    Bitrate { kbps: u32 },
}

impl std::fmt::Display for EncodingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EncodingMode::Crf { crf } => write!(f, "CRF {crf} (constant quality)"),
            EncodingMode::Bitrate { kbps } => write!(f, "{} Mbps (target bitrate)", kbps / 1000),
        }
    }
}

/// YouTube recommended bitrate (kbps) for given resolution and frame rate.
pub fn recommended_bitrate(label: &str, fps: u32) -> u32 {
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

/// Everything ffmpeg needs to encode a stream of raw RGB frames.
#[derive(Clone)]
pub struct EncodeSettings {
    pub resolution: ResolutionPreset,
    pub fps: u32,
    pub encoding: EncodingMode,
    pub audio_path: Option<String>,
}

pub fn build_ffmpeg_args(settings: &EncodeSettings, output_path: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let width = settings.resolution.width;
    let height = settings.resolution.height;

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
        format!("{}", settings.fps),
        "-i".into(),
        "-".into(),
    ]);

    // Audio input (optional)
    if let Some(ref audio) = settings.audio_path {
        args.extend(["-i".into(), audio.clone()]);
    }

    // Video encoding
    args.extend(["-c:v".into(), "libx264".into(), "-pix_fmt".into(), "yuv420p".into()]);

    match &settings.encoding {
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
    if settings.audio_path.is_some() {
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

/// Inclusive start frame and exclusive end frame for a time range at `fps`.
pub fn frame_range(start_time: f32, end_time: f32, fps: u32) -> (u32, u32) {
    let start_frame = (start_time * fps as f32).floor() as u32;
    let end_frame = (end_time * fps as f32).ceil() as u32;
    (start_frame, end_frame)
}

/// Default output path under renders/ (created if missing).
pub fn timestamped_output_path(scene_name: &str, resolution_label: &str, fps: u32) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::fs::create_dir_all("renders").ok();
    format!("renders/{scene_name}_{resolution_label}_{fps}fps_{timestamp}.mp4")
}

/// The export defaults stored in a Doc entry's scene file, if any.
pub fn doc_export_defaults(entry: &SceneEntry) -> Option<ExportDefaults> {
    match entry.source {
        SceneSource::Doc(path) => SceneDoc::load(path).ok().map(|doc| doc.export),
        SceneSource::Builtin(_) => None,
    }
}

pub fn spawn_ffmpeg(args: &[String]) -> std::io::Result<Child> {
    Command::new("ffmpeg")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

// ---------------------------------------------------------------------------
// In-app export mode
// ---------------------------------------------------------------------------

/// The export form's editable state (Configure phase).
pub struct ExportForm {
    pub resolution_index: usize,
    pub fps: u32,
    pub use_bitrate: bool,
    pub crf: u32,
    pub kbps: u32,
    pub start_time: f32,
    pub end_time: f32,
    pub audio_path: String,
    /// Empty = timestamped path under renders/.
    pub output_path: String,
    /// Message from a previous attempt (spawn failure, cancellation).
    pub notice: Option<String>,
}

impl ExportForm {
    fn new(entry: &SceneEntry, duration: f32) -> Self {
        let mut form = Self {
            resolution_index: 1, // 1080p
            fps: 60,
            use_bitrate: false,
            crf: 18,
            kbps: recommended_bitrate("1080p", 60),
            start_time: 0.0,
            end_time: duration,
            audio_path: entry.audio.unwrap_or("").to_string(),
            output_path: String::new(),
            notice: None,
        };
        // Scene documents can carry export defaults.
        if let Some(defaults) = doc_export_defaults(entry) {
            if let Some(index) = defaults
                .resolution
                .as_deref()
                .and_then(|label| RESOLUTION_PRESETS.iter().position(|p| p.label == label))
            {
                form.resolution_index = index;
            }
            if let Some(fps) = defaults.fps {
                form.fps = fps;
            }
            if let Some(output) = defaults.output {
                form.output_path = output;
            }
        }
        form
    }

    pub fn resolution(&self) -> &ResolutionPreset {
        &RESOLUTION_PRESETS[self.resolution_index]
    }

    fn settings(&self) -> EncodeSettings {
        let audio = self.audio_path.trim();
        let audio_path = if audio.is_empty() || !std::path::Path::new(audio).exists() {
            None
        } else {
            Some(audio.to_string())
        };
        EncodeSettings {
            resolution: self.resolution().clone(),
            fps: self.fps,
            encoding: if self.use_bitrate {
                EncodingMode::Bitrate { kbps: self.kbps }
            } else {
                EncodingMode::Crf { crf: self.crf }
            },
            audio_path,
        }
    }
}

/// A running incremental export: one frame rendered per UI frame, read back
/// and piped to ffmpeg on the following frame (draw calls flush on
/// next_frame).
pub struct RenderJob {
    renderer: OffscreenRenderer,
    ffmpeg: Child,
    rgb_buf: Vec<u8>,
    fps: u32,
    next_frame: u32,
    end_frame: u32,
    frames_done: u32,
    total_frames: u32,
    pending_readback: bool,
    output_path: String,
}

impl RenderJob {
    pub fn progress(&self) -> f32 {
        if self.total_frames == 0 {
            return 1.0;
        }
        self.frames_done as f32 / self.total_frames as f32
    }

    pub fn frames_done(&self) -> u32 {
        self.frames_done
    }

    pub fn total_frames(&self) -> u32 {
        self.total_frames
    }

    pub fn output_path(&self) -> &str {
        &self.output_path
    }
}

pub struct DoneInfo {
    pub success: bool,
    pub message: String,
    pub output_path: String,
}

pub enum ExportPhase {
    Configure(ExportForm),
    Render(RenderJob),
    Done(DoneInfo),
}

/// What the export UI asked for this frame (see `ui::export_layout`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportUiEvent {
    None,
    /// Leave export mode, back to the viewer.
    Back,
    Start,
    Cancel,
    /// Write the form's resolution/fps/output into the scene document.
    SaveDefaults,
    /// From the Done screen: return to the form.
    ExportAgain,
}

/// The in-app export screen as an app mode. Owns its own copy of the scene
/// (rebuilt from the entry) so viewer state never leaks into a render.
pub struct ExportMode {
    entry: &'static SceneEntry,
    scene: Scene,
    timeline: Timeline,
    initial_camera: Camera,
    camera: Camera,
    duration: f32,
    pub phase: ExportPhase,
    /// Live preview target drawn behind the UI (Configure/Done phases).
    preview: OffscreenRenderer,
}

impl ExportMode {
    /// Must be called inside the macroquad window (scene builders may load
    /// textures).
    pub fn new(entry: &'static SceneEntry) -> Self {
        let (scene, timeline, camera) = entry.build_or_error_scene();
        let duration = timeline.duration();
        Self {
            entry,
            initial_camera: camera.clone(),
            camera,
            scene,
            timeline,
            duration,
            phase: ExportPhase::Configure(ExportForm::new(entry, duration)),
            preview: OffscreenRenderer::new(DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32),
        }
    }

    pub fn entry(&self) -> &'static SceneEntry {
        self.entry
    }

    pub fn scene_name(&self) -> &'static str {
        self.entry.name
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Run one export-mode frame (UI, state machine step, preview draw).
    /// Returns any app-level navigation request.
    pub fn frame(&mut self) -> UiRequest {
        clear_background(BLACK);

        let is_doc = matches!(self.entry.source, SceneSource::Doc(_));
        let (capture, event) = ui::export_layout(&mut self.phase, self.entry.name, self.duration, is_doc);

        let mut request = UiRequest::None;
        if event == ExportUiEvent::Back {
            request = UiRequest::OpenScene(self.entry);
        }
        // Esc goes back to the viewer, but never mid-render (Cancel first).
        let raw_input = MacroquadInput;
        let input = UiGatedInput::new(&raw_input, capture.pointer, capture.keyboard);
        if matches!(request, UiRequest::None) && !self.is_rendering() && input.is_key_pressed(KeyCode::Escape) {
            request = UiRequest::OpenScene(self.entry);
        }

        self.step(event);

        draw_fitted_texture(self.preview_texture());
        ui::draw();

        request
    }

    /// Advance the export state machine by one UI frame and render the
    /// preview/export frame. Returns true while a render job is active
    /// (used to keep Escape from leaving mid-render).
    pub fn step(&mut self, event: ExportUiEvent) {
        match event {
            ExportUiEvent::Start => self.start_render(),
            ExportUiEvent::Cancel => self.cancel_render(),
            ExportUiEvent::SaveDefaults => self.save_defaults(),
            ExportUiEvent::ExportAgain => {
                self.phase = ExportPhase::Configure(ExportForm::new(self.entry, self.duration));
            }
            ExportUiEvent::None | ExportUiEvent::Back => {}
        }

        match &mut self.phase {
            ExportPhase::Configure(form) => {
                // Live preview of the export start point.
                let t = form.start_time.clamp(0.0, self.duration);
                self.camera = self.initial_camera.clone();
                self.timeline.apply(t, &mut self.scene, &mut self.camera);
                self.preview.render_frame(&self.scene, &self.camera);
                set_default_camera();
            }
            ExportPhase::Render(_) => self.step_render(),
            ExportPhase::Done(_) => {}
        }
    }

    /// The texture to draw as the full-window preview this frame.
    pub fn preview_texture(&self) -> &Texture2D {
        match &self.phase {
            ExportPhase::Render(job) => &job.renderer.target.texture,
            _ => &self.preview.target.texture,
        }
    }

    pub fn is_rendering(&self) -> bool {
        matches!(self.phase, ExportPhase::Render(_))
    }

    /// Persist the Configure form's resolution/fps/output as the scene
    /// document's export defaults.
    fn save_defaults(&mut self) {
        let ExportPhase::Configure(form) = &mut self.phase else {
            return;
        };
        let SceneSource::Doc(path) = self.entry.source else {
            return;
        };
        let result = SceneDoc::load(path).and_then(|mut doc| {
            doc.export = ExportDefaults {
                resolution: Some(form.resolution().label.to_string()),
                fps: Some(form.fps),
                output: (!form.output_path.trim().is_empty()).then(|| form.output_path.trim().to_string()),
            };
            let ron = doc.to_ron_string()?;
            std::fs::write(path, ron).map_err(|e| format!("cannot write {path}: {e}"))
        });
        form.notice = Some(match result {
            Ok(()) => "Saved as scene export defaults.".to_string(),
            Err(error) => format!("Saving defaults failed: {error}"),
        });
    }

    fn start_render(&mut self) {
        let ExportPhase::Configure(form) = &mut self.phase else {
            return;
        };

        form.end_time = form.end_time.clamp(0.0, self.duration);
        form.start_time = form.start_time.clamp(0.0, form.end_time);
        let settings = form.settings();
        let (start_frame, end_frame) = frame_range(form.start_time, form.end_time, settings.fps);
        if end_frame <= start_frame {
            form.notice = Some("Nothing to render: empty time range.".to_string());
            return;
        }

        let output_path = if form.output_path.trim().is_empty() {
            timestamped_output_path(self.entry.name, settings.resolution.label, settings.fps)
        } else {
            let path = form.output_path.trim().to_string();
            if let Some(parent) = std::path::Path::new(&path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).ok();
            }
            path
        };

        let args = build_ffmpeg_args(&settings, &output_path);
        let ffmpeg = match spawn_ffmpeg(&args) {
            Ok(child) => child,
            Err(e) => {
                form.notice = Some(format!("Failed to start ffmpeg: {e}. Is it installed?"));
                return;
            }
        };

        let (width, height) = (settings.resolution.width, settings.resolution.height);
        self.phase = ExportPhase::Render(RenderJob {
            renderer: OffscreenRenderer::new(width, height),
            ffmpeg,
            rgb_buf: Vec::with_capacity((width * height * 3) as usize),
            fps: settings.fps,
            next_frame: start_frame,
            end_frame,
            frames_done: 0,
            total_frames: end_frame - start_frame,
            pending_readback: false,
            output_path,
        });
    }

    fn step_render(&mut self) {
        let ExportPhase::Render(job) = &mut self.phase else {
            return;
        };

        // 1. Read back the frame rendered last UI frame and pipe it to ffmpeg.
        if job.pending_readback {
            job.renderer.read_rgb(&mut job.rgb_buf);
            job.pending_readback = false;
            let ok = match job.ffmpeg.stdin.as_mut() {
                Some(stdin) => stdin.write_all(&job.rgb_buf).is_ok(),
                None => false,
            };
            if !ok {
                let output_path = job.output_path.clone();
                let _ = job.ffmpeg.kill();
                let _ = job.ffmpeg.wait();
                self.phase = ExportPhase::Done(DoneInfo {
                    success: false,
                    message: "ffmpeg pipe broke mid-render.".to_string(),
                    output_path,
                });
                return;
            }
            job.frames_done += 1;
        }

        // 2. Render the next export frame, or finish.
        if job.next_frame < job.end_frame {
            let t = job.next_frame as f32 / job.fps as f32;
            self.camera = self.initial_camera.clone();
            self.timeline.apply(t, &mut self.scene, &mut self.camera);
            job.renderer.render_frame(&self.scene, &self.camera);
            set_default_camera();
            job.pending_readback = true;
            job.next_frame += 1;
        } else {
            // All frames piped: close stdin and wait for ffmpeg to finalize.
            drop(job.ffmpeg.stdin.take());
            let status = job.ffmpeg.wait();
            let success = status.as_ref().map(|s| s.success()).unwrap_or(false);
            let output_path = job.output_path.clone();
            let message = if success {
                "Export complete.".to_string()
            } else {
                format!("ffmpeg exited with {status:?}.")
            };
            self.phase = ExportPhase::Done(DoneInfo {
                success,
                message,
                output_path,
            });
        }
    }

    fn cancel_render(&mut self) {
        let ExportPhase::Render(job) = &mut self.phase else {
            return;
        };
        let _ = job.ffmpeg.kill();
        let _ = job.ffmpeg.wait();
        std::fs::remove_file(&job.output_path).ok();

        let mut form = ExportForm::new(self.entry, self.duration);
        form.notice = Some("Export cancelled.".to_string());
        self.phase = ExportPhase::Configure(form);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(encoding: EncodingMode, audio: Option<&str>) -> EncodeSettings {
        EncodeSettings {
            resolution: preset_by_label("1080p").unwrap(),
            fps: 60,
            encoding,
            audio_path: audio.map(String::from),
        }
    }

    #[test]
    fn ffmpeg_args_crf_no_audio() {
        let args = build_ffmpeg_args(&settings(EncodingMode::Crf { crf: 18 }, None), "out.mp4");
        assert!(args.contains(&"-crf".to_string()));
        assert!(args.contains(&"18".to_string()));
        assert!(args.contains(&"1920x1080".to_string()));
        assert!(!args.contains(&"-c:a".to_string()));
        assert!(!args.contains(&"-shortest".to_string()));
        assert_eq!(args.last().unwrap(), "out.mp4");
    }

    #[test]
    fn ffmpeg_args_bitrate_with_audio() {
        let args = build_ffmpeg_args(&settings(EncodingMode::Bitrate { kbps: 16_000 }, Some("/tmp/test.mp3")), "out.mp4");
        assert!(args.contains(&"-b:v".to_string()));
        assert!(args.contains(&"16000k".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"384k".to_string()));
        assert!(args.contains(&"48000".to_string()));
        assert!(args.contains(&"-shortest".to_string()));
        assert!(args.contains(&"/tmp/test.mp3".to_string()));
    }

    #[test]
    fn frame_range_whole_duration() {
        assert_eq!(frame_range(0.0, 1.0, 60), (0, 60));
    }

    #[test]
    fn frame_range_fractional_times() {
        assert_eq!(frame_range(0.5, 2.5, 60), (30, 150));
    }

    #[test]
    fn frame_range_non_aligned_end() {
        let (_, end) = frame_range(0.0, 1.01, 60);
        assert_eq!(end, 61);
    }

    #[test]
    fn frame_range_30fps() {
        let (start, end) = frame_range(0.0, 2.0, 30);
        assert_eq!(end - start, 60);
    }

    #[test]
    fn recommended_bitrate_youtube_specs() {
        assert_eq!(recommended_bitrate("1080p", 30), 8_000);
        assert_eq!(recommended_bitrate("1080p", 60), 16_000);
        assert_eq!(recommended_bitrate("4K", 30), 35_000);
        assert_eq!(recommended_bitrate("4K", 60), 53_000);
    }

    #[test]
    fn export_form_prefills_from_doc_defaults() {
        let path = "/tmp/rs_namin_export_defaults_test.ron";
        std::fs::write(
            path,
            r#"(
    description: "t",
    objects: [],
    export: (resolution: Some("4K"), fps: Some(30), output: Some("renders/custom.mp4")),
)"#,
        )
        .unwrap();
        let entry = SceneEntry {
            name: "t",
            description: "t",
            kind: crate::registry::SceneKind::Doc,
            source: SceneSource::Doc(path),
            audio: None,
        };
        let form = ExportForm::new(&entry, 5.0);
        assert_eq!(form.resolution().label, "4K");
        assert_eq!(form.fps, 30);
        assert_eq!(form.output_path, "renders/custom.mp4");

        let defaults = doc_export_defaults(&entry).unwrap();
        assert_eq!(defaults.fps, Some(30));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn export_form_defaults_without_doc_block() {
        let path = "/tmp/rs_namin_export_nodefaults_test.ron";
        std::fs::write(path, "(description: \"t\", objects: [])").unwrap();
        let entry = SceneEntry {
            name: "t",
            description: "t",
            kind: crate::registry::SceneKind::Doc,
            source: SceneSource::Doc(path),
            audio: None,
        };
        let form = ExportForm::new(&entry, 5.0);
        assert_eq!(form.resolution().label, "1080p");
        assert_eq!(form.fps, 60);
        assert!(form.output_path.is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn preset_lookup_accepts_lowercase_4k() {
        assert_eq!(preset_by_label("4k").unwrap().width, 3840);
        assert!(preset_by_label("999p").is_none());
    }
}
