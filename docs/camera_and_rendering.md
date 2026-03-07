# Camera & Rendering

## Camera Struct

Implemented in `src/camera/mod.rs`. Fields:

- **Position** (`Vec3`) — where the camera is in world space.
- **Target** (`Vec3`) — the look-at point.
- **Up** (`Vec3`) — the up vector (default: Y-up).
- **FOV** (`f32`) — field of view in degrees (converted to radians for macroquad).
- **Near / Far** (`f32`) — clipping planes.
- **Projection** — `ProjectionMode::Perspective` or `ProjectionMode::Orthographic`.

### Conversion

`camera.to_macroquad() -> Camera3D` converts to macroquad's camera type in one place, handling FOV degree-to-radian conversion and mapping `ProjectionMode` to macroquad's `Projection` enum (note: macroquad spells it `Orthographics`).

### Animatable

The camera implements `Animatable` with properties: `position`, `target`, `up`, `fov`, `near`, `far`. These can be keyframed on the timeline.

### Helpers

- `forward()` — normalized direction from position to target.
- `distance()` — distance from position to target.

## Orbit Controller

Implemented in `src/camera/orbit.rs`. Uses spherical coordinates (azimuth, elevation, distance) around a target point.

- **Middle-click drag**: orbit (rotate around target)
- **Right-click drag**: pan (move target and camera). Pan speed is derived from the camera's FOV and distance for 1:1 mouse tracking.
- **Scroll wheel**: zoom (change distance, clamped to min/max)
- **WASD**: move target along the ground plane (camera follows)
- **Q/E**: move target down/up along the Y axis
- `from_camera(&Camera)` — derive orbit state from an existing camera.
- `apply_to_camera(&Camera)` — write spherical position back to camera.

Configurable: `orbit_speed`, `zoom_speed`, `move_speed`, `min_distance`, `max_distance`.

### Mouse delta coordinate space

`mouse_delta_position()` returns coordinates in the -2..2 range (internally `pixel / screen * 2 - 1`). The orbit controller multiplies by `screen / 2` to recover pixel deltas.

## Runtime Modes

Currently only interactive mode is implemented. The main loop runs:
1. `set_camera(&camera.to_macroquad())` — 3D scene pass
2. `scene.draw_world()` — world-space objects draw with custom meshes
3. `set_default_camera()` — switch to screen space
4. `scene.draw_screen()` — screen-space objects (e.g., Text) draw with pixel coords
5. `orbit.update(&mut camera)` — process mouse input

### Playback Mode (not yet implemented)

Camera would be fully driven by the timeline — no manual input. The main loop would skip `orbit.update()` when in playback mode.

## Scene Objects

Objects are flat custom meshes rendered on the XY plane using `draw_mesh`. See `docs/scene_and_properties.md` for the rendering pattern.

Coordinates are in world space (Y-up).

## CLI Export

Implemented in `src/bin/export.rs`. Run with `cargo run --bin export`.

- Opens a macroquad window with **vsync disabled** (`swap_interval: Some(0)`) for max speed.
- Drives time synthetically at `1/fps` steps.
- Renders each frame to a `render_target`, flushes via `next_frame()`, then reads back pixels.
- Pipes raw RGB directly to ffmpeg's stdin — no intermediate files.
- Outputs to `export_frames/<timestamp>.mp4`.

### Why a separate binary?

macroquad batches GPU draw commands and only flushes on `next_frame().await`. This means readback (`get_texture_data`) requires a flush between render and read. An in-app export was limited to display framerate. The separate binary disables vsync so `next_frame()` returns immediately after flushing, running as fast as the GPU allows.

## Module Location

```
src/camera/
  mod.rs        Camera struct, to_macroquad(), Animatable impl, helpers
  orbit.rs      OrbitController (spherical coords, mouse input)
src/bin/
  export.rs     CLI export binary
src/demo.rs     Demo scene definition (bouncing ball, shapes, text)
src/my_scene.rs User scene definition (active scene used by main and export)
```
