# Camera & Rendering

## Camera Struct

The system owns a `Camera` struct with clean, well-defined semantics:

- **Position** (`Vec3`) — where the camera is in world space.
- **Target** (`Vec3`) — the look-at point.
- **Up** (`Vec3`) — the up vector.
- **FOV** (`f32`) — field of view in degrees.
- **Near / Far** (`f32`) — clipping planes.
- **Projection** — perspective or orthographic.

### Conversion

`camera.to_macroquad() -> Camera3D` converts to macroquad's camera type in **exactly one place**. If macroquad behaves unexpectedly, there's one function to debug.

### Animatable

The camera is animatable via the same property/keyframe system as scene objects. Properties like `position`, `target`, `fov` can be keyframed on the timeline.

## Runtime Modes

### Interactive Mode

- Free orbit camera using spherical coordinates around a look-at target.
- Mouse drag to orbit, pan, zoom.
- The timeline still evaluates, so you can scrub while inspecting the scene.
- The `OrbitController` handles input and updates the camera each frame.

### Playback Mode

- Camera is fully driven by the timeline — no manual input.
- Can render to screen for preview or export frames for video.

The main loop switches behavior based on mode:
- Interactive: `orbit_controller.update(&mut camera)` runs after timeline apply.
- Playback: camera properties are already set by `timeline.apply`, so no additional update needed.

## Offline Export Pipeline

Renders frames for video output, fully decoupled from real time:

1. Drive time synthetically at fixed `1/fps` steps.
2. Render each frame to a macroquad `render_target` (off-screen texture).
3. Write each frame as a PNG to disk.
4. Stitch frames into video using ffmpeg.

Output is **deterministic and frame-perfect** regardless of render cost, since time is not tied to wall-clock.

## Module Location

```
src/camera/
  mod.rs        Camera struct, to_macroquad(), derived helpers
  orbit.rs      OrbitController (spherical coords, input handling)
src/render/
  mod.rs        RenderContext, draw dispatch
  export.rs     Offline frame capture, PNG export, ffmpeg invocation
```
