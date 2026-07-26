# GUI Roadmap

Two phases. **Phase 1** consolidates the four binaries (viewer, example picker, snapshot, export)
into one application with modes — same capabilities, no terminal round-trips, export runs in-app
with a progress indicator. **Phase 2** turns that application into a video designer: create
objects, keyframe their properties on a timeline, and send the result to render without writing
Rust. No DSL — the designer edits a serializable scene document through a typical
video-editor-style UI.

## Substrate (already done)

The refactors that make this tractable, and what each buys the GUI:

- **Unified scene registry** (`src/registry.rs`) — one `SceneEntry` list with names,
  descriptions, kinds, and build fns. The Library screen is a render of this list; export and
  snapshot already resolve any scene through it.
- **Offscreen render pipeline** (`render_util::OffscreenRenderer`) — render-any-scene-to-texture
  at any resolution, with readback. This is both "export as a screen" (feed frames to ffmpeg over
  many UI frames) and "viewport as a widget" (draw the scene into a texture sized to the panel
  egui gives us).
- **Design-space screen coordinates** (`DESIGN_WIDTH/HEIGHT`) — screen-space objects render
  identically in the viewer and at every export resolution, so the editing viewport is truthful.
- **`animatable!` property macro** — every object exposes `property_names()` and typed values
  through one mechanism, so a property inspector can be *generated* (names + `AnimValue` variant
  → widget type) instead of hand-built per object.
- **`InputProvider` seam** — one choke point to gate scene input when the UI wants the pointer.

## Technology choice (July 2026 research)

Version landscape: macroquad 0.4.14 (our exact lock), egui 0.35, **egui-macroquad 0.17.3**
(revived May 2025; pins egui 0.31.1 + macroquad 0.4.14 — matches our lockfile exactly).
egui-miniquad does the real work; its master already has an unreleased egui 0.33 bump. The
bindings historically lag egui by 2–4 minor versions.

- **macroquad's built-in `ui` module (`root_ui`)**: debug/menu tier only — no docking, tables,
  drag-and-drop, or widget ecosystem, plus open correctness bugs. Not viable for a timeline
  editor. (Precedent: FishFight/Jumpy built a macroquad level editor, then rewrote on Bevy.)
- **egui overlay on macroquad** (egui-macroquad): the standard path. Integration gotchas are
  known: call `ui()` first, draw the scene, `egui_macroquad::draw()` last; gate scene input on
  `egui_ctx.wants_pointer_input()/wants_keyboard_input()`; a known high-DPI issue is worked
  around with `set_pixels_per_point`.
- **egui widget ecosystem** (needs egui 0.34/0.35, i.e. not reachable from the 0.31 binding):
  `egui-keyframe` 0.1.0 (Feb 2026) is almost exactly our Phase-2 UI — `CurveEditor` with bezier
  handles + `DopeSheet` with a property tree — but brand new (5 commits); treat as a vendorable
  starting point. `egui_dock`/`egui_tiles` (docking), `egui_plot` (easing curves), `egui-snarl`
  (node graphs) are all actively maintained.
- **Fallback if the bindings stall**: egui-macroquad's shim is ~130 lines over public macroquad
  API (`get_internal_gl`, `register_input_subscriber`, `repeat_all_miniquad_input`); vendoring
  both crates and bumping egui is days, not weeks.

**Recommendation** (ranked):
1. **Phase 1 on egui-macroquad 0.17.3, accepting egui 0.31.** Zero renderer changes; covers
   modes, buttons, progress bars, pickers, inspectors. A first timeline strip can be hand-painted
   with egui's painter API (we own the `Timeline`/keyframe model, and a dope-sheet strip is
   tractable).
2. **When the designer gets serious: vendor egui-miniquad + the shim, bump to egui 0.35** to
   unlock egui-keyframe/egui_dock/egui_plot.
3. Porting rendering to eframe+glow paint callbacks is the escape hatch if the app ever becomes
   UI-first; iced/Dioxus are wrong-shaped for a GL-canvas editor. Not needed now.

