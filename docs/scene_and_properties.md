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

**Pattern for new objects:**
1. Define a `build_mesh(&self) -> Mesh` method that generates vertices and triangle indices.
2. Use a triangle fan for radial shapes (Circle), triangle strips or quads for rectangular shapes.
3. All vertices share the same Z coordinate (`self.position.z`) and normal (`vec4(0, 0, 1, 0)`).
4. Call `draw_mesh(&self.build_mesh())` from the `SceneObject::draw()` impl.

**Reference implementation:** `Circle` in `src/scene/objects/circle.rs`.

### macroquad draw call limits

macroquad's default draw call buffer is **10,000 vertices / 5,000 indices**. A single `draw_mesh` call that exceeds these limits will be silently clamped. Objects with many primitives (e.g., `Spiral` with thousands of dots) must split into multiple `draw_mesh` calls, each within the buffer limits. See `src/scene/objects/spiral.rs` for a chunked draw implementation.

### When to use macroquad primitives

`draw_line_3d` is fine for lines since they have no surface area. Only use macroquad's built-in shape primitives for debug/helper drawing (grid, axes), not for scene objects.

## Design Notes

- **String-keyed properties** allow the animation engine to work generically without compile-time coupling. Typos are caught at scene construction time by `SceneBuilder` (see `src/scene_builder.rs`), which validates property names and AnimValue types.
- **Round-trip invariant:** `set(name, value)` then `get(name)` returns the same value. Enforced by tests.
