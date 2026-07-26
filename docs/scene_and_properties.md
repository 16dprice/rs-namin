# Scene Graph & Property System

For trait definitions, storage, and object implementations see `src/scene/`.

## Rendering Scene Objects

macroquad's built-in 3D primitives (`draw_sphere`, `draw_cube`, etc.) are true 3D volumes — they look the same from every angle. For a 2D-style animation engine rendered in 3D space, objects need to be **flat shapes on a specific plane** so they show correct perspective when the camera orbits.

### Custom mesh approach

Build flat meshes using `draw_mesh` with macroquad's `Vertex` struct. Each vertex needs: `position` (Vec3), `uv` (Vec2), `color` ([u8; 4] via `Color::into()`), `normal` (Vec4).

**Convention:** Objects are flat on the **XY plane** (normal along +Z). This means:
- From the default front view (camera looking along -Z), shapes appear as expected 2D forms.
- Orbiting the camera reveals them as flat — giving depth context.
- The `position.z` field controls depth ordering between objects.

**Two paths for new objects, both going through `MeshBuilder` (`src/scene/mesh.rs`):**

1. **Line-based objects** (paths, curves, anything built from connected segments): delegate to `src/scene/polyline.rs`. It owns `LineSegment`, quad-per-segment mesh building (`draw_polyline_mesh`, one `MeshBuilder::quad` call per segment), progress-reveal (`take_progress`, sub-segment-accurate), and gradient coloring (via `src/scene/color.rs::gradient_sample`). `LSystem` and `Polyline` (`src/scene/objects/l_system.rs`, `polyline.rs`) are both thin wrappers over this — see [l_system_implementation.md](l_system_implementation.md) for how that delegation looks in practice.
2. **Everything else** (radial/rectangular shapes with their own topology): implement `fn build(&self, mb: &mut MeshBuilder)`, using `mb.fan(...)` for radial shapes (Disk, Spiral, Polygon), `mb.strip(...)` for row-to-row rectangular/tube topology (Ring, Arc, Torus, Tube), or `mb.quad(...)`/`mb.primitive(...)` directly. All vertices share the same Z (`self.position.z`) and normal (`vec4(0, 0, 1, 0)`, exposed as `mesh::FLAT_NORMAL`); `mesh::flat_vertex(pos, uv, color)` and `mesh::color_bytes(vec4)` build the `Vertex`/color values. Call it from `SceneObject::draw()` via `let mut mb = MeshBuilder::new(); self.build(&mut mb); mb.draw();`. Reference: `Disk` in `src/scene/objects/disk.rs`.

### macroquad draw call limits

macroquad's default draw call buffer is **10,000 vertices / 5,000 indices** (`MeshBuilder::MAX_VERTICES_PER_MESH`/`MAX_INDICES_PER_MESH`). A single `draw_mesh` call that exceeds these limits is silently clamped. `MeshBuilder` owns this in one place: every object appends primitives (`quad`/`fan`/`strip`/`primitive`) and it starts a new mesh automatically whenever the next primitive wouldn't fit, rebasing indices per mesh. Objects no longer track their own chunk-size constants (the old per-object `MAX_DOTS_PER_MESH`/`MAX_RINGS_PER_CHUNK`/`MAX_SEGMENTS_PER_MESH` are gone) — if you're adding an object with many primitives, just keep calling `mb.quad`/`mb.fan`/`mb.strip` and chunking is automatic. Exception: `VectorText` (`src/scene/objects/vector_text.rs`) tessellates via lyon instead of `MeshBuilder` and does its own chunking with a vertex-remapping pass, since lyon hands back one big vertex/index buffer per contour rather than one primitive at a time.

### When to use macroquad primitives

`draw_line_3d` is fine for lines since they have no surface area. Only use macroquad's built-in shape primitives for debug/helper drawing (grid, axes), not for scene objects.

## The `animatable!` Macro

Most objects declare `Animatable` with one macro call listing `field: Variant` pairs (see `src/scene/traits.rs`):

```ignore
animatable!(Disk { position: Vec3, radius: Float, color: Vec4 });
```

This generates `get`/`set`/`property_names` from the single field list, so the three can't drift apart the way hand-written parallel `match` arms could. Fields must be `Copy` and public.

- **`set` on an unknown property or mismatched variant is a `debug_assert!` (panics in debug, silent no-op in release).** Release builds don't panic because `Timeline::apply` calls `set` every frame for every track — a panic there would crash playback. `SceneBuilder` is what actually catches these mistakes early, at scene construction time (see below); the macro's debug assert is a second line of defense for code paths that bypass `SceneBuilder`.
- **Objects whose setters need side effects implement `Animatable` by hand instead of using the macro.** `Turtle` (`src/scene/objects/turtle.rs`) is the one example: setting `progress` must also derive `position`/`rotation` from the path and re-sync a child `Sprite`, which a straight field assignment can't do.
- **Property round-trip tests are a shared helper, not per-object boilerplate.** `traits::test_support::assert_property_roundtrip` (test-only) iterates `property_names()`, perturbs each value, and asserts `set` then `get` returns it, plus that an unknown name returns `None`. Every macro-based object's inline test calls this one function instead of writing its own round-trip logic. `Turtle` has its own hand-written round-trip tests instead, since its `set` has side effects the shared perturbation helper doesn't account for.

## Property Conventions

- **`progress: Float` means "0.0–1.0 reveal fraction" everywhere it appears** (`Text`, `Ring`, `LSystem`, `VectorText`, `Turtle`). 0.0 = nothing shown/at path start, 1.0 = fully shown/at path end. Keep this meaning if you add `progress` to a new object — it's what lets generic "write-on" animation code work across object types.
- **`rotation: Float` vs `orientation: Mat4` are different things.** `rotation` (present on 2D objects like `Polygon`) is a single planar Z-axis angle in radians. `orientation` (only on `Torus`, a true 3D mesh) is a full `Mat4` — animate it with `AnimValue::Mat4` keyframes (see `Torus::orientation` and the `examples/torus.rs` scene), not `rotation_x/y/z` (those are Camera-only Euler fields, see [camera_and_rendering.md](camera_and_rendering.md)).
- **Some properties are conceptually integers but stored as `Float`** so they can be keyframed (`AnimValue` has no integer variant): `Polygon::sides`, `LSystem::iterations`. Both are floored (and `sides` clamped to a minimum of 3) at draw/use time, not at set time — the stored float value round-trips exactly through `set`/`get` even mid-animation.

## Design Notes

- **String-keyed properties** allow the animation engine to work generically without compile-time coupling. Typos are caught at scene construction time by `SceneBuilder` (see `src/scene_builder.rs`), which validates property names and AnimValue types.
- **SceneBuilder validates at runtime, not at Rust compile time.** When you call `animate()` or `animate_camera()`, it checks the property name against the object's `property_names()` list and compares each keyframe's `AnimValue` variant against the property's current value. If either check fails, it panics immediately with a descriptive error listing the valid properties or expected type. This catches mistakes as soon as the scene `build()` function runs, rather than silently doing nothing when the timeline tries to apply a bad track minutes into a render.
- **SceneBuilder has a sequential/parallel authoring API** (`animate_seq`, `parallel`, `wait`, `animate_for`, etc.) for building animations without manual time tracking. See [animation_and_clock.md](animation_and_clock.md) > "Sequential and Parallel Animation Authoring" for gotchas.
