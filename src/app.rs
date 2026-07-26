//! Application shell: per-frame mode dispatch and mode switching.
//!
//! `run` is the single window loop for the GUI application. Each mode renders
//! one frame and returns a [`UiRequest`]; the shell applies transitions.
//! See docs/gui_plan.md (Phase 1).

use macroquad::prelude::*;

use crate::doc;
use crate::export::ExportMode;
use crate::registry::{self, SceneEntry};
use crate::ui::{self, UiRequest};
use crate::viewer::ViewerMode;

pub enum AppMode {
    /// Scene library: pick any registered scene to open in the viewer.
    Library,
    Viewer(Box<ViewerMode>),
    Export(Box<ExportMode>),
}

impl AppMode {
    /// Build `entry`'s scene and enter the viewer on it. Must be called
    /// inside the macroquad window (scene builders may load textures).
    pub fn viewer(entry: &'static SceneEntry) -> Self {
        AppMode::Viewer(Box::new(ViewerMode::new(entry)))
    }

    /// Enter the export screen for `entry`. Must be called inside the
    /// macroquad window (builds the scene and a render target).
    pub fn export(entry: &'static SceneEntry) -> Self {
        AppMode::Export(Box::new(ExportMode::new(entry)))
    }
}

/// Apply a mode transition requested by this frame's UI.
fn apply_request(mode: &mut AppMode, request: UiRequest) {
    match request {
        UiRequest::OpenScene(entry) => *mode = AppMode::viewer(entry),
        UiRequest::OpenExport(entry) => *mode = AppMode::export(entry),
        UiRequest::OpenLibrary => {
            // Pick up new or edited documents (and their descriptions).
            registry::rescan();
            *mode = AppMode::Library;
        }
        UiRequest::NewScene => match doc::create_untitled() {
            Ok(name) => {
                registry::rescan();
                if let Some(entry) = registry::find(&name) {
                    *mode = AppMode::viewer(entry);
                }
            }
            Err(error) => eprintln!("Failed to create scene document: {error}"),
        },
        UiRequest::None => {}
    }
}

/// Parse `RS_NAMIN_FRAME_DUMP="path.png@N"`: capture frame N to path, then exit.
/// Dev/agent utility — the only way to visually verify the live app
/// (including UI chrome) without a human at the window.
fn frame_dump_spec() -> Option<(String, u32)> {
    let spec = std::env::var("RS_NAMIN_FRAME_DUMP").ok()?;
    let (path, frame) = spec.rsplit_once('@')?;
    Some((path.to_string(), frame.parse().ok()?))
}

/// The application loop. Call from an async macroquad context.
pub async fn run(mut mode: AppMode) {
    let frame_dump = frame_dump_spec();
    let mut frame_index: u32 = 0;

    loop {
        let request = match &mut mode {
            AppMode::Library => library_frame(),
            AppMode::Viewer(viewer) => viewer.frame(),
            AppMode::Export(export) => export.frame(),
        };
        apply_request(&mut mode, request);

        if let Some((path, target_frame)) = &frame_dump {
            if frame_index == *target_frame {
                get_screen_data().export_png(path);
                eprintln!("Frame dump saved: {path}");
                return;
            }
            frame_index += 1;
        }

        next_frame().await;
    }
}

fn library_frame() -> UiRequest {
    clear_background(BLACK);
    let (_capture, request) = ui::library_layout();
    ui::draw();
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    // Uses the torus example: its builder is GL-free (no texture loads), so
    // ViewerMode can be constructed headless in tests.
    fn gl_free_entry() -> &'static SceneEntry {
        registry::find("torus").unwrap()
    }

    #[test]
    fn open_scene_enters_viewer() {
        let mut mode = AppMode::Library;
        apply_request(&mut mode, UiRequest::OpenScene(gl_free_entry()));
        match &mode {
            AppMode::Viewer(v) => assert_eq!(v.scene_name(), "torus"),
            _ => panic!("expected viewer mode"),
        }
    }

    #[test]
    fn open_library_leaves_viewer() {
        let mut mode = AppMode::viewer(gl_free_entry());
        apply_request(&mut mode, UiRequest::OpenLibrary);
        assert!(matches!(mode, AppMode::Library));
    }

    #[test]
    fn none_request_keeps_mode() {
        let mut mode = AppMode::Library;
        apply_request(&mut mode, UiRequest::None);
        assert!(matches!(mode, AppMode::Library));
    }
}
