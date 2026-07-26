# Animation Engine & Clock

See `src/animation/` and `src/clock.rs` for implementations. Scene authoring via `SceneBuilder` lives in `src/scene_builder.rs`.

## Sequential and Parallel Animation Authoring

`SceneBuilder` has two authoring modes that can be freely mixed:

**Absolute-time mode** (`animate`, `animate_camera`, `keyframe`, `keyframe_with_easing`): keyframe times are literal absolute seconds. The cursor is not touched. Use this when you have a fixed known time (e.g., a camera zoom-out at t=35.0).

**Cursor mode** (`animate_seq`, `animate_camera_seq`, `animate_for`, `wait`, `set_cursor`, `parallel`): keyframe times are relative to the cursor. Each seq call advances the cursor by the animation's duration. Use this to chain animations end-to-end without manually tracking absolute times.

### Key gotchas

**`animate_for` requires a prior keyframe.** It sets the easing on the *previous* keyframe and adds a new one. Calling it on an empty `TrackBuilder` panics. Always start with `keyframe(0.0, ...)` before chaining `animate_for` calls:
```ignore
tb.keyframe(0.0, AnimValue::Float(1.0))
  .animate_for(1.0, AnimValue::Float(5.0), Easing::SineOut)   // ok
  .animate_for(1.0, AnimValue::Float(1.0), Easing::SineIn)    // ok
```

**`parallel` advances by the longest, not the sum.** The cursor ends at `start + max(durations)`. Animations inside a `parallel` block all start from the same cursor position. Each one's relative keyframe t=0 maps to that same absolute start time.

**`animate_seq` cursor advance is driven by max keyframe time in the closure.** The cursor advances by however large the largest keyframe time is inside the closure — not by a separate duration argument. If your closure goes up to t=3.0, the cursor advances 3.0 seconds.

**Mixing modes is intentional.** You can use `animate` (absolute) alongside `animate_seq` (cursor-relative) in the same builder. The cursor only moves when seq methods are called. A typical pattern: use `parallel` for the main drawing phase (fixed duration known from data), then `set_cursor` + `animate_seq` for a text reveal sequence with staggered offsets.

### `wait` vs `set_cursor`

`wait(duration)` advances the cursor by `duration` — useful for inserting pauses between sequential animations. `set_cursor(time)` jumps to an absolute time — useful when you want to start a sequence at a specific moment regardless of cursor state (e.g., `set_cursor(text_sequence_start + 1.0)` to stagger text reveals by offset from a known anchor).

## Easing Functions

Easing is **data**: keyframes store the `Easing` enum (`src/animation/easing.rs`) — 28 named variants (linear, quad, cubic, quart, quint, sine, expo, back, elastic, bounce, each with in/out/in-out) plus `Easing::Custom(fn)` for code-authored curves. Named variants serialize (scene documents use them); `Custom` does not. All named variants satisfy boundary invariants: `f(0)=0`, `f(1)=1`.

## Custom Easing Functions

Custom curves are plain `fn(f32) -> f32` functions wrapped in `Easing::Custom`. The input `t` is normalized progress through the segment (0.0 to 1.0), and the output is the eased value (typically 0.0 to 1.0, though overshoot easings like `back` can exceed this range). See `dolly_zoom` in `src/videos/torus_knot.rs` for an example that follows a `1/tan` curve.

### Custom easing limitation (remaining tech debt)

`Easing::Custom` holds a bare function pointer, so custom curves still **cannot capture state** — no `dolly_zoom(fov_start, fov_end)` factory; scene-specific values must be hardcoded in the function body. The planned resolution is a parameterized serializable variant (e.g. `CubicBezier`) rather than boxed closures — see docs/gui_plan.md (curve editor, M2.4).

## Gotchas

- **Easing is per-segment, applied from the starting keyframe.** The easing function on keyframe N controls the curve from keyframe N to N+1 — not the arrival at N.
- **`Bool` doesn't interpolate — it holds the start value until the segment midpoint, then snaps.** `AnimValue::lerp` for `Bool` returns `a` while `t < 0.5` and `b` from `t >= 0.5` onward (see `src/scene/value.rs`). Easing curves on a `Bool` track still only affect which side of 0.5 `t` lands on, not any gradient.
- **Timeline runs every frame regardless of pause state.** This is what makes scrubbing work — the clock position changes, and the next `timeline.apply()` picks it up.
- **`apply` vs `apply_scene_only`:** `apply(time, &mut scene, &mut camera)` drives both scene objects and camera. `apply_scene_only(time, &mut scene)` skips camera tracks, used when the orbit controller drives the camera instead.

## Design Notes

- Paused is a distinct state, not speed=0. See `PlaybackState` enum.
- Once mode auto-pauses at end. Loop wraps. PingPong reverses direction at boundaries.
- Scrubbing and stepping directly mutate `current_time`; timeline re-evaluates next frame.
