# Scene Graph & Property System

Implemented in `src/scene/`.

## Scene Graph

- **Storage:** `Vec<Option<Box<dyn SceneNode>>>` with sequential `ObjectId(usize)` keys. See `src/scene/mod.rs`.
- **Operations:** `add`, `remove`, `get`, `get_mut`, `iter`, `draw_all`.
- **SceneNode** is a supertrait combining `SceneObject + Animatable`, auto-implemented for any type implementing both.
- **Current object types:** `Circle`, `Line` (in `src/scene/objects/`). Future: Axes, Text, etc.

## Traits (`src/scene/traits.rs`)

**SceneObject** — `draw()`, `bounding_box() -> BoundingBox`.

**Animatable** — `get(&str) -> Option<AnimValue>`, `set(&str, AnimValue)`, `property_names() -> &[&str]`.

## AnimValue (`src/scene/value.rs`)

Variants: `Float(f32)`, `Vec2`, `Vec3`, `Vec4` (also Color RGBA), `Bool`, `Transform2D { position, rotation, scale }`.

`AnimValue::lerp(a, b, t)` interpolates between matching variants. Bool snaps at `t >= 0.5`. Panics on mismatched variants.

## Design Notes

- **String-keyed properties** allow the animation engine to work generically without compile-time coupling.
- **Round-trip invariant:** `set(name, value)` then `get(name)` returns the same value. Enforced by tests.
- **Typo risk** will be mitigated by `SceneBuilder` validating property names at scene construction time (not yet implemented).
