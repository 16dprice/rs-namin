# Procedural Keyframes: stepped keyframes (shipped) & keyframe generators (planned)

## The original ask (Aug 2026)

From the turtle-intro workflow: the L-system reveal needed the camera/turtle
to advance one *segment* at a time, which meant programmatically generating a
keyframe per segment — thousands of them. Wanted: a GUI-native way to "put a
collection of keyframes onto an object at some specified intervals along the
progress metric," generalizing to things like "move a disk along a sine wave,
easing in and out of the peaks — keyframes dependent on some demarcated list
of sine-wave values."

## Shipped: stepped keyframes (Layer 1)

The realization: a uniform ramp of N generated keyframes (uniform times,
uniform value increments, same easing each) *is* one keyframe pair with a
stepped interpolation. So keyframes carry a `steps` field (`Keyframe::steps`,
`steps: Some(n)` in RON, a "steps" drag in the keyframe detail strip):
**arrive at this keyframe in N equal sub-steps, each shaped by the keyframe's
easing**. The detail-strip curve widget plots the real staircase.

- L-system reveal: progress 0 → 1, one arrival keyframe, `steps` = segment
  count, SineInOut. `turtle_intro` does exactly this now
  (`TrackBuilder::in_steps`) — pixel-identical to the old generated loop.
- Anything bound to a derived output (`pen_position`) inherits the stepping
  for free, since the pen is computed from the stepped progress.
- Sine-wave riding without any new machinery: a `Plot` of `sin(x)` (colors
  can be zero-alpha to act as an invisible guide), the disk bound to
  `plot.pen_position`, and a stepped progress keyframe — steps = number of
  half-periods pauses at every peak/trough.

## Planned: keyframe generators (Layer 2 — not built yet)

For value shapes that aren't a straight ramp and aren't derivable from an
existing object's output. **Decision made up front: generators are a live
spec stored in the document** (compact, re-tunable), *not* materialized
keyframes — chosen explicitly over materialization, accepting that
individually generated keyframes can't be hand-tweaked.

Sketch:

- `TrackDoc` gains an optional `generator` block:
  `(range: (t0, t1), count: N, value: <source>, easing: SineInOut)` where
  `<source>` is `Linear(from, to)` or per-component expressions in `u` (0→1
  across the range), reusing `scene::expr`. Example: disk position with
  `y = "sin(4 tau u)"`.
- Expansion happens at build time (`SceneDoc::build` appends the generated
  keyframes to the runtime `Track`); the doc stays small at any count.
- Open question for build time: does a generator *replace* the track's
  literal keyframes, coexist with them (generated range must not overlap
  hand keyframes?), or live on a track that allows no hand keyframes at all?
  Simplest first cut: a track has literal keyframes *or* a generator.
- UI: a "Generate…" button on the track lane opens a small form editing the
  spec; the dope sheet paints the generated range as a hatched band (not N
  diamonds); editing the spec regenerates on rebuild like any doc edit.
- Validation mirrors tracks: value-variant type check per component,
  count ≥ 2, range sanity — trial-build in the editor like bindings.

Related parked ideas that overlap: animatable binding offsets, value mapping
on bindings (scale/ease between source and target), time-shifted bindings
via evaluate-at-(t − delay).
