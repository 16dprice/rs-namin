# Debug Overlay & UI

Implemented in `src/debug/`. All keybindings are configurable via `Keybindings` struct in `src/debug/keybindings.rs`.

## HUD (Heads-Up Display)

Screen-space overlay (toggle: F1) showing:

- Object count in the scene.
- Current time / duration, playback state, playback speed.
- Loop mode.

Camera info will be added once the camera system is implemented.

## Scrub Bar

Visual timeline bar at the bottom of the screen (toggle: F2):

- Shows playhead position along the timeline.
- Keyframe tick marks from all tracks.
- Time readout (current time / duration).
- Play/pause state indicator.
- Click-drag to scrub. Auto-pauses when dragging begins, resumes on release if was playing.

## Value Inspector

Right-side panel (toggle: F3) showing:

- All properties of every scene object (from `Animatable::property_names()`).
- Current interpolated values formatted by type.

## Transport Controls

Keybindings for playback (all configurable in `Keybindings`):

- Play/pause toggle (default: Space).
- Step forward/backward one frame (default: Right/Left arrows). Auto-pauses.
- Speed up/down by 2x (default: Up/Down arrows). Clamped to 0.125x–8x range.

## Not Yet Implemented

- **World-space debug draws** — ground grid, origin axes, orbit-target crosshair, per-object bounding boxes. Requires camera system.
- **Snap-to-view keys** — Blender-style numpad views. Requires camera system.
- **Camera state log** — ring buffer of camera snapshots. Requires camera system.

## Module Location

```
src/debug/
  mod.rs              DebugOverlay (HUD, toggle state, input handling, draw dispatch)
  keybindings.rs      Keybindings struct (all configurable key mappings)
  scrub_bar.rs        ScrubBar (visual timeline + drag-to-scrub)
  value_inspector.rs  ValueInspector (per-object property viewer)
```
