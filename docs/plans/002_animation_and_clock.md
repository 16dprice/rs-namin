# Plan: Animation Engine & Clock

## Goal

Implement the keyframe animation engine and clock/transport system, connecting the property system to time-driven animation.

## Steps

### 1. Easing functions (`src/animation/easing.rs`)

Define `EasingFn` as `fn(f32) -> f32`. Implement:
- `linear`, `quad_in`, `quad_out`, `quad_in_out`
- `cubic_in`, `cubic_out`, `cubic_in_out`

### 2. Track and Keyframe (`src/animation/track.rs`)

- `Keyframe` struct: `time: f32`, `value: AnimValue`, `easing: EasingFn`.
- `Track` struct: `object_id: ObjectId`, `property_name: String`, `keyframes: Vec<Keyframe>` (sorted by time).
- `Track::evaluate(time: f32) -> Option<AnimValue>`:
  - Empty track returns `None`.
  - Before first keyframe: clamp to first value.
  - After last keyframe: clamp to last value.
  - Between keyframes: compute normalized `t`, apply easing, lerp `AnimValue`.

### 3. Timeline (`src/animation/timeline.rs`)

- `Timeline` struct: `tracks: Vec<Track>`.
- `Timeline::apply(time: f32, scene: &mut Scene)`: evaluate every track, write results into scene via `Animatable::set`.
- `Timeline::duration() -> f32`: max time across all keyframes in all tracks.

### 4. Clock (`src/clock.rs`)

- `PlaybackState` enum: `Playing`, `Paused`.
- `LoopMode` enum: `Once`, `Loop`, `PingPong`.
- `Clock` struct: `current_time`, `playback_state`, `playback_speed`, `loop_mode`, `duration`, `fps`.
- Methods: `tick(dt)`, `play()`, `pause()`, `toggle()`, `step_forward()`, `step_backward()`, `scrub(time)`, `set_speed(speed)`.

### 5. Wire into main loop

- Create a simple timeline with a few keyframes animating the circle's position or radius.
- Add clock with tick in the main loop.
- Call `timeline.apply(clock.current_time, &mut scene)` each frame.
- Verify animation plays visually.

---

**Checkpoint: check in with user before writing tests.**

---

### 6. Tests

**Easing tests:**
- Boundary invariants: `easing(0.0) == 0.0`, `easing(1.0) == 1.0` for all easings.

**Track tests:**
- Empty track returns `None`.
- Single keyframe: all times return that value.
- Two keyframes: clamping before/after, correct midpoint interpolation.
- Easing effect: non-linear easing produces different result than linear at `t=0.5`.
- Hold segment: two keyframes with same value produce that value at midpoint.

**Clock tests:**
- Paused: `tick()` does not advance time.
- Playing: `tick()` advances by `dt * speed`.
- Loop mode wrapping.
- PingPong reversal.
- Frame-step forward/back by exactly `1/fps`.
- Scrub clamps to `[0, duration]`.

**Timeline integration tests:**
- Build scene + timeline, apply at known times, assert property values.

### 7. Build checks

`cargo build`, `cargo test`, `cargo clippy -- -D warnings` — all must pass.

---

**Checkpoint: check in with user before committing.**

---

### 8. Update docs

- Update `docs/animation_and_clock.md` to reflect implementation.
- Remove planning language replaced by code.
