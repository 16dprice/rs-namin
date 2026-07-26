# Agent Guidelines

## Project Documentation

Architecture docs live in `docs/`. These focus on gotchas, design rationale, and things not obvious from reading the code. For API details, read the source directly.

| Doc | Covers |
|-----|--------|
| [overview.md](docs/overview.md) | Project summary, scope, design decisions |
| [scene_and_properties.md](docs/scene_and_properties.md) | Rendering gotchas (flat meshes, draw call limits) |
| [animation_and_clock.md](docs/animation_and_clock.md) | Animation gotchas, clock design notes |
| [camera_and_rendering.md](docs/camera_and_rendering.md) | Camera/macroquad gotchas, CLI export rationale |
| [debug_and_ui.md](docs/debug_and_ui.md) | Debug overlay gotchas |
| [module_layout.md](docs/module_layout.md) | Main loop ordering and why it matters |
| [vector_text.md](docs/vector_text.md) | Vector text rendering design (bezier-based write-on animation) |
| [l_system_implementation.md](docs/l_system_implementation.md) | L-system engine design (string rewriting, turtle graphics, presets) |
| [testing.md](docs/testing.md) | Testing strategy |
| [agent_testing.md](docs/agent_testing.md) | Input abstraction, snapshot capture, scenario runner |

## General Expectations

### Write Tests

Every code change must include corresponding tests. See [docs/testing.md](docs/testing.md) for the project's testing strategy. At minimum:

- New functions get unit tests covering typical inputs and edge cases.
- New traits or trait impls get round-trip and contract tests.
- Bug fixes include a regression test that would have caught the bug.

### Test Organization

- **Unit tests** go inline at the bottom of the file they test, in a `#[cfg(test)] mod tests { ... }` block.
- **Integration tests** that exercise multiple modules together go in `src/tests/` as separate files, registered from `src/tests/mod.rs`.
- Do not create separate `_tests.rs` files alongside source files.

### Build Checks Before Finishing

Before considering any task complete, run:

```sh
./scripts/validate.sh
```

This runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` with a clean pass/fail summary. All checks must pass. Do not mark work as done with failing builds, tests, or lint warnings.

A pre-commit hook in `.githooks/pre-commit` runs this automatically on every commit. New clones must activate it once:

```sh
git config core.hooksPath .githooks
```

### Keep Docs in Sync with Code

Documentation in `docs/` describes what needs to be built. Once code has been written that satisfies a requirement described in a doc:

- Remove or replace the speculative/planning language with a brief reference to the actual implementation (e.g., module path, key types).
- Delete implementation-specific guidelines that are now encoded in the source code itself. The code is the source of truth once it exists.
- Keep high-level architectural context and rationale — remove step-by-step instructions that the code already embodies.

The goal is to prevent docs from drifting into stale "plans" that contradict the actual codebase.

### Verifying Visual and Interactive Behavior

This project is visual — many bugs only manifest on screen. Use the tools in [docs/agent_testing.md](docs/agent_testing.md) to verify your work:

- **Input-handling changes** (orbit controls, keybindings, mouse interactions): Write tests using `ScriptedInput` from `src/input.rs`. All code that reads mouse/keyboard input must accept `&dyn InputProvider` — never call macroquad input functions directly. See existing tests in `src/camera/orbit.rs` for patterns.
- **Rendering changes** (new scene objects, visual tweaks, camera changes): Run `cargo run --bin snapshot -- --time <T> --output snapshot.png` and read the resulting PNG to verify the scene looks correct. Use multiple `--times` to check different points in the animation.
- **Any change you're unsure about visually**: Take a snapshot before and after your change to confirm nothing broke.

### Version Control

Always commit `.claude/` directory contents (settings, agent definitions, agent memory) alongside other changes. These files are part of the project and should stay in sync with the remote.

### Code Style

- Follow standard Rust conventions (`rustfmt`, `clippy`).
- Prefer simple, direct solutions. Don't over-abstract or add features beyond what was requested.
- Keep functions short and focused. If a function needs a comment explaining what it does, consider renaming it or splitting it.
