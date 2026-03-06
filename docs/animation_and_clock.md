# Animation Engine & Clock

## Keyframe Animation

### Keyframe

A single data point: a time and an `AnimValue`, plus an optional easing function.

### Track

A `Track` is a sequence of keyframes for **one property on one object**.

- `evaluate(time: f32) -> AnimValue` — pure function that returns the interpolated value at a given time.
- Behavior at boundaries: clamp to first/last keyframe value outside the track's time range.
- Between keyframes: lerp the `AnimValue` with the easing function applied to the interpolation parameter `t`.
- **Hold segments:** if two adjacent keyframes have the same value, no interpolation occurs (effectively a step function).
- An empty track returns `None` (no override).

### Timeline

A `Timeline` is a collection of tracks.

- `apply(time: f32, scene: &mut Scene)` — evaluates every track at the given time and writes the resulting values into the scene via the `Animatable` trait.
- Evaluation is a **pure function**: given a time, produce a set of property overrides. The timeline itself has no state beyond its track definitions.
- The timeline always runs every frame, regardless of whether the clock is paused. This ensures the scene always reflects the current time position.

### Easing

Easing functions have the signature `fn(f32) -> f32`, mapping normalized time `[0, 1]` to an output curve.

Built-in easings: linear, quad in/out/in-out, cubic in/out/in-out, and more as needed.

## Clock & Transport Controls

The `Clock` struct is the **single source of truth** for time in the entire system. Nothing reads wall-clock time directly.

### State

- `current_time: f32` — the current position on the timeline.
- `playback_state: PlaybackState` — `Playing` or `Paused` (paused is a distinct code path, not speed=0).
- `playback_speed: f32` — multiplier for time advancement (e.g., 0.5x, 1x, 2x).
- `loop_mode: LoopMode` — `Once`, `Loop`, `PingPong`.
- `duration: f32` — total length of the timeline.

### Transport Operations

- **Play / Pause** — toggle `playback_state`.
- **Tick** — advance `current_time` by `real_dt * playback_speed`. No-op when paused. Handles loop wrapping.
- **Frame-step forward/back** — advance by exactly `1/fps` regardless of playback state.
- **Scrub** — set `current_time` to an arbitrary value (clamped to `[0, duration]`).
- **Speed control** — adjust `playback_speed`.

### Design Notes

- Paused state is distinct from speed=0 to keep the code paths clear and avoid edge cases.
- Scrubbing and frame-stepping directly mutate `current_time`. The timeline re-evaluates on the next frame.

## Module Location

```
src/animation/
  mod.rs          re-exports
  value.rs        AnimValue enum + lerp
  track.rs        Track, Keyframe, evaluate()
  timeline.rs     Timeline, apply()
  easing.rs       easing functions
src/clock.rs      Clock, PlaybackState, LoopMode, transport logic
```
