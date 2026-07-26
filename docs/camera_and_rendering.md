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

`snapshot` and `export` both render through `render_util::OffscreenRenderer` (`src/render_util.rs`), which owns a depth-buffered `RenderTarget` plus the two-pass render (3D world, then design-space screen — see "Screen-Space Design Canvas" below) and pixel readback. It exists to encode two non-obvious macroquad render-target gotchas in one place instead of every render-to-texture call site re-discovering them:

### Depth buffer must be explicitly enabled

`render_target()` defaults to `depth: false` — no depth buffer is created. Without a depth buffer, depth testing silently does nothing and triangles draw in submission order. This means 3D scenes with overlapping geometry (e.g. a torus knot) will render with incorrect depth ordering: parts that should be behind will draw in front. `OffscreenRenderer::new` always calls `render_target_ex()` with `depth: true`.

**How we found this:** The interactive viewer (which renders to the default framebuffer) showed correct depth ordering, but snapshot/export output had self-intersections in a torus knot. The `snapshot --time 10` tool was used to capture a reproducible frame. Inspecting macroquad's source revealed that `RenderTargetParams::default()` sets `depth: false`, and `render_target()` calls `render_target_ex` with that default.

### Viewport must match render target resolution

When a `Camera3D` renders to a `RenderTarget`, macroquad still uses the *window* dimensions for the projection matrix aspect ratio unless you set an explicit viewport. `snapshot` caps its window to the requested resolution (`width.min(1280)`, `height.min(720)`); `export` always opens a fixed 1280x720 window regardless of the chosen export resolution. Either way the window can differ from the render target size, and that mismatch distorts the perspective if the viewport isn't set explicitly. `OffscreenRenderer::render_frame` sets `cam3d.viewport` to the render target's own dimensions every frame, so callers never need to think about the window size.

### Checklist for new render-to-texture code

Prefer using `OffscreenRenderer` directly rather than building a render target by hand. If you do need a custom one:

1. Use `render_target_ex` with `depth: true` if the scene has any 3D geometry.
2. Set `cam3d.viewport` to match the render target dimensions.
3. Test with `snapshot --time T` and compare against the interactive viewer at the same time.

## Screen-Space Design Canvas

Screen-space objects (e.g. `Text`) are positioned in pixels, but those pixels are interpreted against a fixed `DESIGN_WIDTH`/`DESIGN_HEIGHT` canvas (1280x720, `src/render_util.rs`) rather than the actual window or render-target resolution. `render_util::screen_space_camera(render_target)` builds the `Camera2D` that maps that canvas onto whatever it's given — `None` for the interactive viewer's window (`src/viewer.rs`), `Some(target)` for `OffscreenRenderer`.

This is what makes screen-space content WYSIWYG: a `Text` object at canvas position `(640, 360)` occupies the same fraction of the frame in the viewer, a 720p snapshot, and a 4K export — the design canvas is scaled to the output, not the other way around. Non-16:9 outputs stretch the canvas non-uniformly rather than letterboxing. Author screen-space positions against the 1280x720 canvas regardless of what resolution you intend to export at.

## CLI Snapshot

Run with `cargo run --bin snapshot`. Renders single frames to PNG for visual inspection. See `src/bin/snapshot.rs`.

```sh
cargo run --bin snapshot -- --time 1.5 --output frame.png
cargo run --bin snapshot -- --times 0,0.5,1.0 --output frames/
```

## CLI Export

Run with `cargo run --bin export`. See `src/bin/export.rs`. With no flags it prompts interactively (scene, resolution, fps, encoding, time range, audio); passing `--scene NAME` switches to non-interactive mode for scripting (`--resolution`, `--fps`, `--crf`/`--bitrate`, `--start`/`--end`, `--audio`, `--output` — run `--help` for the full list and defaults). Both modes resolve `NAME` against `src/registry.rs`.

### Why a separate binary?

macroquad batches GPU draw commands and only flushes on `next_frame().await`. This means readback (`get_texture_data`) requires a flush between render and read. An in-app export was limited to display framerate. The separate binary disables vsync so `next_frame()` returns immediately after flushing, running as fast as the GPU allows.
