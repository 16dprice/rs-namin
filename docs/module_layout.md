# Module Layout & Main Loop

## Main Loop Structure

See `src/viewer.rs` for the full implementation. The ordering below is load-bearing:

```
each frame:
    handle debug keybindings (toggle overlay, snap views)
    handle transport keys (play/pause, step, speed)
    scrub_bar.update(&mut clock)
    clock.tick(real_dt)                          // no-op if paused
    if camera_follow_timeline:
        camera = initial_camera.clone()
        timeline.apply(clock.current_time, &mut scene, &mut camera)
    else:
        timeline.apply_scene_only(clock.current_time, &mut scene)

    set_camera(camera.to_macroquad())
    debug.draw_world(...)                        // grid, axes
    scene.draw_world()                           // world-space objects

    set_default_camera()                         // switch to screen space
    scene.draw_screen()                          // screen-space objects (Text)
    debug.draw_hud(...)                          // camera info, value inspector
    scrub_bar.draw(...)

    orbit_controller.update(&mut camera)         // runs last to avoid consuming UI input
```

### Why this order matters

- Input is handled before state updates so keybindings take effect on the current frame.
- Clock ticks before timeline applies, ensuring evaluation at the new time.
- Timeline applies even when paused — this is what makes scrubbing work.
- Two-pass rendering: world-space objects draw after `set_camera`, screen-space UI draws after `set_default_camera()`.
- Orbit controller runs last so it doesn't consume mouse input before UI elements (e.g., scrub bar dragging).

## Scene Organization

Scenes are `fn() -> (Scene, Timeline, Camera)`. Two registries organize them:

- **`src/videos/`** — `Video` struct + `VIDEOS` registry. Used by CLI export (`src/bin/export.rs`).
- **`src/examples/`** — `Example` struct + `EXAMPLES` registry. Used by the example picker (`src/bin/example.rs`) and snapshot tool (`src/bin/snapshot.rs` via `--scene`).

`src/my_scene.rs` is the active user scene — used by the interactive viewer (`src/main.rs`) as the default scene.
