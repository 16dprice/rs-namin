# Module Layout & Main Loop

## Main Loop Structure

See `src/viewer.rs` for the full implementation. The ordering below is load-bearing:

```
each frame:
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
    debug.draw(...)                                // HUD, mouse coords, value inspector, scrub bar
    debug.scrub_bar.draw_ticks(&timeline, clock.duration)

    if camera_follow_timeline:
        orbit = OrbitController::from_camera(&camera) // re-derive so stale orbit state doesn't leak in when toggled off
    else:
        orbit.update(&mut camera, &input)              // runs last to avoid consuming UI input

    debug.record_camera(&camera, clock.current_time)   // dedup ring buffer, see camera_and_rendering.md
```

### Why this order matters

- Input is handled before state updates so keybindings take effect on the current frame.
- Clock ticks before timeline applies, ensuring evaluation at the new time.
- Timeline applies even when paused — this is what makes scrubbing work.
- **Three-pass rendering, not two**: world-space objects draw after `set_camera(camera...)`; screen-space objects then draw in their own pass under `screen_space_camera(None)` (design-space pixels, matching exports); the debug overlay draws last under `set_default_camera()` (real window pixels — it isn't part of the authored scene, so it isn't subject to the design-canvas convention). See [camera_and_rendering.md](camera_and_rendering.md) > "Screen-Space Design Canvas".
- Orbit controller update runs last so it doesn't consume mouse input before UI elements (e.g., scrub bar dragging).
- In `camera_follow_timeline` mode, the orbit controller is **not** updated — it's re-derived fresh from the timeline-driven camera every frame (`OrbitController::from_camera`). This is what prevents stale orbit state from snapping the camera when the mode is toggled back off.

## Scene Organization

Scenes are `fn() -> (Scene, Timeline, Camera)`. `src/registry.rs` holds one `SceneEntry` list (name, description, `SceneKind` badge, build fn, default audio) covering every example, video, and the scratch scene — `src/examples/mod.rs` and `src/videos/mod.rs` are now just `pub mod` declarations feeding that list, not registries of their own.

- The `example`, `snapshot`, and `export` binaries all resolve a scene by name against `registry::SCENES`/`registry::find` (`example` lists every entry as an interactive picker; `snapshot --scene NAME` and `export --scene NAME` look one up directly).
- The interactive viewer (`src/main.rs`) does **not** go through the registry — it always builds `my_scene::build()` directly, since it has a fixed default scene rather than a picker. `my_scene` is also registered as the `Scratch`-kind entry `my_scene`, so `snapshot`/`export` can still target it explicitly by name.
