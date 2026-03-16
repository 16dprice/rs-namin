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

**`docs/overview.md`** — Contains the scene objects list and binary list. Goes stale when:
- New scene objects added to `src/scene/objects/`
- New binaries added to `src/bin/`
Fix: cross-check against `src/scene/objects/mod.rs` pub use list and `src/bin/` directory listing.

## Stable Files (low staleness risk)

**`docs/camera_and_rendering.md`** — Covers gotchas (macroquad spelling, FOV radians, depth buffer). Stable because these are macroquad quirks unlikely to change.

**`docs/animation_and_clock.md`** — Covers easing count and per-segment easing gotcha. Stable.

**`docs/scene_and_properties.md`** — Covers flat mesh approach and draw call limits. Stable.

**`docs/testing.md`** — Object type list in SceneBuilder tests section. Could drift if new objects added without updating coverage list.

**`docs/agent_testing.md`** — Scenario builder API. Stable; API hasn't changed.

## What is NOT in the docs (intentionally)

- Individual object property names — these are readable from each object's `property_names()` impl
- Keyframe API details — TrackBuilder builder methods are readable from scene_builder.rs
- Individual easing function implementations — just the count and family names matter

## Tooling Conventions Documented in AGENTS.md

**`./scripts/validate.sh`** — Runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. Replaces the old bare three-command block in AGENTS.md "Build Checks Before Finishing". Added 2026-03-15.

**`.githooks/pre-commit`** — Calls `scripts/validate.sh`. Requires `git config core.hooksPath .githooks` once per clone. Documented in AGENTS.md "Build Checks Before Finishing".

Staleness risk: if validate.sh steps change, AGENTS.md description of what it runs will drift. The AGENTS.md section describes the steps inline rather than just pointing to the script — worth keeping current if steps change.

## Known Documentation Gaps

- No doc covering `AnimValue` variants (Float, Vec2, Vec3, Vec4, Bool, Transform2D, Mat4) and when to use each. Currently scattered in scene_and_properties.md and testing.md.
- No doc for `src/videos/` structure vs `src/examples/` — only mentioned briefly in module_layout.md.
