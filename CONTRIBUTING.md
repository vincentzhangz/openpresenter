# Contributing to OpenPresenter

Thanks for your interest in contributing! This guide covers how to build, test,
and submit changes.

## Getting Started

1. Fork the repository and clone your fork.
2. Install the prerequisites listed in the [README](README.md#prerequisites)
   (Rust 1.97+, FFmpeg, pkg-config, and optionally the NDI SDK).
3. Build:

   ```bash
   # With NDI output (default feature)
   cargo build

   # Without the NDI SDK (stub mode) — also what CI runs for the no-default path
   cargo build --no-default-features
   ```

## Development Workflow

Before opening a pull request, make sure the following all pass locally:

```bash
cargo fmt --check            # formatting
cargo clippy --no-default-features -- -D warnings   # lint, warnings are errors
cargo test                   # unit + integration tests (with coverage via llvm-cov)
cargo build --no-default-features   # ensure the NDI-less path still compiles
```

Run the app with:

```bash
cargo run
```

## Branching & Pull Requests

- Create a topic branch off `main` (e.g. `fix/ndi-reconnect`, `feat/stage-display`).
- Keep commits focused and write clear messages.
- Open a PR against `main` with a short description of the change and the
  motivation. Link any related issues.
- CI must be green (fmt, clippy, test) before a PR can be merged.

## Code Conventions

OpenPresenter follows a layered architecture (see the [README](README.md#architecture)):

- **`domain/`** holds the pure data model — no I/O, no `iced`.
- **`services/`** contains business logic that orchestrates repositories and the
  domain. Services must never reference `iced`, `MainWindow`, or UI message types.
- **`ui/`** is the iced application. Prefer moving feature logic into a per-feature
  module under `ui/` (e.g. `ui/slides`, `ui/layers`, `ui/props`) that owns its own
  nested `Message` enum and auto-wraps into the root `Message` via `impl From`. Do
  **not** expand the root `Message` enum with new feature variants.
- Per-feature state lives in `ui/state.rs` as cohesive `State` structs owned by
  `MainWindow`.
- Errors use `anyhow::Result` for fallible operations and `thiserror` for typed
  domain errors. Surface user-action failures through `MainWindow::set_error`
  rather than adding new `eprintln!` paths.

## Code of Conduct

By participating, you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).
