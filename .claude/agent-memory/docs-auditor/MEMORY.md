# Docs Auditor Memory Index

## Audit History
- [audit_2026_03_15.md](audit_2026_03_15.md) — First full audit; pseudocode accuracy issues in module_layout.md
- 2026-03-21 — LaTeX feature audit: updated vector_text.md (removed Future Work, added LaTeX constructors + gotchas), overview.md (removed LaTeX from Out of Scope, added VectorText to scene objects list), auto-memory MEMORY.md (binary count, VectorText constructors, validate.sh)
- 2026-04-02 — Sequential/parallel API audit: added "Sequential and Parallel Animation Authoring" section to animation_and_clock.md (cursor model, animate_for gotcha, parallel advance semantics, mixing modes, wait vs set_cursor); added pointer in scene_and_properties.md Design Notes; updated doc_patterns.md staleness risk for animation_and_clock.md
- 2026-07-26 — Applied a pre-verified accuracy audit (facts supplied by caller, not independently discovered): rewrote l_system_implementation.md (was a stale pre-implementation spec, worst offender in repo); fixed module_layout.md pseudocode (2nd drift — scrub_bar.draw_ticks, record_camera, conditional orbit re-derivation missing); fixed dolly_zoom location, render-target window-size claim, dvisvgm `--exact` flag, testing.md object-coverage claims (Ring not covered, Spiral has no round-trip test); trimmed README/overview/scene_and_properties enumerations to pointers at src/scene/objects/. See doc_patterns.md for lessons (pre-implementation-spec staleness, module_layout.md 2nd-drift, shared polyline.rs module, enumeration drift).

## Documentation Patterns
- [doc_patterns.md](doc_patterns.md) — What goes stale, what stays accurate, coverage gaps
