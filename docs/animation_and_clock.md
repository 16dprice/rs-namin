# Animation Engine & Clock

Implemented in `src/animation/` and `src/clock.rs`.

## Keyframe Animation

- **Keyframe** (`src/animation/track.rs`): time + `AnimValue` + easing function. Constructors: `Keyframe::new` (linear easing) and `Keyframe::with_easing`.
- **Track**: sequence of keyframes for one property on one object. `evaluate(time) -> Option<AnimValue>` clamps at boundaries, lerps between keyframes with easing applied. Empty tracks return `None`. Easing is applied from the *starting* keyframe of each segment.
- **Timeline** (`src/animation/timeline.rs`): collection of tracks. `apply(time, &mut scene)` evaluates all tracks and writes values into the scene. `duration()` returns max time across all tracks. Runs every frame regardless of pause state.
- **Easing** (`src/animation/easing.rs`): `EasingFn = fn(f32) -> f32`. Built-in: `linear`, `quad_in/out/in_out`, `cubic_in/out/in_out`.

## Clock (`src/clock.rs`)

Single source of truth for time. Fields: `current_time`, `playback_state` (Playing/Paused), `playback_speed`, `loop_mode` (Once/Loop/PingPong), `duration`, `fps`.

Transport: `play()`, `pause()`, `toggle()`, `tick(dt)`, `step_forward()`, `step_backward()`, `scrub(time)`, `set_speed(speed)`.

## Design Notes

- Paused is a distinct state, not speed=0.
- Once mode auto-pauses at end. Loop wraps. PingPong reverses direction at boundaries.
- Scrubbing and stepping directly mutate `current_time`; timeline re-evaluates next frame.
