# Agent Testing Infrastructure

Tools for automated agents to verify interactive behavior without a human in the loop.

## When to Use What

| What you're verifying | Tool | Needs GL context? |
|---|---|---|
| Input behavior (orbit direction, zoom speed, key toggles) | `ScriptedInput` + unit tests | No |
| Visual output (object positions, colors, rendering correctness) | `cargo run --bin snapshot` + read the PNG | Yes |
| Multi-frame interactions (drag sequences, animation playback) | Scenario runner (not yet implemented) | No |

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
```

The binary renders to an offscreen target and saves PNGs. Read the resulting PNG to visually inspect the output. Output defaults to `snapshot.png`; for multiple times, pass a directory path to `--output`.

Key files: `src/bin/snapshot.rs`, `src/render_util.rs`.

---

## Tier 3: Scenario Runner (Not Yet Implemented)

### Problem

`ScriptedInput` tests a single frame of input. Snapshots capture a single rendered frame. Neither can test multi-frame interactive sequences like "drag the mouse rightward for 30 frames and verify the camera orbited 90 degrees" or "press play, advance 2 seconds, then verify the animation reached the expected state."

### Design

A `Scenario` struct that describes a sequence of steps — inject input, advance N frames, assert state. Scenarios run in `cargo test` without a GL context.

### API

Define in `src/scenario.rs` (new file). Register in `src/lib.rs`.

```rust
pub struct Scenario {
    steps: Vec<Step>,
}

enum Step {
    RunFrames { count: u32, input: ScriptedInput },
    AssertCamera(Box<dyn Fn(&Camera) -> bool>, String),
    AssertOrbit(Box<dyn Fn(&OrbitController) -> bool>, String),
    AssertScene(Box<dyn Fn(&Scene) -> bool>, String),
}
```

Builder API:

```rust
impl Scenario {
    pub fn new() -> Self { ... }
    pub fn run_frames(mut self, count: u32, input: ScriptedInput) -> Self { ... }
    pub fn idle(mut self, count: u32) -> Self { ... }
    pub fn assert_camera(mut self, msg: &str, check: impl Fn(&Camera) -> bool + 'static) -> Self { ... }
    pub fn assert_orbit(mut self, msg: &str, check: impl Fn(&OrbitController) -> bool + 'static) -> Self { ... }
    pub fn assert_scene(mut self, msg: &str, check: impl Fn(&Scene) -> bool + 'static) -> Self { ... }
}
```

### Executor

`Scenario::run(self, scene, timeline, camera)` executes all steps. Each `RunFrames` step advances accumulated time by `count * input.frame_time()`, applies the timeline, and runs the orbit controller. Does not render — no GL context needed.

### Example

```rust
#[test]
fn orbit_90_degrees_right() {
    let (mut scene, timeline, mut camera) = my_scene::build();

    let drag_right = ScriptedInput::default()
        .with_mouse_button(MouseButton::Middle)
        .with_mouse_delta(vec2(0.05, 0.0));

    Scenario::new()
        .run_frames(60, drag_right)
        .assert_orbit("azimuth should be ~π/2", |o| {
            (o.azimuth - std::f32::consts::FRAC_PI_2).abs() < 0.2
        })
        .assert_camera("camera X should be positive", |c| c.position.x > 0.0)
        .run(&mut scene, &timeline, &mut camera);
}
```

### Implementation plan

```
src/scenario.rs (new: Scenario struct and executor)
src/lib.rs (register scenario module)
src/tests/scenarios.rs (new: scenario-based integration tests)
```

Place scenario-based tests in `src/tests/scenarios.rs`, registered from `src/tests/mod.rs`.
