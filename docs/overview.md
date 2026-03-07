# Rust Manim — Project Overview

## Summary

A manim-inspired animation engine built in Rust using macroquad for rendering. Supports real-time interactive scene inspection with a free camera and a scripted playback/export mode where camera and object properties are driven by a keyframe timeline.

## In Scope

- **Scene graph** with a trait-based object system (`SceneObject`) and sequential `ObjectId` handles.
- **Property system** using a string-keyed `AnimValue` enum. Objects implement an `Animatable` trait. The animation system drives objects generically through this interface.
- **Keyframe animation engine.** Tracks, timelines, easing functions, pure evaluation.
- **Clock / transport controls.** Play, pause, frame-step, scrub, variable speed, loop modes.
- **Scene objects:** Circle, Line, Rectangle, Polygon, Spiral — flat custom meshes on the XY plane. Text — screen-space overlay.
- **Two runtime modes:** Interactive (free camera) and Playback (timeline-driven camera, not yet implemented).
- **Camera wrapper** converting to macroquad's `Camera3D` in one place. Camera is animatable.
- **Debug overlay** with HUD, world-space helpers, camera log, snap-to-view, value inspector.
- **Scrub bar UI** at the bottom of the screen.
- **CLI export tool** (`cargo run --bin export`) renders the animation to MP4 via ffmpeg with vsync disabled for maximum speed.
- **Automated testing** covering math, animation, properties, clock, and orbit controller.

## Out of Scope (For Now)

- LaTeX rendering or rich math typesetting.
- Hot-reloadable scripting language (Lua, Rhai, etc.) — start with the Rust DSL, add scripting later if needed.
- Derive macro for `Animatable` — writing impls by hand for now (6 object types + Camera).
- GPU-accelerated or shader-based rendering beyond what macroquad provides.
- Audio synchronization.
- GUI editor for authoring keyframes (debug tools only, not an authoring UI).
- Spline or bezier-path interpolation for spatial tracks (start with per-component lerp + easing).
- `SceneBuilder` DSL with build-time property name validation.
- Custom font loading for Text objects (currently uses macroquad's default font).

## Key Design Decisions

- **Property system is string-keyed.** Flexible and generic, but typos are runtime errors. Will be mitigated by validating property names in a `SceneBuilder` at scene construction time.
- **Everything is 3D from day one.** Camera, transforms, and positions are all `Vec3` even if early objects are flat. Retrofitting 3D later is much harder.
- **Clock is the single source of truth for time.** Paused state is a distinct code path (not speed=0). Scrubbing and frame-stepping directly mutate `clock.current_time`. `timeline.apply` runs every frame regardless of play state.
- **Camera wraps macroquad.** The system owns a `Camera` struct with clean semantics. Conversion to macroquad's `Camera3D` happens in one function.
- **CLI export decouples from real time.** A separate binary (`src/bin/export.rs`) drives time synthetically at `1/fps` with vsync disabled. Pipes raw RGB frames directly to ffmpeg's stdin — no intermediate files.
- **Library + binary crate structure.** Shared code lives in `src/lib.rs`. The interactive viewer (`src/main.rs`) and CLI exporter (`src/bin/export.rs`) are separate binaries that import from the library.

## Related Docs

- [Scene & Properties](scene_and_properties.md) — scene graph, traits, property system
- [Animation & Clock](animation_and_clock.md) — keyframes, tracks, timeline, easing, clock
- [Camera & Rendering](camera_and_rendering.md) — camera, orbit controller, CLI export
- [Debug & UI](debug_and_ui.md) — debug overlay, scrub bar, value inspector
- [Module Layout](module_layout.md) — directory structure, main loop
- [Testing](testing.md) — testing strategy
