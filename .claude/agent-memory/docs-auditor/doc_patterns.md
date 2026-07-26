---
name: Documentation staleness patterns for rs-namin
description: Which docs tend to go stale, why, and what to check first in future audits
type: project
---

## Highest-Risk Files (check first)

**`docs/module_layout.md`** — Contains pseudocode of the main frame loop. Goes stale when:
- Function signatures change (e.g., adding parameters like `&input` to `orbit.update`)
- Debug overlay API refactors (e.g., consolidating `draw_hud` + `scrub_bar.draw` into `draw`)
- Call order changes in viewer.rs
Fix: compare pseudocode against `src/viewer.rs` function call by function call.

**This file has now drifted twice** (first fix 2026-03-15, second 2026-07-26 — see [audit_2026_03_15.md](audit_2026_03_15.md)). The second round of drift added: `scrub_bar.draw_ticks(&timeline, clock.duration)` (never in the pseudocode at all), `debug.record_camera(...)` at the end of the frame (never in the pseudocode), and — most importantly — the pseudocode implied `orbit_controller.update` runs unconditionally every frame, but in `camera_follow_timeline` mode it is **not** called at all; instead `orbit` is fully replaced via `OrbitController::from_camera(&camera)`. This is exactly the kind of conditional-vs-unconditional distinction that's easy to flatten when summarizing code into pseudocode. **Check this file on every audit, not just ones triggered by camera-specific changes** — viewer.rs is a natural place for small additions (a new debug-draw call, a new end-of-frame hook) that don't feel like "camera work" but still change the loop.

**`docs/overview.md`** — Contains the scene objects list and binary list. Goes stale when:
- New scene objects added to `src/scene/objects/`
- New binaries added to `src/bin/`
Fix: cross-check against `src/scene/objects/mod.rs` pub use list and `src/bin/` directory listing.

## Stable Files (low staleness risk)

**`docs/camera_and_rendering.md`** — Covers gotchas (macroquad spelling, FOV radians, depth buffer). Stable because these are macroquad quirks unlikely to change.

**`docs/animation_and_clock.md`** — Covers easing count, per-segment easing gotcha, and the sequential/parallel SceneBuilder API. The SceneBuilder section is medium-risk: go stale if method names change (`animate_seq`, `animate_for`, `parallel`, `wait`, `set_cursor`) or if the `animate_for` "requires prior keyframe" constraint is relaxed. Check `src/scene_builder.rs` `TrackBuilder::animate_for` assert on schema change.

**`docs/scene_and_properties.md`** — Covers flat mesh approach and draw call limits. Stable.

**`docs/testing.md`** — Object type list in SceneBuilder tests section. Could drift if new objects added without updating coverage list.

**`docs/agent_testing.md`** — Scenario builder API. Stable; API hasn't changed.

## Enumeration Drift (updated 2026-07-26)

Every hand-maintained list of object types found across the docs had drifted independently: README.md's scene objects table (11 of 16, table-form with descriptions), docs/overview.md's scene objects bullet (12 of 16), docs/testing.md's SceneBuilder coverage line (11 objects, claimed `Ring` coverage that doesn't exist), and the now-rewritten l_system_implementation.md preset list (this one was actually still accurate). Same root cause each time: a list added when N objects existed, never revisited as new objects (`LSystem`, `Polyline`, `Sprite`, `Turtle`, `VectorText`) were added to `src/scene/objects/`.

Fix pattern applied: where the doc doesn't need per-item descriptions (overview.md, testing.md), replaced the enumeration with a pointer to `src/scene/objects/` or the specific test/impl. Where per-item descriptions add real value (README's public-facing table), trimmed to a pointer anyway per AGENTS.md philosophy — descriptions were one-liners restating the type name, not worth the maintenance burden. **Future audits: don't just refresh these lists — ask whether the list needs to exist at all, since `src/scene/objects/mod.rs`'s `pub use` block is a strictly better source of truth than any doc copy.**

## What is NOT in the docs (intentionally)

- Individual object property names — these are readable from each object's `property_names()` impl
- Keyframe API details — TrackBuilder builder methods are readable from scene_builder.rs
- Individual easing function implementations — just the count and family names matter

## Tooling Conventions Documented in AGENTS.md

**`./scripts/validate.sh`** — Runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. Replaces the old bare three-command block in AGENTS.md "Build Checks Before Finishing". Added 2026-03-15.

**`.githooks/pre-commit`** — Calls `scripts/validate.sh`. Requires `git config core.hooksPath .githooks` once per clone. Documented in AGENTS.md "Build Checks Before Finishing".

