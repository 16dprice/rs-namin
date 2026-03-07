# Module Layout & Main Loop

## Directory Structure

```
src/
  lib.rs                  Library crate root (re-exports all modules)
  main.rs                 macroquad entry point, main loop
  scene/
    mod.rs                Scene struct, ObjectId, SceneNode supertrait, add/remove/iterate
    objects/              Circle, Line, Rectangle, Polygon
    traits.rs             SceneObject trait (draw, bounding_box) + Animatable trait
    value.rs              AnimValue enum + lerp implementation
  animation/
    mod.rs
    track.rs              Track, Keyframe, evaluate()
    timeline.rs           Timeline (collection of tracks), apply()
    easing.rs             Easing functions (linear, quad, cubic, etc.)
  camera/
    mod.rs                Camera struct, to_macroquad(), derived helpers
    orbit.rs              OrbitController (spherical coords, input handling)
  clock.rs                Clock, PlaybackState, LoopMode, transport logic
  debug/
    mod.rs                DebugOverlay (HUD, toggle state, input handling, draw dispatch)
    keybindings.rs        Keybindings struct (all configurable key mappings)
    scrub_bar.rs          ScrubBar (visual timeline + drag-to-scrub)
    value_inspector.rs    ValueInspector (per-object property viewer)
  demo.rs                 Shared demo scene definition (used by main and export binaries)
  tests/
    mod.rs                Integration test registration
    timeline_integration.rs  Timeline + scene integration tests
    scene_integration.rs     Scene graph integration tests
  bin/
    export.rs             CLI export binary (headless, vsync-off, pipes to ffmpeg)
```

## Main Loop Structure

```
each frame:
    handle debug keybindings (toggle overlay, snap views)
    handle transport keys (play/pause, step, speed)
    scrub_bar.update(&mut clock)
    clock.tick(real_dt)                          // no-op if paused
    timeline.apply(clock.current_time, &mut scene)  // always runs

    set_camera(camera.to_macroquad())
    debug.draw_world(...)                        // grid, axes
    scene.draw_all()

    set_default_camera()                         // switch to screen space
    debug.draw_hud(...)                          // camera info, value inspector
    scrub_bar.draw(...)

    orbit_controller.update(&mut camera)         // runs last to avoid consuming UI input
```

### Frame Ordering Notes

- Debug keybindings and transport keys are handled first, before any state updates.
- The clock ticks before the timeline applies, ensuring the timeline evaluates at the new time.
- The timeline always applies, even when paused — this is what makes scrubbing work.
- World-space debug draws happen after `set_camera` but before screen-space UI.
- Screen-space UI (HUD, scrub bar) draws after `set_default_camera()`.
- Orbit controller runs last so it doesn't consume mouse input before UI elements (e.g., scrub bar dragging).

## Demo Scene

The `demo::build()` function in `src/demo.rs` constructs a shared demo scene used by both the interactive viewer and the CLI export binary. It returns a `(Scene, Timeline)` tuple with example objects and animations (bouncing ball, pulsing rectangle, spinning hexagon).