## Phase 1 — integrated application shell

One binary (`rs-namin`), one window, a mode enum:

```
enum AppMode { Library, Viewer { entry }, Export { job } }
```

- **Library**: scene list from `registry::SCENES` (kind badges: example/video/scratch), click →
  Viewer. Replaces the `example` binary's terminal picker.
- **Viewer**: today's viewer loop (orbit camera, clock, scrub bar, overlays) wrapped in egui
  chrome: transport controls, camera-mode toggle, "Snapshot frame" button (writes PNG via
  `OffscreenRenderer` — replaces the `snapshot` binary's common case), "Export…" button.
- **Export**: a form (resolution/fps/encoding/range/audio — the same config struct the CLI
  export already has) plus a progress bar. Rendering runs *incrementally*: each UI frame renders
  a batch of export frames into the `OffscreenRenderer` and feeds ffmpeg's stdin, so the UI stays
  live and shows progress/cancel. On completion, link to the output file.
- The CLI binaries stay (they're thin wrappers over the same registry + pipeline; CI/scripting
  still wants them).

Milestones (each is a well-scoped agent task):
- **M1.1** ✅ **DONE** — egui-macroquad 0.17.3 integrated (resolved cleanly against our locked
  macroquad 0.4.14). `src/ui.rs` owns the egui frame protocol (`layout` first / `draw` last);
  `UiGatedInput` in `src/input.rs` gates scene input on `wants_pointer/keyboard_input`; the F1
  HUD is now an egui window with working transport controls (play/pause, log-scale speed
  slider, camera-follow checkbox). Verified live via the new `RS_NAMIN_FRAME_DUMP` frame-dump
  utility (see docs/agent_testing.md) — no high-DPI artifacts on a standard display.
  Known gap carried to M1.3: the scrub bar reads macroquad input directly (pre-existing), so
  it isn't UI-gated; it gets replaced by an egui slider in M1.3 anyway.
- **M1.2** ✅ **DONE** — `src/app.rs` shell: `AppMode { Library, Viewer(ViewerMode) }` with
  per-frame dispatch and `UiRequest`-driven transitions; the viewer loop became
  `ViewerMode::frame`. The library screen lists all registry scenes (kind badge +
  description) and replaces the `example` binary's terminal picker; the viewer got a top app
  bar (`< Library`, scene name, shortcut hints) and Esc navigates back. Opening a scene
  rebuilds it fresh, inside the GL context. `cargo run` starts in the viewer on `my_scene`;
  `cargo run --bin example` starts in the library. Design note from review: egui's stock look
  is fine — prioritize flow over visual polish in M1.3+.
- **M1.3** ✅ **DONE** — bottom transport panel (play/pause, frame step, loop-mode combo,
  log-scale speed slider, monospace time readout) with a full-width scrub slider that paints
  keyframe ticks from the timeline; scrub pause/resume semantics live in the pure
  `apply_scrub` fn (unit-tested). The hand-drawn scrub bar and text value inspector are
  deleted — the inspector is an egui window (F3) showing live per-object property values via
  the `Animatable` surface, and the last input-gating gap is closed (all interactive chrome
  is egui now; only F4/F6 viewport overlays draw with macroquad). App-bar Snapshot button
  renders the scene UI-free through `OffscreenRenderer` into `snapshots/` with one-frame-lag
  readback and a transient status message. F2 now toggles the transport bar.
- **M1.4** ✅ **DONE** — `AppMode::Export` (`ExportMode` in `src/export.rs`): a Configure
  form (resolution/fps/CRF-or-bitrate/range/audio/output) with a live scene preview behind
  it, an incremental Render phase (one export frame per UI frame, piped to ffmpeg's stdin
  with one-frame-lag readback; progress bar + cancel; UI stays live and shows the frame
  being encoded), and a Done screen. Reached via the viewer's Export… button; the export
  core (presets, ffmpeg args, frame math) moved to lib `src/export.rs`, shared with the CLI
  binary. End-to-end verified: in-app export produced a valid 1080p h264 MP4 (ffprobe
  checked). Known trade-off: the app window is vsync-paced, so in-app export runs at
  ~display-refresh fps per second of wall clock (the CLI export sets swap_interval 0 and
  remains the fast path); batching multiple render targets per UI frame is the future
  optimization if it matters.
- **M1.5** Polish pass agreed from the design-language conversation (theme, layout, shortcuts).

## Phase 2 — in-app video designer

The designer edits a **scene document** instead of Rust code:

```
SceneDoc {
  objects: [ { id, type: "disk", params: {...}, initial: {position, radius, color, ...} } ],
  tracks:  [ { object_id, property, keyframes: [ { t, value, easing } ] } ],
  camera tracks, duration, design notes
}
```

Prerequisite refactors (small, high-leverage, do before UI work):
- **Easing as data**: `EasingFn` is a bare `fn(f32) -> f32` pointer — it can't be serialized,
  enumerated, or shown in a picker (docs already flag this as tech debt; the dolly-zoom hack in
  `torus_knot.rs` exists because closures don't fit either). Replace with
  `enum Easing { Linear, QuadIn, …, CubicBezier(p1, p2) }` implementing `eval(t)`. The
  bezier variant subsumes custom curves and is what a curve editor edits.
- **Spawnable object types**: a `SceneDoc` needs to construct objects from data. Add an object-
  type registry: name → `spawn(params) -> Box<dyn SceneNode>` + a params schema. Start with the
  data-friendly objects (Disk, Ring, Rectangle, Polygon, Arc, Arrow, Line, Text, Spiral, Torus);
  Tube (point lists), LSystem (rule config), VectorText (fonts/LaTeX) get schemas incrementally.
- **Serde**: derive on `AnimValue`, keyframes, tracks, the doc itself. Format: RON or JSON files
  in a project directory; they become `SceneKind::Doc` entries in the registry so the Library,
  viewer, and export pipeline all pick them up with zero extra plumbing.

UI build-out (iterative; order negotiable after the design conversation):
- **M2.1** SceneDoc + easing enum + serde + doc-backed registry entries (no UI yet — a
  hand-written .ron file renders and exports).
- **M2.2** Object palette + property inspector: add/select objects, edit initial properties via
  widgets generated from `property_names()` + `AnimValue` variant (Float → drag value, Vec3 →
  three drags, Vec4 color → color picker, Bool → checkbox).
- **M2.3** Dope sheet: tracks × time grid, add/move/delete keyframes, scrub-synced playhead,
  per-keyframe easing picker. Hand-painted on egui 0.31, or vendor-bump first and adapt
  egui-keyframe (decide at the time).
- **M2.4** Curve editor for eased segments (egui_plot or the egui-keyframe CurveEditor).
- **M2.5** Viewport interaction: click-select (bounding boxes exist), drag-move writing back to
  initial properties / keyframes.
- **M2.6** "Send to render": export form pre-filled from the doc; runs the M1.4 pipeline.

## Open decisions for the design-language conversation

- Layout: left object palette / right inspector / bottom timeline (Blender-style) vs. minimal
  chrome with floating panels. Docking (egui_dock) implies the vendor-bump earlier.
- Theming: egui's default dark vs. a custom theme; how much visual identity matters now.
- Keyboard-first (Space/F-keys carry over from the viewer) vs. mouse-first affordances.
- What happens to the debug overlays — fold into GUI panels (value inspector, camera log) or
  keep as F-key toggles for "engine dev mode".
- Whether `my_scene.rs` remains the scratchpad or Phase 2 docs replace that workflow entirely.

## Sequencing

M1.1 → M1.2 → M1.3/M1.4 (parallelizable) → design conversation → M1.5 → M2.1 → M2.2 → M2.3 →
M2.4/M2.5/M2.6. M1.1 and M2.1 are the risk-retiring spikes; everything after M2.1 is UI labor on
a stable data model. Each milestone is sized for delegation to a coding agent with this document
plus the relevant module docs as context.
