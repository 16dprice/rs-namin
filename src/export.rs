use macroquad::prelude::*;

pub struct Exporter {
    output_dir: String,
    /// The actual directory for the current export run (output_dir/timestamp).
    active_dir: String,
    fps: f32,
    width: u32,
    height: u32,
    render_target: RenderTarget,
    current_frame: u32,
    total_frames: u32,
    active: bool,
    /// After rendering to the RT, we need one next_frame() to flush GPU commands
    /// before reading back pixels. This tracks whether there's a frame to save.
    pending_save: bool,
}

impl Exporter {
    pub fn new(width: u32, height: u32, fps: f32, output_dir: &str) -> Self {
        let render_target = render_target(width, height);
        render_target.texture.set_filter(FilterMode::Nearest);

        Self {
            output_dir: output_dir.to_string(),
            active_dir: String::new(),
            fps,
            width,
            height,
            render_target,
            current_frame: 0,
            total_frames: 0,
            active: false,
            pending_save: false,
        }
    }

    pub fn start(&mut self, duration: f32) {
        self.current_frame = 0;
        self.total_frames = (duration * self.fps).ceil() as u32;
        self.active = true;
        self.pending_save = false;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.active_dir = format!("{}/{}", self.output_dir, timestamp);
        std::fs::create_dir_all(&self.active_dir).ok();
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.pending_save = false;
    }

    /// The synthetic time for the current frame being rendered.
    pub fn current_time(&self) -> f32 {
        self.current_frame as f32 / self.fps
    }

    pub fn progress(&self) -> (u32, u32) {
        (self.current_frame, self.total_frames)
    }

    /// Clear the render target before drawing a new frame.
    pub fn clear(&self) {
        let export_cam = Camera3D {
            render_target: Some(self.render_target.clone()),
            ..Default::default()
        };
        set_camera(&export_cam);
        clear_background(BLACK);
    }

    /// Returns a Camera3D that renders to the export render target.
    pub fn export_camera(&self, base: &Camera3D) -> Camera3D {
        Camera3D {
            render_target: Some(self.render_target.clone()),
            ..*base
        }
    }

    /// Save the previously rendered frame from the render target.
    /// Call this AFTER next_frame().await so GPU commands have been flushed.
    /// Returns true if export is complete.
    pub fn save_pending_frame(&mut self) -> bool {
        if !self.pending_save {
            return false;
        }
        self.pending_save = false;

        let image = self.render_target.texture.get_texture_data();
        let path = format!("{}/frame_{:05}.png", self.active_dir, self.current_frame);
        image.export_png(&path);

        self.current_frame += 1;

        if self.current_frame >= self.total_frames {
            self.active = false;
            return true;
        }
        false
    }

    /// Mark that a frame has been rendered to the render target.
    pub fn mark_rendered(&mut self) {
        self.pending_save = true;
    }

    /// Draw the render target as a preview on screen with progress overlay.
    pub fn draw_preview(&self) {
        draw_texture_ex(
            &self.render_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                flip_y: true, // render targets are Y-flipped in OpenGL
                ..Default::default()
            },
        );

        let (current, total) = self.progress();
        let progress_text = format!("Exporting: {}/{} frames  [Esc to cancel]", current, total);
        draw_text(&progress_text, 10.0, 30.0, 20.0, WHITE);

        let bar_w = screen_width() - 20.0;
        let pct = current as f32 / total as f32;
        draw_rectangle(10.0, 40.0, bar_w, 8.0, DARKGRAY);
        draw_rectangle(10.0, 40.0, bar_w * pct, 8.0, GREEN);
    }
}
