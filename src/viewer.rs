use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::camera::orbit::OrbitController;
use crate::clock::{self, Clock};
use crate::debug::{DebugOverlay, SnapView};
use crate::input::{InputProvider, MacroquadInput, UiGatedInput};
use crate::registry::SceneEntry;
use crate::render_util;
use crate::scene::Scene;
use crate::ui::{self, UiRequest};

/// The interactive viewer as an app mode: orbit controls, debug overlay,
/// scrub bar, egui HUD. One scene per instance; opening another scene from
/// the library constructs a fresh `ViewerMode`.
pub struct ViewerMode {
    scene_name: &'static str,
    scene: Scene,
    timeline: Timeline,
    camera: Camera,
    initial_camera: Camera,
    clock: Clock,
    debug_overlay: DebugOverlay,
    orbit: OrbitController,
}

impl ViewerMode {
    /// Build the entry's scene and set up playback. Must be called inside the
    /// macroquad window (scene builders may load textures).
    pub fn new(entry: &'static SceneEntry) -> Self {
        let (scene, timeline, camera) = (entry.build)();

        let mut clock = Clock::new(timeline.duration(), 60.0);
        clock.loop_mode = clock::LoopMode::Loop;
        clock.play();

        Self {
            scene_name: entry.name,
            initial_camera: camera.clone(),
            orbit: OrbitController::from_camera(&camera),
            scene,
            timeline,
            camera,
            clock,
            debug_overlay: DebugOverlay::new(),
        }
    }

    pub fn scene_name(&self) -> &'static str {
        self.scene_name
    }

    /// Run one viewer frame (input, playback, all render passes, UI).
    /// Returns any app-level navigation request.
    pub fn frame(&mut self) -> UiRequest {
        clear_background(BLACK);

        // egui input+layout pass first: it decides whether it captures this
        // frame's pointer/keyboard, and scene input is gated accordingly.
        let (capture, mut request) =
            ui::viewer_layout(&mut self.debug_overlay, &mut self.clock, &self.scene, &self.camera, self.scene_name);
        let raw_input = MacroquadInput;
        let input = UiGatedInput::new(&raw_input, capture.pointer, capture.keyboard);

        let snap = self.debug_overlay.handle_input(&mut self.clock, &input);
        self.debug_overlay.update(&mut self.clock);

        if matches!(request, UiRequest::None) && input.is_key_pressed(KeyCode::Escape) {
            request = UiRequest::OpenLibrary;
        }

        match snap {
            SnapView::Front => self.orbit.snap_front(),
            SnapView::Right => self.orbit.snap_right(),
            SnapView::Top => self.orbit.snap_top(),
            SnapView::None => {}
        }

        self.clock.tick(get_frame_time());

        if self.debug_overlay.camera_follow_timeline {
            self.camera = self.initial_camera.clone();
            self.timeline.apply(self.clock.current_time, &mut self.scene, &mut self.camera);
        } else {
            self.timeline.apply_scene_only(self.clock.current_time, &mut self.scene);
        }

        // 3D scene pass
        set_camera(&self.camera.to_macroquad());
        self.debug_overlay.draw_world(&self.orbit, &self.scene);
        self.scene.draw_world();

        // Screen-space scene pass: design-space coordinates, so screen-space
        // objects match their size/position in exports (WYSIWYG).
        set_camera(&render_util::screen_space_camera(None));
        self.scene.draw_screen();

        // Debug overlay uses real window pixels.
        set_default_camera();
        self.debug_overlay.draw(&self.clock, &self.scene, &self.camera, &input);
        self.debug_overlay.scrub_bar.draw_ticks(&self.timeline, self.clock.duration);

        // egui paint pass — last, so the UI draws on top of everything.
        ui::draw();

        // Orbit controller runs last so it doesn't consume input before UI.
        if self.debug_overlay.camera_follow_timeline {
            self.orbit = OrbitController::from_camera(&self.camera);
        } else {
            self.orbit.update(&mut self.camera, &input);
        }

        self.debug_overlay.record_camera(&self.camera, self.clock.current_time);

        request
    }
}
