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

**`docs/camera_and_rendering.md`** — Covers gotchas (macroquad spelling, FOV radians, depth buffer). The macroquad-quirk content is stable, but as of 2026-07-26 it also documents `OffscreenRenderer` and the design-canvas screen-space convention (see below) — those are real architecture, not macroquad trivia, so re-check this file whenever `render_util.rs` changes, not just for camera-specific work.

**`docs/animation_and_clock.md`** — Covers easing count, per-segment easing gotcha, and the sequential/parallel SceneBuilder API. The SceneBuilder section is medium-risk: go stale if method names change (`animate_seq`, `animate_for`, `parallel`, `wait`, `set_cursor`) or if the `animate_for` "requires prior keyframe" constraint is relaxed. Check `src/scene_builder.rs` `TrackBuilder::animate_for` assert on schema change.

**`docs/scene_and_properties.md`** — Covers flat mesh approach, draw call limits, and (as of 2026-07-26) the `animatable!` macro and property conventions (`progress`, `rotation` vs `orientation`, float-as-int properties). No longer purely stable — the macro/MeshBuilder sections should be re-checked whenever `src/scene/traits.rs` or `src/scene/mesh.rs` change, since both are now single choke points many objects depend on.

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

## Shared Line-Rendering Module (added 2026-07-26, updated 2026-07-26 second pass)

`src/scene/polyline.rs` is a shared utility (not object-specific) used by both `LSystem` (`src/scene/objects/l_system.rs`) and `Polyline` (`src/scene/objects/polyline.rs`). It owns `LineSegment`, `take_progress` (sub-segment-accurate progress reveal — lerps the boundary segment, doesn't just floor to whole segments), `draw_polyline_mesh` (one quad per segment), and gradient color sampling (delegates to `src/scene/color.rs::gradient_sample`). Any doc describing L-system or Polyline mesh-building should point here rather than re-describing the mesh logic per-object — both objects are thin wrappers now. If a third line-based object is added, check whether it also delegates here before assuming the "hand-built mesh per object" pattern in scene_and_properties.md still applies uniformly.

**`MAX_SEGMENTS_PER_MESH = 833` is gone** — chunking was extracted into `src/scene/mesh.rs::MeshBuilder` (see next section) and `draw_polyline_mesh` now just calls `mb.quad(...)` per segment. If you ever see a doc citing a per-object chunk-size constant (`MAX_DOTS_PER_MESH`, `MAX_RINGS_PER_CHUNK`, `MAX_SEGMENTS_PER_MESH`), it's stale — `grep -rn "MAX_.*_PER_MESH\|MAX_.*_PER_CHUNK" src/` to confirm before trusting a doc's claim about chunk sizes.

## MeshBuilder — Central Mesh-Chunking Module (added 2026-07-26 second pass)

`src/scene/mesh.rs::MeshBuilder` replaced every object's hand-rolled draw-call chunking (Spiral's dot chunking, Ring/Torus/Tube's row-strip chunking, polyline's segment chunking). Objects now implement `fn build(&self, mb: &mut MeshBuilder)` using `mb.quad`/`mb.fan`/`mb.strip`/`mb.primitive`, and `MeshBuilder` owns `MAX_VERTICES_PER_MESH`/`MAX_INDICES_PER_MESH` (10k/5k) in one place, auto-flushing into a new `Mesh` (with rebased indices) whenever the next primitive wouldn't fit. `VectorText` is the one exception — it tessellates via lyon and does its own chunking with a vertex-remapping pass, since lyon hands back a whole contour's buffer at once rather than one primitive at a time.

**Doc impact:** any doc that used to say "hand-build a `build_mesh(&self) -> Mesh` method" (old pattern, e.g. `scene_and_properties.md`) or listed per-object chunk constants is now describing dead code. `MeshBuilder` has its own unit tests (chunk triggers, index rebasing, fan/strip assembly, oversized-primitive panic) — per-object mesh tests only need to assert vertex/index *counts*, not chunking behavior itself; that distinction is worth keeping in `testing.md` if it drifts again.

## `animatable!` Macro (added 2026-07-26 second pass)

