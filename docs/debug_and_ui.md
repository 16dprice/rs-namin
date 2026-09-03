# Debug Overlay & UI

See `src/debug/` for the overlay flags/keybindings and `src/ui.rs` for the egui chrome (app bar, transport bar, HUD, value inspector). All keybindings are configurable via the `Keybindings` struct in `src/debug/keybindings.rs`.

Toggle keys: F1 (camera HUD window), F2 (transport bar), F3 (value inspector), F4 (world-space grid/axes/crosshair), F5 (camera mode), F6 (mouse world coords). The visibility flags live on `DebugOverlay`; the widgets themselves are egui, except F4/F6 which draw with macroquad in the viewport.

The transport bar holds play/pause, frame stepping, loop mode, speed, and a full-width scrub slider with keyframe ticks painted from the timeline. The app bar's Snapshot button renders the scene (no UI) through `OffscreenRenderer` into `snapshots/` — readback happens one frame later because draw calls only flush on `next_frame`.

Preview mode (P key or the app bar's Preview button) hides every panel and overlay and plays the scene back exactly as export renders it: the document camera driven by camera tracks, rendered through `OffscreenRenderer` and letterboxed to the window at the 16:9 design aspect. An animated rainbow ring outlines the video frame — the scene background is black like the window, so the frame edge would otherwise be invisible. The orbit camera is untouched, so Esc (or P) returns to the editing view as it was. Transport keys (space, arrows) stay live; the panel-toggle and snap keys deliberately don't. See `ViewerMode::preview_frame` in `src/viewer.rs`.

Snap-to-view: Numpad 1 (front), Numpad 3 (right), Numpad 7 (top). Sets the orbit controller to a standard view while preserving target and distance. See `OrbitController::snap_front/snap_right/snap_top` in `src/camera/orbit.rs`.

## World-Space Helpers (F4)

- Grid and origin axes (XYZ colored lines)
- Orbit-target crosshair — yellow 3-axis cross at the orbit controller's target point, scales with camera distance
- Per-object bounding boxes — wireframe AABBs around world-space objects (controlled by `bounding_boxes_visible` flag, off by default)
- Mouse world coords (F6) — shows XY world coordinates (raycasted onto Z=0 plane) near the cursor. See `draw_mouse_coords` in `src/debug/mod.rs`. The projection matrix used for the unprojection branches on `camera.projection`: perspective builds it from `fovy`/aspect/near/far as usual, but orthographic reinterprets `fovy` as a vertical world-unit extent to build an orthographic frustum — reusing the perspective math here would misplace the readout whenever the camera is in orthographic mode.

## Camera State Log

Ring buffer (256 entries) recording camera position/target changes over time. Deduplicates consecutive identical states. See `src/debug/camera_log.rs`.

## Gotchas

- **Scrubbing auto-pauses while dragging** and resumes on release only if playback was active when the drag started. The logic is the pure `apply_scrub` function in `src/ui.rs` (kept egui-free so it stays unit-testable).
- **Orbit controller runs last in the frame** so it doesn't consume mouse input before the UI; egui input capture is handled separately via `UiGatedInput`.
- **`handle_input` returns `SnapView`** — the caller must apply snap-to-view to the orbit controller. See `ViewerMode::frame` in `src/viewer.rs` for the pattern.
