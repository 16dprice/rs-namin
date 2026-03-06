# Scene Graph & Property System

## Scene Graph

The scene holds all renderable objects with stable IDs.

- **Storage:** Generational arena or slotmap for stable `ObjectId` references that survive insertions/deletions.
- **Operations:** Add, remove, iterate over objects.
- **Object types:** Circle, Line, Axes, Text, and more as needed.

## SceneObject Trait

Every renderable object implements `SceneObject`:

- `draw()` — render the object using macroquad primitives.
- `bounding_box()` — return an axis-aligned bounding box for debug visualization and selection.

## Property System

Properties are the bridge between the animation engine and scene objects.

### AnimValue Enum

A tagged union representing any animatable value:

- `Float(f32)`
- `Vec2(Vec2)`
- `Vec3(Vec3)`
- `Vec4(Vec4)` — also used for Color (RGBA)
- `Bool(bool)`
- `Transform2D` — position, rotation, scale bundled together

`AnimValue` implements `lerp` for interpolation between two values of the same variant.

### Animatable Trait

Objects that can be animated implement `Animatable`:

- `get(property_name: &str) -> Option<AnimValue>` — read a property by name.
- `set(property_name: &str, value: AnimValue)` — write a property by name.
- `property_names() -> &[&str]` — list all animatable properties (used for validation and the value inspector).

### Design Notes

- **String-keyed properties** are flexible and allow the animation engine to work generically with any object type without compile-time coupling.
- **Typo risk** is mitigated by `SceneBuilder` validating all property names at scene construction time.
- **Round-trip invariant:** `set(name, value)` then `get(name)` must return the same value. This is enforced by tests.

## Module Location

```
src/scene/
  mod.rs          Scene struct, ObjectId, add/remove/iterate
  objects/        Circle, Line, Axes, Text, etc.
  traits.rs       SceneObject + Animatable trait definitions
```