Staleness risk: if validate.sh steps change, AGENTS.md description of what it runs will drift. The AGENTS.md section describes the steps inline rather than just pointing to the script — worth keeping current if steps change.

## Medium-Risk Files

**`docs/l_system_implementation.md`** — Covers engine design, module split, animatable properties, and preset table. Goes stale when:
- Animatable properties are added or renamed (check `src/scene/objects/l_system.rs` `property_names()`)
- New presets are added to the preset enum/list
- The three-way module split changes (`src/scene/l_system.rs` engine, `src/scene/polyline.rs` shared mesh/reveal/gradient utility, `src/scene/objects/l_system.rs` scene object)
Notable: preset table has default theta values for all presets — verify any new preset has a default assigned.

**Rewritten 2026-07-26** — this was the worst-offending doc in the repo: it was written as a pre-implementation spec (`apply_configuration`, type `LSystemConfiguration` with field `replacement_rules`, `get_lines(..., start_position: Vec2, ...)`) and never updated once the real code landed with different names (`apply_rules`, `LSystemConfig` with field `rules`, no `start_position` param — turtle always starts at origin). A "What NOT to Implement" section also actively told future readers not to build gradient coloring and bounding boxes — both of which now exist (`LSystem::with_colors`, `bounding_box()`). Rewritten in reference voice (describes what exists, not what to build).
**Lesson:** pre-implementation spec docs are a distinct staleness risk from "Future Work" sections (see below) — they don't announce themselves as speculative, so a stale spec reads as if it's still authoritative. When a doc describes function signatures, type names, or "do not implement X" in imperative voice, verify every named symbol against the actual source before trusting it, even if nothing marks it as a plan.

## Known Documentation Gaps

- No doc covering `AnimValue` variants (Float, Vec2, Vec3, Vec4, Bool, Transform2D, Mat4) and when to use each. Currently scattered in scene_and_properties.md and testing.md.
- No doc for `src/videos/` structure vs `src/examples/` — only mentioned briefly in module_layout.md.

## High-Risk Pattern: "Future Work" Sections

**`docs/vector_text.md`** had a "Future work" section listing LaTeX as not-yet-implemented. When the feature shipped, the section became actively misleading. Future audits should treat any "Future work" or "Out of scope" language as a red flag to verify against the actual codebase.

Fix pattern: replace speculative sections with concrete gotcha notes documenting what was hard-won during implementation (e.g., dvisvgm's compact path format, SVG Y-flip, coordinate normalization).

## LaTeX Pipeline Gotchas (added 2026-03-21, updated 2026-07-26)

The implementation in `src/scene/latex.rs` has several non-obvious behaviors worth knowing for future work:
- dvisvgm compact path format uses negative signs/decimal points as implicit separators — requires a custom tokenizer
- `<use>` elements reference `<path>` defs in `<defs>`; `<rect>` elements represent rules (fraction bars, etc.)
- Coordinates normalized by `PT_PER_EM = 10.0` (TeX default 10pt font) so 1 em ≈ 1 world unit
- `advance_x` is always 0.0 for LaTeX glyphs — layout from SVG positions, not character metrics
- Export binary: scene building happens inside `async export_main()` so GL context is available (Texture2D loading requires it)
- The dvisvgm invocation is `dvisvgm --no-fonts --exact -o <svg> <dvi>` (`src/scene/latex.rs:61`). `--exact` (short for `--exact-bbox`) makes dvisvgm compute each glyph's bounding box from its actual shape instead of the font's approximate TFM metrics — the parser reads that box (`parse_viewbox_min_x`) to normalize coordinates, so this flag is load-bearing for correct glyph positions, not just cosmetic. If this doc drifts again, check the `.args([...])` call directly rather than trusting a remembered flag list.

## Shared Line-Rendering Module (added 2026-07-26)

`src/scene/polyline.rs` is a shared utility (not object-specific) used by both `LSystem` (`src/scene/objects/l_system.rs`) and `Polyline` (`src/scene/objects/polyline.rs`). It owns `LineSegment`, `take_progress` (sub-segment-accurate progress reveal — lerps the boundary segment, doesn't just floor to whole segments), `draw_polyline_mesh` (chunked quad meshes, `MAX_SEGMENTS_PER_MESH = 833`), and gradient color sampling (delegates to `src/scene/color.rs::gradient_sample`). Any doc describing L-system or Polyline mesh-building should point here rather than re-describing the mesh logic per-object — both objects are thin wrappers now. If a third line-based object is added, check whether it also delegates here before assuming the "hand-built mesh per object" pattern in scene_and_properties.md still applies uniformly.
