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

    set_default_camera()                           // switch to screen space
    scene.draw_screen()                            // screen-space objects (Text)
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
- Two-pass rendering: world-space objects draw after `set_camera`, screen-space UI draws after `set_default_camera()`.
- Orbit controller update runs last so it doesn't consume mouse input before UI elements (e.g., scrub bar dragging).
- In `camera_follow_timeline` mode, the orbit controller is **not** updated — it's re-derived fresh from the timeline-driven camera every frame (`OrbitController::from_camera`). This is what prevents stale orbit state from snapping the camera when the mode is toggled back off.

## Scene Organization

Scenes are `fn() -> (Scene, Timeline, Camera)`. Two registries organize them:

- **`src/videos/`** — `Video` struct + `VIDEOS` registry. Used by CLI export (`src/bin/export.rs`).
- **`src/examples/`** — `Example` struct + `EXAMPLES` registry. Used by the example picker (`src/bin/example.rs`) and snapshot tool (`src/bin/snapshot.rs` via `--scene`).

`src/my_scene.rs` is the active user scene — used by the interactive viewer (`src/main.rs`) as the default scene.
