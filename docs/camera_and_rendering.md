# Camera & Rendering

See `src/camera/mod.rs` for the Camera struct and `src/camera/orbit.rs` for the orbit controller.

## Gotchas

- **FOV is stored in degrees** but macroquad's `Camera3D` expects radians. `to_macroquad()` handles the conversion — don't convert manually.
- **macroquad spells it `Orthographics`** (not Orthographic). `to_macroquad()` handles this mapping via `ProjectionMode`.
- **`mouse_delta_position()` returns coords in the -2..2 range** (internally `pixel / screen * 2 - 1`). The orbit controller multiplies by `screen / 2` to recover pixel deltas.
- **Orbit pan scale** is derived from FOV and distance for 1:1 mouse tracking: `2 * distance * tan(fov/2) / screen_height`.

## Timeline Camera Mode

Toggled with F5 (`camera_follow_timeline` flag on `DebugOverlay`). When active, the camera resets to its initial state each frame and is fully driven by timeline tracks — orbit input is ignored. When toggled off, orbit mode resumes and `apply_scene_only()` is used so camera tracks don't conflict with manual control. See `src/main.rs`.

## CLI Export

Run with `cargo run --bin export`. See `src/bin/export.rs`.

### Why a separate binary?

macroquad batches GPU draw commands and only flushes on `next_frame().await`. This means readback (`get_texture_data`) requires a flush between render and read. An in-app export was limited to display framerate. The separate binary disables vsync so `next_frame()` returns immediately after flushing, running as fast as the GPU allows.
