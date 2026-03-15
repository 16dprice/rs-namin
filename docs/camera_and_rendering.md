# Camera & Rendering

See `src/camera/mod.rs` for the Camera struct and `src/camera/orbit.rs` for the orbit controller.

## Gotchas

- **FOV is stored in degrees** but macroquad's `Camera3D` expects radians. `to_macroquad()` handles the conversion — don't convert manually.
- **macroquad spells it `Orthographics`** (not Orthographic). `to_macroquad()` handles this mapping via `ProjectionMode`.
- **`mouse_delta_position()` returns coords in the -2..2 range** (internally `pixel / screen * 2 - 1`). The orbit controller multiplies by `screen / 2` to recover pixel deltas.
- **Orbit pan scale** is derived from FOV and distance for 1:1 mouse tracking: `2 * distance * tan(fov/2) / screen_height`.

## Camera Properties

The `Camera` struct supports the following animatable properties:

- `position` (Vec3), `target` (Vec3), `up` (Vec3) — standard camera transform
- `fov` (Float) — field of view in degrees
- `near` (Float), `far` (Float) — clipping planes
- `rotation_x` (Float), `rotation_y` (Float), `rotation_z` (Float) — Euler rotations in radians (pitch, yaw, roll). Applied as a quaternion (`Quat::from_euler(EulerRot::YXZ, y, x, z)`) to the position offset from target via `rotated_position()`. This allows animating a camera orbit with a single rotation track instead of computing position keyframes.

## Timeline Camera Mode

Toggled with F5 (`camera_follow_timeline` flag on `DebugOverlay`). When active, the camera resets to its initial state each frame and is fully driven by timeline tracks — orbit input is ignored. When toggled off, orbit mode resumes and `apply_scene_only()` is used so camera tracks don't conflict with manual control. See `src/viewer.rs`.

## Input Abstraction

The orbit controller and debug overlay accept `&dyn InputProvider` (defined in `src/input.rs`) instead of calling macroquad input functions directly. This enables scripted input injection in tests via `ScriptedInput`. See [agent_testing.md](agent_testing.md).

## Render Targets

Both snapshot and export render to an offscreen `RenderTarget` rather than the default framebuffer. There are two critical gotchas with macroquad render targets:

### Depth buffer must be explicitly enabled

`render_target()` defaults to `depth: false` — no depth buffer is created. Without a depth buffer, depth testing silently does nothing and triangles draw in submission order. This means 3D scenes with overlapping geometry (e.g. a torus knot) will render with incorrect depth ordering: parts that should be behind will draw in front.

The fix is to use `render_target_ex()` with `depth: true`:

```rust
use macroquad::texture::{render_target_ex, RenderTargetParams};

let rt = render_target_ex(width, height, RenderTargetParams {
    depth: true,
    ..Default::default()
});
```

**How we found this:** The interactive viewer (which renders to the default framebuffer) showed correct depth ordering, but snapshot/export output had self-intersections in a torus knot. The `snapshot --time 10` tool was used to capture a reproducible frame. Inspecting macroquad's source revealed that `RenderTargetParams::default()` sets `depth: false`, and `render_target()` calls `render_target_ex` with that default.

### Viewport must match render target resolution

When a `Camera3D` renders to a `RenderTarget`, macroquad still uses the *window* dimensions for the projection matrix aspect ratio unless you set an explicit viewport. The snapshot/export binaries cap the window size (e.g. `width.min(1280)`), so the window may be smaller than the render target. This mismatch distorts the perspective.

The fix is to set the viewport on the Camera3D to match the render target:

```rust
cam3d.render_target = Some(rt.clone());
cam3d.viewport = Some((0, 0, width as i32, height as i32));
```

### Checklist for new render-to-texture code

1. Use `render_target_ex` with `depth: true` if the scene has any 3D geometry.
2. Set `cam3d.viewport` to match the render target dimensions.
3. Test with `snapshot --time T` and compare against the interactive viewer at the same time.

## CLI Snapshot

Run with `cargo run --bin snapshot`. Renders single frames to PNG for visual inspection. See `src/bin/snapshot.rs`.

```sh
cargo run --bin snapshot -- --time 1.5 --output frame.png
cargo run --bin snapshot -- --times 0,0.5,1.0 --output frames/
```

## CLI Export

Run with `cargo run --bin export`. See `src/bin/export.rs`.

### Why a separate binary?

macroquad batches GPU draw commands and only flushes on `next_frame().await`. This means readback (`get_texture_data`) requires a flush between render and read. An in-app export was limited to display framerate. The separate binary disables vsync so `next_frame()` returns immediately after flushing, running as fast as the GPU allows.
