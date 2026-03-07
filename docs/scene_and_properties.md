# Scene Graph & Property System

Implemented in `src/scene/`.

## Scene Graph

- **Storage:** `Vec<Option<Box<dyn SceneNode>>>` with sequential `ObjectId(usize)` keys. See `src/scene/mod.rs`.
- **Operations:** `add`, `remove`, `get`, `get_mut`, `iter`, `draw_world`, `draw_screen`.
- **SceneNode** is a supertrait combining `SceneObject + Animatable`, auto-implemented for any type implementing both.
- **Current object types:** `Circle`, `Line`, `Rectangle`, `Polygon`, `Spiral`, `Text` (in `src/scene/objects/`).

## Traits (`src/scene/traits.rs`)

**SceneObject** — `draw()`, `bounding_box() -> BoundingBox`, `is_screen_space() -> bool` (default `false`). Screen-space objects (like `Text`) are drawn after `set_default_camera()` instead of during the 3D camera pass.

**Animatable** — `get(&str) -> Option<AnimValue>`, `set(&str, AnimValue)`, `property_names() -> &[&str]`.

## AnimValue (`src/scene/value.rs`)

Variants: `Float(f32)`, `Vec2`, `Vec3`, `Vec4` (also Color RGBA), `Bool`, `Transform2D { position, rotation, scale }`.

`AnimValue::lerp(a, b, t)` interpolates between matching variants. Bool snaps at `t >= 0.5`. Panics on mismatched variants.

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

**Reference implementation:** `Circle` in `src/scene/objects/circle.rs` — triangle fan disk with 32 segments.

### macroquad draw call limits

macroquad's default draw call buffer is **10,000 vertices / 5,000 indices**. A single `draw_mesh` call that exceeds these limits will be silently clamped. Objects with many primitives (e.g., `Spiral` with thousands of dots) must split into multiple `draw_mesh` calls, each within the buffer limits. See `Spiral` for a chunked draw implementation.

### Screen-space objects

Objects that return `is_screen_space() == true` (e.g., `Text`) are drawn via `scene.draw_screen()` after `set_default_camera()`, using pixel coordinates. They use macroquad's `draw_text` rather than custom meshes.

### When to use macroquad primitives

`draw_line_3d` is fine for lines since they have no surface area. Only use macroquad's built-in shape primitives for debug/helper drawing (grid, axes), not for scene objects.

## Design Notes

- **String-keyed properties** allow the animation engine to work generically without compile-time coupling.
- **Round-trip invariant:** `set(name, value)` then `get(name)` returns the same value. Enforced by tests.
- **Typo risk** is mitigated by `SceneBuilder` validating property names at scene construction time (planned).
