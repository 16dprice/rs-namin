# Rust Manim — Project Overview

## Summary

A manim-inspired animation engine built in Rust using macroquad for rendering. Supports real-time interactive scene inspection with a free camera and a scripted playback/export mode where camera and object properties are driven by a keyframe timeline.

## In Scope

- **Scene graph** with a trait-based object system (`SceneObject`) and sequential `ObjectId` handles.
- **Property system** using a string-keyed `AnimValue` enum. Objects implement an `Animatable` trait. The animation system drives objects generically through this interface.
- **Keyframe animation engine.** Tracks, timelines, easing functions, pure evaluation.
- **Clock / transport controls.** Play, pause, frame-step, scrub, variable speed, loop modes.
- **Scene objects:** Circle, Line, Rectangle, Polygon, Spiral, Arc, Arrow, Torus — flat custom meshes on the XY plane (Torus is a true 3D mesh). Text — screen-space overlay.
- **Two runtime modes:** Interactive (free orbit camera) and Timeline (keyframe-driven camera, toggled with F5).
- **Camera wrapper** converting to macroquad's `Camera3D` in one place. Camera is animatable.
- **Debug overlay** with HUD, world-space helpers, value inspector, scrub bar, orbit-target crosshair, bounding boxes, snap-to-view, camera state log.
- **CLI export tool** (`cargo run --bin export`) renders the animation to MP4 via ffmpeg, with optional audio muxing.
- **Example runner** (`cargo run --bin example`) presents an interactive picker for example scenes.
- **Automated testing** covering math, animation, properties, clock, orbit controller, and export.
- **SceneBuilder DSL** for constructing scenes with validated property names and types. See `src/scene_builder.rs`.
- **Agent testing infrastructure** — input abstraction (`InputProvider` trait + `ScriptedInput`), headless snapshot capture (`cargo run --bin snapshot`), and a scenario runner for multi-frame integration tests. See [agent_testing.md](agent_testing.md).

## Out of Scope (For Now)

- LaTeX rendering or rich math typesetting.
- Hot-reloadable scripting language (Lua, Rhai, etc.).
- Derive macro for `Animatable` — writing impls by hand for now.
- GPU-accelerated or shader-based rendering beyond what macroquad provides.
- Audio synchronization (the CLI export can mux an audio file into the MP4, but there is no beat-sync or timeline-aware audio engine).
- GUI editor for authoring keyframes (debug tools only, not an authoring UI).
- Spline or bezier-path interpolation for spatial tracks (start with per-component lerp + easing).
- Custom font loading for Text objects (currently uses macroquad's default font).

## Key Design Decisions

- **Property system is string-keyed.** Flexible and generic, but typos are runtime errors. Mitigated by `SceneBuilder` which validates property names and types at scene construction time. See `src/scene_builder.rs`.
- **Everything is 3D from day one.** Camera, transforms, and positions are all `Vec3` even if early objects are flat. Retrofitting 3D later is much harder.
- **Clock is the single source of truth for time.** Paused state is a distinct code path (not speed=0). `timeline.apply` runs every frame regardless of play state.
- **CLI export decouples from real time.** A separate binary drives time synthetically with vsync disabled. See [camera_and_rendering.md](camera_and_rendering.md) for why.
- **Library + binary crate structure.** Shared code lives in `src/lib.rs`. Four binaries: interactive viewer (`src/main.rs`), CLI exporter (`src/bin/export.rs`), snapshot tool (`src/bin/snapshot.rs`), and example runner (`src/bin/example.rs`).
