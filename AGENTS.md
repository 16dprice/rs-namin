# Agent Guidelines

## Project Documentation

Architecture docs live in `docs/`. These focus on gotchas, design rationale, and things not obvious from reading the code. For API details, read the source directly.

| Doc | Covers |
|-----|--------|
| [overview.md](docs/overview.md) | Project summary, scope, design decisions |
| [scene_and_properties.md](docs/scene_and_properties.md) | Rendering gotchas (flat meshes, draw call limits) |
| [animation_and_clock.md](docs/animation_and_clock.md) | Animation gotchas, clock design notes |
| [camera_and_rendering.md](docs/camera_and_rendering.md) | Camera/macroquad gotchas, CLI export rationale |
| [debug_and_ui.md](docs/debug_and_ui.md) | Debug overlay gotchas, roadmap |
| [module_layout.md](docs/module_layout.md) | Main loop ordering and why it matters |
| [testing.md](docs/testing.md) | Testing strategy |

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
cargo build
cargo test
cargo clippy -- -D warnings
```

All three must pass. Do not mark work as done with failing builds, tests, or lint warnings.

### Keep Docs in Sync with Code

Documentation in `docs/` describes what needs to be built. Once code has been written that satisfies a requirement described in a doc:

- Remove or replace the speculative/planning language with a brief reference to the actual implementation (e.g., module path, key types).
- Delete implementation-specific guidelines that are now encoded in the source code itself. The code is the source of truth once it exists.
- Keep high-level architectural context and rationale — remove step-by-step instructions that the code already embodies.

The goal is to prevent docs from drifting into stale "plans" that contradict the actual codebase.

### Code Style

- Follow standard Rust conventions (`rustfmt`, `clippy`).
- Prefer simple, direct solutions. Don't over-abstract or add features beyond what was requested.
- Keep functions short and focused. If a function needs a comment explaining what it does, consider renaming it or splitting it.
