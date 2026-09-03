# Agent Testing Infrastructure

Tools for automated agents to verify interactive behavior without a human in the loop.

## When to Use What

| What you're verifying | Tool | Needs GL context? |
|---|---|---|
| Input behavior (orbit direction, zoom speed, key toggles) | `ScriptedInput` + unit tests | No |
| Visual output (object positions, colors, rendering correctness) | `cargo run --bin snapshot` + read the PNG | Yes |
| Multi-frame interactions (drag sequences, animation playback) | `Scenario` runner + integration tests | No |
| The live viewer including UI chrome (egui panels, overlays) | `RS_NAMIN_FRAME_DUMP` + read the PNG | Yes |

### Behavioral testing with `ScriptedInput`

Use this for anything involving mouse or keyboard input. Construct a `ScriptedInput` with the desired state, pass it to `OrbitController::update()` or `DebugOverlay::handle_input()`, and assert the resulting state.

```rust
use crate::input::ScriptedInput;

let input = ScriptedInput::default()
    .with_mouse_button(MouseButton::Middle)
    .with_mouse_delta(vec2(0.01, 0.0));

orbit.update(&mut cam, &input);
assert!(orbit.azimuth > initial_azimuth);
```

Key files: `src/input.rs` (trait + both impls), `src/camera/orbit.rs` (orbit tests), `src/debug/mod.rs` (overlay tests).

When writing new input-handling code, always accept `&dyn InputProvider` rather than calling macroquad functions directly. This keeps the code testable. See existing tests in `src/camera/orbit.rs` for patterns covering direction, clamping, speed, and no-input stability.

### Visual verification with snapshots

Use this when you need to see what the scene actually looks like — verifying a new object renders correctly, checking that a rendering change didn't break things, or debugging visual issues.

```sh
cargo run --bin snapshot                                           # t=0, default size
cargo run --bin snapshot -- --time 1.5 --output frame.png          # specific time
cargo run --bin snapshot -- --times 0,0.5,1.0,2.0 --output frames/ # multiple frames
cargo run --bin snapshot -- --width 640 --height 360               # custom resolution
cargo run --bin snapshot -- --scene torus --time 1.0               # specific example scene
```

By default, snapshot renders `my_scene`. Use `--scene NAME` to render a named example from `src/examples/`.

The binary renders to an offscreen target and saves PNGs. Read the resulting PNG to visually inspect the output. Output defaults to `snapshot.png`; for multiple times, pass a directory path to `--output`.

Key files: `src/bin/snapshot.rs`, `src/render_util.rs`.

### Viewer frame dumps (UI chrome included)

The snapshot binary renders scenes offscreen and never runs the egui UI. To verify the
*viewer* itself — egui panels, debug overlays, layering — run the real app with
`RS_NAMIN_FRAME_DUMP="path.png@N"`: it saves frame N of the live window's framebuffer to
`path.png` and exits. Do not screen-capture the desktop instead (`xwd`/`gnome-screenshot`
grab whatever overlaps the window, including the user's other windows).

```sh
RS_NAMIN_FRAME_DUMP=/tmp/viewer.png@30 cargo run                      # default scene
RS_NAMIN_SCENE=demo RS_NAMIN_FRAME_DUMP=/tmp/editor.png@30 cargo run  # any scene (docs open with the editor)
RS_NAMIN_PREVIEW=1 RS_NAMIN_SCENE=demo RS_NAMIN_FRAME_DUMP=/tmp/p.png@30 cargo run  # chrome-free export preview
```

Key file: `src/viewer.rs` (`frame_dump_spec`).

---

### Multi-frame scenarios with `Scenario`

Use this for testing multi-frame interactive sequences — drag the mouse for 60 frames and verify orbit angle, zoom to min distance, pan then orbit, etc. Scenarios run in `cargo test` without a GL context.

```rust
use crate::input::ScriptedInput;
use crate::scenario::Scenario;

let drag_right = ScriptedInput::default()
    .with_mouse_button(MouseButton::Middle)
    .with_mouse_delta(vec2(0.008, 0.0));

Scenario::new()
    .run_frames(60, drag_right)
    .assert_orbit("azimuth should be ~π/2", |o| {
        (o.azimuth - std::f32::consts::FRAC_PI_2).abs() < 0.1
    })
    .assert_camera("camera X should be positive", |c| c.position.x > 0.0)
    .run(&mut scene, &timeline, &mut camera);
```

Builder methods: `run_frames(count, input)`, `idle(count)`, `assert_camera(msg, fn)`, `assert_orbit(msg, fn)`, `assert_scene(msg, fn)`. The executor advances time, applies the timeline, and updates the orbit controller each frame.

Key files: `src/scenario.rs` (struct + executor), `src/tests/scenarios.rs` (integration tests).

**Mouse delta calibration:** `mouse_delta` is in macroquad's -2..2 range. Pixel delta = `raw * screen_width * 0.5`. Azimuth per frame = `pixel_delta * orbit_speed(0.005)`. For ~π/2 over 60 frames, use `vec2(0.008, 0.0)`.
