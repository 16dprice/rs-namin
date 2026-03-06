# Module Layout & Main Loop

## Directory Structure

```
src/
  main.rs                 macroquad entry point, mode switching, main loop
  scene/
    mod.rs                Scene struct, ObjectId (slotmap key), add/remove/iterate
    objects/              Circle, Line, Axes, Text, etc.
    traits.rs             SceneObject trait (draw, bounding_box) + Animatable trait
  animation/
    mod.rs
    value.rs              AnimValue enum + lerp implementation
    track.rs              Track, Keyframe, evaluate()
    timeline.rs           Timeline (collection of tracks), apply()
    easing.rs             Easing functions (linear, quad, cubic, etc.)
  camera/
    mod.rs                Camera struct, to_macroquad(), derived helpers
    orbit.rs              OrbitController (spherical coords, input handling)
  clock.rs                Clock, PlaybackState, LoopMode, transport logic
  debug/
    mod.rs                DebugOverlay (HUD, world-space draws, keybindings)
    scrub_bar.rs          ScrubBar (visual timeline + drag-to-scrub)
    value_inspector.rs    Per-object property viewer (when paused)
    camera_log.rs         CameraDebugLog, CameraSnapshot, dump_recent()
  render/
    mod.rs                RenderContext, draw dispatch
    export.rs             Offline frame capture, PNG export, ffmpeg invocation
  script/
    mod.rs                SceneBuilder DSL, validation
```

## Main Loop Structure

```
each frame:
    handle debug keybindings (toggle overlay, snap views)
    handle transport keys (play/pause, step, speed)
    scrub_bar.update(&mut clock)
    clock.tick(real_dt)                          // no-op if paused
    timeline.apply(clock.current_time, &mut scene)  // always runs

    match mode:
        Interactive => orbit_controller.update(&mut camera)
        Playback    => (camera already set by timeline.apply)

    set_camera(camera.to_macroquad())
    debug.draw_world(...)                        // grid, axes, bounding boxes
    scene.draw_all()

    set_default_camera()                         // switch to screen space
    debug.draw_hud(...)                          // camera info, value inspector
    scrub_bar.draw(...)

    if exporting: capture framebuffer to PNG
```

### Frame Ordering Notes

- Debug keybindings and transport keys are handled first, before any state updates.
- The clock ticks before the timeline applies, ensuring the timeline evaluates at the new time.
- The timeline always applies, even when paused — this is what makes scrubbing work.
- World-space debug draws happen after `set_camera` but before screen-space UI.
- Screen-space UI (HUD, scrub bar) draws after `set_default_camera()`.
- Export capture happens last, after all drawing is complete.

## Scene Builder DSL

The `SceneBuilder` provides a builder-pattern API for defining scenes in Rust:

- Collects objects, animation tracks, and camera keyframes.
- Produces a `(Scene, Timeline)` pair.
- **Validates property names at build time** — if a track references a property that doesn't exist on its target object, the builder returns an error instead of silently creating a broken animation.