`Animatable` impls are now generated by a macro (`src/scene/traits.rs`) from one `field: Variant` list per object, instead of hand-written parallel `get`/`set`/`property_names` methods. Two behaviors worth knowing:
- `set` on an unknown property or wrong `AnimValue` variant is a `debug_assert!` — panics in debug builds, silent no-op in release (because `Timeline::apply` calls `set` unconditionally every frame; a release panic there would crash playback mid-render).
- `Turtle` (`src/scene/objects/turtle.rs`) is the **only** hand-written `Animatable` impl left, because its `set("progress", ...)` has side effects (derives `position`/`rotation` from a path, re-syncs a child `Sprite`) that a macro-generated field assignment can't express. If a future object needs side-effecting setters, `Turtle` is the reference pattern.
- A shared test helper, `traits::test_support::assert_property_roundtrip`, replaced every object's individual round-trip test body — it perturbs each declared property and asserts `set`/`get` round-trips, plus checks an unknown name returns `None`. All macro-based objects call this one helper now; only `Turtle` has bespoke round-trip tests. **If you add a new object and its round-trip test doesn't call this helper, that's a signal something's off** (either it should be macro-based, or its hand-written impl needs its own tests like Turtle's).

## Unified Scene Registry (added 2026-07-26 second pass)

`src/registry.rs` replaced the old two-registry split (`src/videos::VIDEOS` + `src/examples::EXAMPLES`) with one `SceneEntry` list (name/description/`SceneKind`/build fn/audio) covering examples, videos, and the scratch scene (`my_scene`). `src/examples/mod.rs` and `src/videos/mod.rs` are now just `pub mod` declarations.

**Non-obvious asymmetry worth re-checking on every future audit of this area:** `example`, `snapshot`, and `export` all resolve scenes by name through `registry::find`/`registry::SCENES` — but the interactive viewer (`src/main.rs`) does **not**; it always calls `my_scene::build()` directly rather than looking itself up in the registry, even though `my_scene` is also a registered `Scratch`-kind entry. The registry module's own doc comment claims "the viewer, snapshot, export, and picker binaries all resolve scene names against the same list" — that's *aspirational/inaccurate* about the viewer; verify against `src/main.rs` directly rather than trusting that comment if this changes again. `docs/gui_plan.md` (written more carefully) only credits "export and snapshot" with resolving through the registry — a useful cross-check.

## Design-Space Screen Canvas + OffscreenRenderer (added 2026-07-26 second pass)

`render_util::OffscreenRenderer` (`src/render_util.rs`) now owns the two-pass offscreen render pipeline (depth-buffered target, 3D world pass, design-space screen pass, RGB/RGBA readback) shared by `export` and `snapshot`. `docs/camera_and_rendering.md`'s old Render Targets section had inline code snippets duplicating this logic — replaced with prose pointing at the type, since the code now self-documents.

New convention: screen-space objects (`Text`) are authored in pixels against a **fixed 1280x720 design canvas** (`DESIGN_WIDTH`/`DESIGN_HEIGHT` in `render_util.rs`), not the actual window/render-target resolution. `render_util::screen_space_camera(render_target)` maps that canvas onto whatever it's given. This makes screen-space content WYSIWYG across the viewer and every export resolution. **Consequence for `module_layout.md`'s frame-loop pseudocode:** the viewer is now a *three*-pass loop, not two — 3D world pass, then a separate design-space screen pass (`set_camera(screen_space_camera(None))`), then `set_default_camera()` for the debug overlay in real window pixels. A doc describing this as "two-pass: world then screen+UI together" is stale; check `src/viewer.rs` render-pass ordering specifically whenever `render_util.rs` or `viewer.rs` changes, in addition to the usual module_layout.md frame-loop check.

## Multi-PR Refactor Audits (added 2026-07-26 second pass)

This session's trigger was different from prior audits: four coordinated refactor PRs landed together (property-macro generation, registry unification, OffscreenRenderer extraction, MeshBuilder extraction) rather than one feature. Useful approach for this shape of task: read the actual diff/git log first (`git log --oneline`) to get commit-level scope, then verify each named claim (function names, behavior, file locations) against current source before touching any doc — several of the caller's own claims in the task prompt were exactly right once verified (e.g. Bool lerp semantics, Tube's `with_closed` rename), which meant no doc needed touching for those; only fix what's actually wrong or actually missing, don't pad docs to "cover" every item in a change list.
