# L-System Implementation

A deterministic, context-free L-system engine: **string rewriting** (apply rules to an axiom) followed by **turtle graphics** (interpret the string as line segments).

## Module Structure

- **`src/scene/l_system.rs`** — Pure engine, no rendering dependencies. `apply_rules(config: &LSystemConfig, iterations: usize) -> String` does the rewriting; `get_lines(l_string: &str, theta: f32, step_distance: f32) -> Vec<LineSegment>` runs the turtle. Also home to the preset constructors (`dragon_curve()`, `sierpinski()`, etc.), each returning `(LSystemConfig, f32)` — config plus a sensible default theta.
- **`src/scene/polyline.rs`** — Shared line-rendering utility, not L-system-specific. Defines `LineSegment`, `take_progress` (sub-segment progress reveal), `draw_polyline_mesh` (chunked hand-built quad meshes, `MAX_SEGMENTS_PER_MESH = 833`), and gradient-aware `PolylineStyle`/`PolylineTransform`. Both `LSystem` (`src/scene/objects/l_system.rs`) and `Polyline` (`src/scene/objects/polyline.rs`) delegate their mesh building here — the L-system object holds no drawing code of its own beyond calling `draw_polyline_mesh`.
- **`src/scene/color.rs`** — `gradient_sample` powers the multi-color gradient (`with_colors`) shared by both objects.

## Turtle Graphics Gotchas

- Turtle state is plain floats (`x: f32, y: f32, angle: f32`) plus a `Vec<(f32, f32, f32)>` stack — not `Vec2`-based. `get_lines` takes **no start position**; the turtle always starts at `(0, 0)` facing `π/2` (up). World placement happens later via the scene object's `position`/`scale` transform, not the engine.
- y-up coordinate system: forward step uses `+sin(dir)`, not `-sin`.
- `+` is CCW (left, `angle += theta`), `-` is CW (right, `angle -= theta`) — easy to flip when porting from a screen-coordinate reference.
- `]` with an empty stack panics (`expect("unmatched ']' in L-system string")`) — this is intentional; malformed strings should fail loudly rather than silently no-op.
- Characters with no matching rule (`+`, `-`, `[`, `]`, and rewriting-only variables like `X`, `A`, `B`) always pass through `apply_rules` unchanged; only `F`/`G` draw in `get_lines`.

## Scene Object (`LSystem`)

7 animatable properties (`Self::PROPERTY_NAMES` in `src/scene/objects/l_system.rs`): `position`, `color`, `theta`, `scale`, `iterations`, `progress`, `line_width`.

- `theta` is **not** stored in `LSystemConfig` — the engine takes it as a parameter to `get_lines` at call time. The scene object holds its own `theta: f32` field (seeded from the preset's default, animatable independently), so a keyframed `theta` track diverges the rendered curve from the preset's original angle.
- `iterations` is a `Float` (so it can be keyframed) but is floored before calling `apply_rules`. Because string rewriting is discrete, animating `iterations` across an integer boundary produces a hard jump in the curve, not a morph — there's no interpolation between iteration levels.
- `progress` reveal is sub-segment smooth, not `floor(progress * total_lines)`: `polyline::take_progress` lerps the boundary segment's endpoint, so write-on animation doesn't visibly jump between whole segments.
- Gradient coloring: `LSystem::with_colors(Vec<Color>)` sets a color list; segments sample from it via `gradient_sample` keyed by a *stable* total segment count (`total_segment_count()`, computed at full iteration/progress) so colors don't shift as `progress` reveals more of the curve.
- `bounding_box()` is computed from the current segment set (post-`progress`), scaled and translated by `scale`/`position`.

## Preset Configurations

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

Note: `My7` with theta=`π/3` produces a Koch snowflake. The `+++` and `---` in `My8` are three repeated single-character turn operations, not special syntax.
