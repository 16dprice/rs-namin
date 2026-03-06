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

- Build a scene + timeline using `SceneBuilder`.
- Apply the timeline at known times.
- Assert that object properties have the expected interpolated values.
- Verify that multiple tracks on the same object compose correctly.

## Clock Behavior Tests

- Pause: `tick()` does not advance time when paused.
- Play: `tick()` advances by `dt * speed`.
- Loop wrapping: time wraps correctly in `Loop` mode, reverses in `PingPong`.
- Frame-step: advances by exactly `1/fps` in either direction.
- Scrub clamping: setting time outside `[0, duration]` clamps to bounds.

## Scene Builder Validation Tests

- Correct object and track counts after building.
- Rejection of invalid property names (names that don't exist on the target object).
- Rejection of tracks targeting non-existent objects.

## Visual Regression Tests

- Render the scene to a texture at specific known times.
- Compare the rendered frame against golden PNG images.
- Use a pixel-diff threshold to allow for minor rendering differences across platforms.
