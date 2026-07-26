# Module Layout & Main Loop

## App Shell

`src/app.rs` owns the window loop: `app::run(AppMode)` dispatches one frame per mode
(`Library` or `Viewer(ViewerMode)`), applies the `UiRequest` transition the mode's UI
returned, and handles the `RS_NAMIN_FRAME_DUMP` capture. `rs-namin` starts in the viewer on
`my_scene`; the `example` binary starts in the library. Opening a scene from the library
constructs a fresh `ViewerMode` (rebuilding the scene — animation state never leaks between
visits). Scene builds happen inside the loop, i.e. inside the GL context.

## Viewer Frame Structure

See `ViewerMode::frame` in `src/viewer.rs` for the full implementation. The ordering below is load-bearing:

```
each frame:
    capture = ui::layout(...)                      // egui input+layout pass FIRST — decides what input egui captures
    input = UiGatedInput::new(&raw, capture...)    // scene controls see suppressed pointer/keyboard while UI has them
    snap = debug.handle_input(&mut clock, &input)  // keybindings, transport keys; returns snap-to-view request
    debug.update(&mut clock)                       // scrub bar drag state
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
    debug.draw(...)                                // mouse coords, value inspector, scrub bar (HUD is the egui window)
    debug.scrub_bar.draw_ticks(&timeline, clock.duration)

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

Scenes are `fn() -> (Scene, Timeline, Camera)`. `src/registry.rs` holds one `SceneEntry` list (name, description, `SceneKind` badge, build fn, default audio) covering every example, video, and the scratch scene — `src/examples/mod.rs` and `src/videos/mod.rs` are now just `pub mod` declarations feeding that list, not registries of their own.

- Every binary resolves scenes against `registry::SCENES`/`registry::find`: `rs-namin` looks up `my_scene` and opens the viewer on it, `example` renders the registry as the in-app library screen, and `snapshot --scene NAME` / `export --scene NAME` look entries up directly.
