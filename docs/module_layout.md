# Module Layout & Main Loop

## App Shell

`src/app.rs` owns the window loop: `app::run(AppMode)` dispatches one frame per mode
(`Library`, `Viewer(ViewerMode)`, or `Export(ExportMode)`), applies the `UiRequest`
transition the mode's UI returned, and handles the `RS_NAMIN_FRAME_DUMP` capture.
`rs-namin` starts in the library (`RS_NAMIN_SCENE=name` opens the viewer directly); the `example` binary also starts in the library.
Opening a scene (or its export screen) constructs a fresh mode instance that rebuilds the
scene — animation state never leaks between visits. Scene builds happen inside the loop,
i.e. inside the GL context. `ExportMode` renders incrementally: one export frame per UI
frame piped to ffmpeg, with the readback lagging one frame behind the render (draw calls
only flush on `next_frame`).

## Viewer Frame Structure

See `ViewerMode::frame` in `src/viewer.rs` for the full implementation. The ordering below is load-bearing:

```
each frame:
    finish pending snapshot readback (if any)      // offscreen texture readable one frame after render
    response = ui::viewer_layout(...)              // egui input+layout pass FIRST — app bar, transport (scrubbing), HUD, inspector
    input = UiGatedInput::new(&raw, response...)   // scene controls see suppressed pointer/keyboard while UI has them
    snap = debug.handle_input(&mut clock, &input)  // keybindings, transport keys; returns snap-to-view request
    apply snap to orbit (snap_front/snap_right/snap_top)
    clock.tick(real_dt)                            // no-op if paused
    if camera_follow_timeline:
        camera = initial_camera.clone()
        timeline.apply(clock.current_time, &mut scene, &mut camera)
    else:
        timeline.apply_scene_only(clock.current_time, &mut scene)

    set_camera(camera.to_macroquad())
    debug.draw_world(...)                          // grid, axes
    scene.draw_world()                             // world-space objects

    set_camera(screen_space_camera(None))          // design-space screen pass (1280x720 canvas)
    scene.draw_screen()                            // screen-space objects (Text) — WYSIWYG with exports

    set_default_camera()                           // switch to real window pixels
    debug.draw(&camera, &input)                    // mouse-coords readout (all other chrome is egui)

    if snapshot requested: render scene to OffscreenRenderer, restore default camera

    ui::draw()                                     // egui paint pass LAST — UI on top of everything

    if camera_follow_timeline:
        orbit = OrbitController::from_camera(&camera) // re-derive so stale orbit state doesn't leak in when toggled off
    else:
        orbit.update(&mut camera, &input)              // runs last to avoid consuming UI input

    debug.record_camera(&camera, clock.current_time)   // dedup ring buffer, see camera_and_rendering.md
```

### Why this order matters

- **egui brackets the frame**: `ui::layout` runs first (egui collects input and lays out, reporting `wants_pointer/keyboard_input` for gating via `UiGatedInput`), and `ui::draw` runs after all macroquad drawing so panels paint on top. Splitting or reordering these breaks either input gating or layering.
- Input is handled before state updates so keybindings take effect on the current frame.
- Clock ticks before timeline applies, ensuring evaluation at the new time.
- Timeline applies even when paused — this is what makes scrubbing work.
- **Three-pass rendering, not two**: world-space objects draw after `set_camera(camera...)`; screen-space objects then draw in their own pass under `screen_space_camera(None)` (design-space pixels, matching exports); the debug overlay draws last under `set_default_camera()` (real window pixels — it isn't part of the authored scene, so it isn't subject to the design-canvas convention). See [camera_and_rendering.md](camera_and_rendering.md) > "Screen-Space Design Canvas".
- Orbit controller update runs last so it doesn't consume mouse input before UI elements (e.g., scrub bar dragging).
- In `camera_follow_timeline` mode, the orbit controller is **not** updated — it's re-derived fresh from the timeline-driven camera every frame (`OrbitController::from_camera`). This is what prevents stale orbit state from snapping the camera when the mode is toggled back off.

## Scene Organization

A scene builds to `(Scene, Timeline, Camera)` from one of two sources: a built-in builder fn (examples, videos, the scratch scene) or a **scene document** (`scenes/*.ron`, parsed and validated by `src/doc.rs`). `src/registry.rs` holds one `SceneEntry` list (name, description, `SceneKind` badge, `SceneSource`, default audio); `registry::scenes()` returns builtins plus documents discovered once per process — `src/examples/mod.rs` and `src/videos/mod.rs` are just `pub mod` declarations feeding that list.

- Every binary resolves scenes against `registry::scenes()`/`registry::find`: `rs-namin` and `example` render the registry as the in-app library screen (`RS_NAMIN_SCENE=name` opens the viewer directly), and `snapshot --scene NAME` / `export --scene NAME` look entries up directly.
- Builds go through `SceneEntry::build_scene()` (returns `Result` — document loading/validation can fail; CLIs exit with the message) or `build_or_error_scene()` (the app's variant, which turns a failure into a visible error-text scene).
