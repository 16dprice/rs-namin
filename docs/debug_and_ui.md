# Debug Overlay & UI

See `src/debug/` for implementation. All keybindings are configurable via `Keybindings` struct in `src/debug/keybindings.rs`.

Toggle keys: F1 (HUD), F2 (scrub bar), F3 (value inspector), F4 (world-space grid/axes), F5 (camera mode).

## Gotchas

- **Scrub bar auto-pauses on drag** and resumes on release if playback was active. This state machine lives in `ScrubBar` — see `src/debug/scrub_bar.rs`.
- **Orbit controller runs last in the frame** so it doesn't consume mouse input before UI elements like scrub bar dragging.

## Not Yet Implemented

- Orbit-target crosshair — visual indicator of the orbit controller's target point.
- Per-object bounding boxes — wireframe boxes around scene objects.
- Snap-to-view keys — Blender-style numpad views.
- Camera state log — ring buffer of camera snapshots.
