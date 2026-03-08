# Animation Engine & Clock

See `src/animation/` and `src/clock.rs` for implementations.

## Easing Functions

28 easing functions available in `src/animation/easing.rs`: linear, quad, cubic, quart, quint, sine, expo, back, elastic, bounce (each with in/out/in-out variants). All satisfy boundary invariants: `f(0)=0`, `f(1)=1`.

## Gotchas

- **Easing is per-segment, applied from the starting keyframe.** The easing function on keyframe N controls the curve from keyframe N to N+1 — not the arrival at N.
- **Timeline runs every frame regardless of pause state.** This is what makes scrubbing work — the clock position changes, and the next `timeline.apply()` picks it up.
- **`apply` vs `apply_scene_only`:** `apply(time, &mut scene, &mut camera)` drives both scene objects and camera. `apply_scene_only(time, &mut scene)` skips camera tracks, used when the orbit controller drives the camera instead.

## Design Notes

- Paused is a distinct state, not speed=0. See `PlaybackState` enum.
- Once mode auto-pauses at end. Loop wraps. PingPong reverses direction at boundaries.
- Scrubbing and stepping directly mutate `current_time`; timeline re-evaluates next frame.
