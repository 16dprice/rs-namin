# Agent Guidelines

## Project Documentation

Architecture docs live in `docs/`. Start with [docs/overview.md](docs/overview.md) for project summary, scope, and links to all other docs.

| Doc | Covers |
|-----|--------|
| [overview.md](docs/overview.md) | Project summary, scope, design decisions |
| [scene_and_properties.md](docs/scene_and_properties.md) | Scene graph, traits, property system |
| [animation_and_clock.md](docs/animation_and_clock.md) | Keyframes, tracks, timeline, easing, clock |
| [camera_and_rendering.md](docs/camera_and_rendering.md) | Camera, orbit controller, export pipeline |
| [debug_and_ui.md](docs/debug_and_ui.md) | Debug overlay, scrub bar, value inspector |
| [module_layout.md](docs/module_layout.md) | Directory structure, main loop |
| [testing.md](docs/testing.md) | Testing strategy |

## General Expectations

### Write Tests

Every code change must include corresponding tests. See [docs/testing.md](docs/testing.md) for the project's testing strategy. At minimum:

- New functions get unit tests covering typical inputs and edge cases.
- New traits or trait impls get round-trip and contract tests.
- Bug fixes include a regression test that would have caught the bug.

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
