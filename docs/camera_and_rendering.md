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

- **Right-click drag**: orbit (rotate around target)
- **Middle-click drag**: pan (move target and camera)
- **Scroll wheel**: zoom (change distance, clamped to min/max)
- `from_camera(&Camera)` — derive orbit state from an existing camera.
- `apply_to_camera(&Camera)` — write spherical position back to camera.

Configurable: `orbit_speed`, `zoom_speed`, `min_distance`, `max_distance`. Pan speed is derived from the camera's FOV and screen height for 1:1 mouse tracking.

## Runtime Modes

Currently only interactive mode is implemented. The main loop runs:
1. `set_camera(&camera.to_macroquad())` — 3D scene pass
2. `scene.draw_all()` — objects draw with 3D primitives
3. `set_default_camera()` — switch to screen space for UI
4. `orbit.update(&mut camera)` — process mouse input

### Playback Mode (not yet implemented)

Camera would be fully driven by the timeline — no manual input. The main loop would skip `orbit.update()` when in playback mode.

## Scene Objects

Objects use macroquad's 3D primitives:
- `Circle` draws with `draw_sphere`
- `Line` draws with `draw_line_3d`

Coordinates are in world space (Y-up).

## Offline Export Pipeline (not yet implemented)

Planned approach:
1. Drive time synthetically at fixed `1/fps` steps.
2. Render each frame to a macroquad `render_target`.
3. Write each frame as PNG.
4. Stitch with ffmpeg.

## Module Location

```
src/camera/
  mod.rs        Camera struct, to_macroquad(), Animatable impl, helpers
  orbit.rs      OrbitController (spherical coords, mouse input)
```
