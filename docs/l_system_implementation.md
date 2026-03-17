## L-System Implementation

A deterministic, context-free L-system engine with two stages: **string rewriting** (apply rules to an axiom) and **turtle graphics** (interpret the string to produce line segments).

---

### Module Structure

**`src/scene/l_system.rs`** — Pure engine. No rendering dependencies. Exposes `apply_configuration` (string rewriting) and `get_lines` (turtle graphics). Output is `Vec<LineSegment>`.

**`src/scene/objects/l_system.rs`** — `LSystem` scene object. Wraps the engine, builds flat quad meshes from line segments (one thin quad per segment on the XY plane), and implements `SceneObject` + `Animatable`. Follows the same hand-built mesh pattern as `src/scene/objects/disk.rs`.

Line segments are rendered as hand-built flat quads — no lyon tessellation. This matches the project convention for all scene objects.

---

### Data Structures

**ReplacementRule** — `from: char`, `to: String`

**LSystemConfiguration** — `axiom: String`, `replacement_rules: Vec<ReplacementRule>`

**LineSegment** — `start: Vec2`, `end: Vec2`

---

### Stage 1: String Rewriting (`apply_configuration`)

**Input:** `LSystemConfiguration`, `iterations: usize`

**Algorithm:**

1. Start with `l_string = axiom`.
2. Repeat `iterations` times: for each character, replace it with the matching rule's `to` string, or pass it through unchanged if no rule matches.
3. Return `l_string`.

Characters with no rule (`+`, `-`, `[`, `]`, etc.) always pass through unchanged.

**Example — Dragon Curve, 3 iterations:**
```
Axiom:       "F"
Rules:       F → "F+G",  G → "F-G"

Iteration 1: "F+G"
Iteration 2: "F+G+F-G"
Iteration 3: "F+G+F-G+F+G-F-G"
```

---

### Stage 2: Turtle Graphics (`get_lines`)

**Input:** `l_string: &str`, `theta: f32` (radians), `start_position: Vec2`, `step_distance: f32`

**Default `step_distance`:** `1.0` (one world unit). Controlled at the scene-object level via the `scale` property — do not bake scaling into the engine.

**Turtle state:** `current_position: Vec2`, `dir: f32` (radians), `stack: Vec<(Vec2, f32)>`

Initial `dir` is `π/2` (facing up in world space).

| Character | Action |
|-----------|--------|
| `'F'` or `'G'` | Move forward and draw. `new_pos = current_position + step_distance * vec2(cos(dir), sin(dir))`. Emit `LineSegment { start: current_position, end: new_pos }`. Update `current_position`. |
| `'+'` | Turn left (CCW): `dir += theta` |
| `'-'` | Turn right (CW): `dir -= theta` |
| `'['` | Push `(current_position, dir)` |
| `']'` | Pop `(current_position, dir)` — panic if stack is empty |
| `' '` | Skip |
| Anything else | Ignore (variables like `X`, `A`, `B` drive string rewriting only) |

**Coordinate system:** rs-namin uses y-up world space, so the forward step uses `+sin(dir)`. Do not use `-sin`.

**`+`/`-` convention:** `+` is CCW (left), `-` is CW (right). Easy to mix up with screen-coordinate systems where the sign flips.

---

### Animatable Properties

| Property | Type | Notes |
|----------|------|-------|
| `position` | Vec3 | World-space position of the L-system |
| `color` | Vec4 | Line color |
| `theta` | Float | Turning angle in radians |
| `scale` | Float | Scales `step_distance` (default 1.0) |
| `iterations` | Float | Floored to integer before use |
| `progress` | Float 0..1 | Reveal: draw lines progressively in order |

`iterations` is Float (not integer) so it can be keyframed. Floor it when calling `apply_configuration`.

`progress` enables write-on animation by rendering only the first `floor(progress * total_lines)` segments.

---

### Preset Configurations

Theta is not stored in `LSystemConfiguration` — it is passed to `get_lines` at call time. Every preset has a default theta; use that default when no override is supplied.

| Name | Axiom | Rules | Default Theta |
|------|-------|-------|---------------|
| **Dragon Curve** | `"F"` | `F → "F+G"`, `G → "F-G"` | `π/2` (90°) |
| **Sierpinski** | `"F"` | `F → "G-F-G"`, `G → "F+G+F"` | `π/3` (60°) |
| **Koch** | `"F"` | `F → "F-F+F+F-F"` | `π/2` (90°) |
| **Fractal Plant** | `"-X"` | `X → "F+[[X]-X]-F[-FX]+X"`, `F → "FF"` | `π/7` (~25.7°) |
| **Crystal** | `"X"` | `X → "F[+X][-X]FX"`, `F → "FF"` | `π/7` (~25.7°) |
| **Binary Tree** | `"F"` | `F → "G[-F]+F"`, `G → "GG"` | `π/4` (45°) |
| **Hilbert** | `"A"` | `A → "+BF-AFA-FB+"`, `B → "-AF+BFB+FA-"` | `π/2` (90°) |
| **Tree** | `"F"` | `F → "F[+F]F[-F]F"` | `π/7` (~25.7°) |
| **My1** | `"F"` | `F → "F[+F[-F[+F[-F]]]]"` | `π/4` (45°) |
| **My2** | `"F"` | `F → "F[+F-F][-F+F]F"` | `π/4` (45°) |
| **My3** | `"F"` | `F → "F[-F-F][+F+F]F"` | `π/4` (45°) |
| **My4** | `"F"` | `F → "F-F+F"` | ~2.09 rad (~120°) |
| **My5** | `"F"` | `F → "F[F-F+F+F-F]F[F+F-F-F+F]F"` | `π/4` (45°) |
| **My6** | `"F"` | `F → "F[-F+F+F-]F"` | `π/4` (45°) |
| **My7** | `"F--F--F"` | `F → "F+F--F+F"` | `π/3` (60°) |
| **My8** | `"F"` | `F → "FF[+++F][---F]"` | `π/4` (45°) |

Note: `My7` with theta=`π/3` produces a Koch snowflake. The `+++` and `---` in `My8` are repeated single-character operations, not special syntax.

---

### Usage Example (Pseudocode)

```
config = LSystemConfiguration {
    axiom: "F",
    replacement_rules: [
        ReplacementRule { from: 'F', to: "F+G" },
        ReplacementRule { from: 'G', to: "F-G" },
    ]
}

l_string = apply_configuration(config, iterations=10)
lines = get_lines(l_string, theta=PI/2, start_position=(0, 0), step_distance=1.0)

// `lines` is now a list of line segments; the scene object builds meshes from these
```

---

### What NOT to Implement

Do not port the following:
- Bounding box calculation, auto-scaling, or translation
- Rainbow coloring or any color logic
- Animation interpolation between L-system states
- Any rendering, UI, video, or PNG export code in the engine layer (`src/scene/l_system.rs`)
- Lyon tessellation — build line segment meshes as hand-constructed thin quads, not via lyon
