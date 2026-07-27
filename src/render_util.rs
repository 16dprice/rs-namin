use macroquad::prelude::*;
use macroquad::texture::{RenderTarget, RenderTargetParams, render_target_ex};

use crate::camera::Camera;
use crate::scene::Scene;

/// The design resolution for screen-space objects. Screen-space coordinates
/// (e.g. `Text` position and font_size, in pixels) are interpreted against
/// this canvas and scaled to the actual output, so screen-space content
/// occupies the same fraction of the frame in the viewer and at every export
/// resolution. Non-16:9 outputs stretch the canvas non-uniformly.
pub const DESIGN_WIDTH: f32 = 1280.0;
pub const DESIGN_HEIGHT: f32 = 720.0;

/// Offscreen two-pass renderer shared by the export and snapshot binaries:
/// a depth-buffered render target, the 3D world pass, the design-space
/// screen pass, and pixel readback.
pub struct OffscreenRenderer {
    pub target: RenderTarget,
    pub width: u32,
    pub height: u32,
}

impl OffscreenRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        // Render targets have no depth buffer unless requested; a 3D scene
        // without one z-fights (see docs/camera_and_rendering.md).
        let target = render_target_ex(
            width,
            height,
            RenderTargetParams {
                depth: true,
                ..Default::default()
            },
        );
        target.texture.set_filter(FilterMode::Nearest);
        Self { target, width, height }
    }

    /// Render one frame into the target: 3D world pass, then design-space
    /// screen pass. The result is available to `read_*` only after the
    /// following `next_frame().await` flushes the draw calls.
    pub fn render_frame(&self, scene: &Scene, camera: &Camera) {
        let mut cam3d = camera.to_macroquad();
        cam3d.render_target = Some(self.target.clone());
        cam3d.viewport = Some((0, 0, self.width as i32, self.height as i32));
        set_camera(&cam3d);
        clear_background(BLACK);
        scene.draw_world();

        set_camera(&screen_space_camera(Some(self.target.clone())));
        scene.draw_screen();
    }

    /// Read back the most recently flushed frame as top-to-bottom RGBA rows.
    pub fn read_rgba(&self, out: &mut Vec<u8>) {
        let image = self.target.texture.get_texture_data();
        rgba_flipped(image.get_image_data(), self.width as usize, self.height as usize, out);
    }

    /// Read back the most recently flushed frame as top-to-bottom RGB rows.
    pub fn read_rgb(&self, out: &mut Vec<u8>) {
        let image = self.target.texture.get_texture_data();
        rgba_to_rgb_flipped(image.get_image_data(), self.width as usize, self.height as usize, out);
    }
}

/// Camera for the screen-space pass, mapping the design canvas onto the full
/// output with origin top-left and Y down (matching macroquad's default
/// screen coordinates). Pass a render target for offscreen rendering or
/// `None` to draw to the window (viewer).
///
/// The Y zoom sign depends on the destination: render targets are stored
/// bottom-up (OpenGL convention, compensated by the flipped readback in
/// `rgba_flipped`/`rgba_to_rgb_flipped`), while the default framebuffer is
/// presented directly — a negative Y there renders everything upside-down at
/// the bottom of the window.
pub fn screen_space_camera(render_target: Option<RenderTarget>) -> Camera2D {
    let y_sign = if render_target.is_some() { -1.0 } else { 1.0 };
    Camera2D {
        zoom: vec2(2.0 / DESIGN_WIDTH, y_sign * 2.0 / DESIGN_HEIGHT),
        target: vec2(DESIGN_WIDTH / 2.0, DESIGN_HEIGHT / 2.0),
        render_target,
        ..Default::default()
    }
}

/// Convert RGBA pixel data to RGB with Y-flip (OpenGL render targets are upside-down).
pub fn rgba_to_rgb_flipped(rgba: &[[u8; 4]], width: usize, height: usize, out: &mut Vec<u8>) {
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

/// Convert RGBA pixel data to RGBA with Y-flip (OpenGL render targets are upside-down).
pub fn rgba_flipped(rgba: &[[u8; 4]], width: usize, height: usize, out: &mut Vec<u8>) {
    out.clear();
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba[y * width + x];
            out.push(pixel[0]);
            out.push(pixel[1]);
            out.push(pixel[2]);
            out.push(pixel[3]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_flipped_strips_alpha_and_flips() {
        let rgba: Vec<[u8; 4]> = vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 128]];
        let mut out = Vec::new();
        rgba_to_rgb_flipped(&rgba, 2, 2, &mut out);
        assert_eq!(out, vec![0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0,]);
    }

    #[test]
    fn rgba_flipped_preserves_alpha_and_flips() {
        let rgba: Vec<[u8; 4]> = vec![[255, 0, 0, 255], [0, 255, 0, 128], [0, 0, 255, 64], [255, 255, 255, 0]];
        let mut out = Vec::new();
        rgba_flipped(&rgba, 2, 2, &mut out);
        assert_eq!(out, vec![0, 0, 255, 64, 255, 255, 255, 0, 255, 0, 0, 255, 0, 255, 0, 128,]);
    }

    #[test]
    fn single_pixel() {
        let rgba: Vec<[u8; 4]> = vec![[42, 99, 200, 128]];
        let mut out = Vec::new();
        rgba_to_rgb_flipped(&rgba, 1, 1, &mut out);
        assert_eq!(out, vec![42, 99, 200]);
    }

    #[test]
    fn reuses_buffer() {
        let rgba: Vec<[u8; 4]> = vec![[1, 2, 3, 4]];
        let mut out = vec![99, 99, 99, 99, 99];
        rgba_to_rgb_flipped(&rgba, 1, 1, &mut out);
        assert_eq!(out, vec![1, 2, 3]);
    }

    // Regression: with a negative Y zoom the default-framebuffer screen pass
    // renders upside-down at the bottom of the window (render targets get
    // away with it because their readback flips). Can't construct a
    // RenderTarget headless, so only the window path is pinned here.
    #[test]
    fn screen_space_camera_window_path_is_y_down() {
        let cam = screen_space_camera(None);
        assert!(cam.zoom.y > 0.0);
        assert!(cam.zoom.x > 0.0);
    }
}
