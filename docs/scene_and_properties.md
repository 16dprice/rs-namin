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

**Two paths for new objects:**

1. **Line-based objects** (paths, curves, anything built from connected segments): delegate to `src/scene/polyline.rs`. It owns `LineSegment`, chunked quad-mesh building (`draw_polyline_mesh`), progress-reveal (`take_progress`, sub-segment-accurate), and gradient coloring (via `src/scene/color.rs::gradient_sample`). `LSystem` and `Polyline` (`src/scene/objects/l_system.rs`, `polyline.rs`) are both thin wrappers over this — see [l_system_implementation.md](l_system_implementation.md) for how that delegation looks in practice.
2. **Everything else** (radial/rectangular shapes with their own topology): hand-build a `build_mesh(&self) -> Mesh` method. Use a triangle fan for radial shapes (Disk), triangle strips or quads for rectangular ones. All vertices share the same Z (`self.position.z`) and normal (`vec4(0, 0, 1, 0)`). Call `draw_mesh(&self.build_mesh())` from `SceneObject::draw()`. Reference: `Disk` in `src/scene/objects/disk.rs`.

### macroquad draw call limits

macroquad's default draw call buffer is **10,000 vertices / 5,000 indices**. A single `draw_mesh` call that exceeds these limits will be silently clamped. Objects with many primitives must split into multiple `draw_mesh` calls, each within the buffer limits. Reference implementations: `polyline::draw_polyline_mesh` (chunk size derived from the buffer limits, shared by `LSystem`/`Polyline`); `Spiral` (`src/scene/objects/spiral.rs`) chunks dot meshes; `Tube` (`src/scene/objects/tube.rs`) chunks its path mesh; `VectorText` (`src/scene/objects/vector_text.rs`) chunks lyon-tessellated bezier paths with a vertex-remapping pass for correct index reuse.

### When to use macroquad primitives

`draw_line_3d` is fine for lines since they have no surface area. Only use macroquad's built-in shape primitives for debug/helper drawing (grid, axes), not for scene objects.

## Design Notes

- **String-keyed properties** allow the animation engine to work generically without compile-time coupling. Typos are caught at scene construction time by `SceneBuilder` (see `src/scene_builder.rs`), which validates property names and AnimValue types.
- **SceneBuilder validates at runtime, not at Rust compile time.** When you call `animate()` or `animate_camera()`, it checks the property name against the object's `property_names()` list and compares each keyframe's `AnimValue` variant against the property's current value. If either check fails, it panics immediately with a descriptive error listing the valid properties or expected type. This catches mistakes as soon as the scene `build()` function runs, rather than silently doing nothing when the timeline tries to apply a bad track minutes into a render.
- **SceneBuilder has a sequential/parallel authoring API** (`animate_seq`, `parallel`, `wait`, `animate_for`, etc.) for building animations without manual time tracking. See [animation_and_clock.md](animation_and_clock.md) > "Sequential and Parallel Animation Authoring" for gotchas.
- **Round-trip invariant:** `set(name, value)` then `get(name)` returns the same value. Enforced by tests.
