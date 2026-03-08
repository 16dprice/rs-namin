# Testing Strategy

## Pure Math Tests

- Lerp correctness for all `AnimValue` variants.
- Easing boundary invariants: `easing(0.0) == 0.0`, `easing(1.0) == 1.0`.
- Easing monotonicity where applicable.

## Track Evaluation Tests

- Clamping: values before the first keyframe return the first keyframe's value; after the last, the last.
- Interpolation: mid-point between two keyframes returns the correct lerped value.
- Hold segments: two adjacent keyframes with the same value produce no interpolation artifacts.
- Easing effects: verify that a non-linear easing produces different output than linear at `t=0.5`.
- Empty tracks: return `None` (no override).

## Property Round-Trip Tests

- For every object type: `set(name, value)` then `get(name)` returns the same value.
- Covers all `AnimValue` variants for each property.

## Timeline Integration Tests

- Build a scene + timeline, apply at known times.
- Assert that object properties have the expected interpolated values.
- Verify that multiple tracks on the same object compose correctly.

## Clock Behavior Tests

- Pause: `tick()` does not advance time when paused.
- Play: `tick()` advances by `dt * speed`.
- Loop wrapping: time wraps correctly in `Loop` mode, reverses in `PingPong`.
- Frame-step: advances by exactly `1/fps` in either direction.
- Scrub clamping: setting time outside `[0, duration]` clamps to bounds.

## Orbit Controller Tests

- `compute_position` at zero angles, with azimuth, with elevation, with offset target.
- `apply_to_camera` sets position and target correctly.
- `from_camera` round-trip: deriving orbit state from a camera and recomputing position matches the original.

## Export Tests

- `rgba_to_rgb_flipped` correctly strips alpha and flips rows. Inline tests in `src/bin/export.rs`.
- Frame count math: `floor(start * fps)` to `ceil(end * fps)` produces the expected frame range.

## Not Yet Implemented

- **Scene Builder validation tests** — once `SceneBuilder` is implemented.
- **Visual regression tests** — render to texture and compare against golden images.
