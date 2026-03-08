# Debug Overlay & UI

See `src/debug/` for implementation. All keybindings are configurable via `Keybindings` struct in `src/debug/keybindings.rs`.

Toggle keys: F1 (HUD), F2 (scrub bar), F3 (value inspector), F4 (world-space grid/axes/crosshair), F5 (camera mode).

Snap-to-view: Numpad 1 (front), Numpad 3 (right), Numpad 7 (top). Sets the orbit controller to a standard view while preserving target and distance. See `OrbitController::snap_front/snap_right/snap_top` in `src/camera/orbit.rs`.

## World-Space Helpers (F4)

- Grid and origin axes (XYZ colored lines)
- Orbit-target crosshair — yellow 3-axis cross at the orbit controller's target point, scales with camera distance
- Per-object bounding boxes — wireframe AABBs around world-space objects (controlled by `bounding_boxes_visible` flag, off by default)

## Camera State Log

Ring buffer (256 entries) recording camera position/target changes over time. Deduplicates consecutive identical states. See `src/debug/camera_log.rs`.

## Gotchas

- **Scrub bar auto-pauses on drag** and resumes on release if playback was active. This state machine lives in `ScrubBar` — see `src/debug/scrub_bar.rs`.
- **Orbit controller runs last in the frame** so it doesn't consume mouse input before UI elements like scrub bar dragging.
- **`handle_input` returns `SnapView`** — the caller must apply snap-to-view to the orbit controller. See `src/main.rs` for the pattern.
