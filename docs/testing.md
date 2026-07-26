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

Every macro-based object (all of `src/scene/objects/` except `Turtle`) has a `property_round_trip` test that calls the shared helper `traits::test_support::assert_property_roundtrip` (`src/scene/traits.rs`): it iterates `property_names()`, perturbs each value, asserts `set` then `get` returns it, and checks an unknown name returns `None`. Adding a new macro-based object gets this coverage for free with one test-body line — no per-object round-trip logic to write. `Turtle` implements `Animatable` by hand (its `set` has side effects) and has its own hand-written round-trip tests instead.

Since per-object property coverage now lives inline in each object file, `src/tests/scene_integration.rs` covers only `Scene` container behavior (add/remove/iterate/len) — it is not where to look for property tests.

## Mesh Building Tests

- `MeshBuilder` (`src/scene/mesh.rs`) has its own unit tests: chunking triggers when the next primitive would exceed the 10k-vertex/5k-index limits, indices are rebased correctly per chunk, `quad`/`fan`/`strip` produce the expected index patterns, and a single oversized primitive panics. Per-object mesh tests (e.g. `Ring::mesh_half_sweep`, `Torus::mesh_vertex_count`) only assert their own vertex/index *counts* — chunking behavior itself is covered once, here, not per object.

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
- Input-driven tests via `ScriptedInput`: orbit direction, elevation clamping, zoom direction/clamping, pan direction, WASD movement, no-input stability. See `src/camera/orbit.rs`.
- Snap-to-view: `snap_front`, `snap_right`, `snap_top` set correct azimuth/elevation, preserve target/distance. See `src/camera/orbit.rs`.

## Debug Overlay Tests

- Key toggle tests via `ScriptedInput`: play/pause, HUD visibility, camera follow, speed adjustment, step-forward-pauses, snap-to-view returns. See `src/debug/mod.rs`.
- Camera state log: ring buffer wrap, deduplication, iteration order. See `src/debug/camera_log.rs`.

## Render Utility Tests

- `rgba_to_rgb_flipped` correctly strips alpha and flips rows. See `src/render_util.rs`.
- `rgba_flipped` preserves alpha and flips rows.

## Export Tests

- Frame count math: `floor(start * fps)` to `ceil(end * fps)` produces the expected frame range. See `src/bin/export.rs`.

## Snapshot Tests

- `output_path` logic for single vs. multiple frame output. See `src/bin/snapshot.rs`.
- Rendering pipeline is verified manually via `cargo run --bin snapshot`. See [agent_testing.md](agent_testing.md) for usage.

## Scenario Integration Tests

- Multi-frame orbit accumulation, zoom clamping, pan-then-orbit composition, idle timeline playback, camera stability, WASD movement. See `src/tests/scenarios.rs`.

## SceneBuilder Validation Tests

- Property name validation: invalid names panic with descriptive error. See `src/scene_builder.rs`.
- Type validation: wrong AnimValue variant panics.
- End-to-end: `build()` then `timeline.apply()` produces correct interpolated values.
- `all_object_types_work` exercises most object types plus camera properties, but is not exhaustive (e.g. `Ring` is not covered) — check the test itself in `src/scene_builder.rs` rather than assuming full coverage.
