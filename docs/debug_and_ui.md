# Debug Overlay & UI

All debug features are toggled with F-keys.

## HUD (Heads-Up Display)

Screen-space overlay showing:

- Camera position, target, distance, forward vector, FOV.
- Object count in the scene.
- Current time, playback state, playback speed.

## World-Space Debug Draws

Rendered in 3D, visible from the camera's perspective:

- **Ground grid** on the XZ plane.
- **Origin axes** — RGB = XYZ (red = X, green = Y, blue = Z).
- **Orbit-target crosshair** — shows where the camera is looking at in interactive mode.
- **Per-object bounding boxes** — wireframe AABBs around each scene object.

## Snap-to-View Keys

Following Blender numpad conventions:

- Front, right, top views (and their opposites).
- Perspective/orthographic toggle.

## Camera State Log

`CameraDebugLog` maintains a ring buffer of recent `CameraSnapshot` entries, each tagged with a trigger label (e.g., "orbit", "timeline:keyframe_3").

- Dump recent history to console on keypress.
- Useful for debugging camera jumps or unexpected movements.

## Value Inspector

When paused with an object selected:

- Shows all properties of the selected object (from `Animatable::property_names()`).
- Displays current interpolated values.
- Indicates which keyframe segment is active for each property.

## Scrub Bar

A visual timeline bar at the bottom of the screen:

- Shows playhead position along the timeline.
- Keyframe tick marks from all tracks.
- Time readout (current time / duration).
- Play/pause state indicator.
- Playback speed display.
- **Interaction:** click-drag to scrub. Auto-pauses when dragging begins.

## Module Location

```
src/debug/
  mod.rs              DebugOverlay (HUD, world-space draws, keybindings)
  scrub_bar.rs        ScrubBar (visual timeline + drag-to-scrub)
  value_inspector.rs  Per-object property viewer
  camera_log.rs       CameraDebugLog, CameraSnapshot, dump_recent()
```
