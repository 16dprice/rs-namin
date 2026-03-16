# Animation Engine & Clock

See `src/animation/` and `src/clock.rs` for implementations.

## Easing Functions

28 easing functions available in `src/animation/easing.rs`: linear, quad, cubic, quart, quint, sine, expo, back, elastic, bounce (each with in/out/in-out variants). All satisfy boundary invariants: `f(0)=0`, `f(1)=1`.

## Custom Easing Functions

You can write custom easing functions as plain `fn(f32) -> f32` functions. The input `t` is normalized progress through the segment (0.0 to 1.0), and the output is the eased value (typically 0.0 to 1.0, though overshoot easings like `back` can exceed this range). See `dolly_zoom` in `src/my_scene.rs` for an example that follows a `1/tan` curve.

### EasingFn closure limitation (tech debt)

`EasingFn` is currently `fn(f32) -> f32` — a bare function pointer. This means easing functions **cannot capture state**. You can't write a factory like `dolly_zoom(fov_start, fov_end)` that returns a closure parameterized by those values. Any scene-specific values must be hardcoded in the function body.

Changing `EasingFn` to `Box<dyn Fn(f32) -> f32>` or `Arc<dyn Fn(f32) -> f32>` would enable closures at the cost of a heap allocation per keyframe. This is tracked as tech debt in `src/animation/easing.rs`.

## Gotchas

- **Easing is per-segment, applied from the starting keyframe.** The easing function on keyframe N controls the curve from keyframe N to N+1 — not the arrival at N.
- **Timeline runs every frame regardless of pause state.** This is what makes scrubbing work — the clock position changes, and the next `timeline.apply()` picks it up.
- **`apply` vs `apply_scene_only`:** `apply(time, &mut scene, &mut camera)` drives both scene objects and camera. `apply_scene_only(time, &mut scene)` skips camera tracks, used when the orbit controller drives the camera instead.

## Design Notes

- Paused is a distinct state, not speed=0. See `PlaybackState` enum.
- Once mode auto-pauses at end. Loop wraps. PingPong reverses direction at boundaries.
- Scrubbing and stepping directly mutate `current_time`; timeline re-evaluates next frame.
