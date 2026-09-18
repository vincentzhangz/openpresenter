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

## Architecture & Layering

OpenPresenter follows a strict one-way acyclic layered architecture (see [`docs/architecture.md`](docs/architecture.md)):

```
src/
├── domain/      # Pure data models (Presentation, Slide, Object, Cue, Action, Playlist, Prop, Look)
│                # Serde + pure domain helpers only. NO I/O, NO iced.
├── db/          # SQLite persistence (rusqlite) + versioned migrations.
├── services/    # Business logic orchestration over repositories (no iced, no UI types).
├── render/      # Software BGRA rasteriser + GPU glyphon/wgpu text renderer.
├── ndi/         # NDI SDK v6 FFI sender loop.
├── media/       # FFmpeg video decoder + rodio audio playback.
├── recording/   # H.264 video recording pipeline.
├── triggers/    # HTTP (axum), OSC (rosc), and macro automation dispatchers.
├── output/      # Multi-screen output routing & look configuration models.
└── ui/          # iced application shell + per-feature modules.
```

**Dependency Rule**: `ui → services → db / domain`; `ui → triggers`; `ui → render / media / ndi`.
Nothing outside `ui` may depend on `iced`.

## Feature Modules & Message Pattern

Each feature in `ui/` (e.g. `ui/slides`, `ui/playlist`, `ui/stage`, `ui/library`, `ui/props`, `ui/output`, `ui/songs`, `ui/bible`) must follow the nested `Message` pattern:

1. **Feature-scoped `Message` enum**:
   ```rust
   #[derive(Debug, Clone)]
   pub enum Message {
       DoSomething,
   }
   ```
2. **Auto-wrap into root `Message`** via `impl From<Message> for RootMessage` (in `src/ui/messages.rs`).
3. **Module `update` function**:
   ```rust
   pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> { ... }
   ```
4. **State encapsulation**: Feature state lives in a dedicated struct in `src/ui/state.rs` (e.g. `StageState`, `LibraryState`) and is owned by `MainWindow`.

## Quality & Parity Standards

- **Zero Panics in Production**: Live operators must never experience a crash. Never index slices or collections with direct brackets `[index]` without bounds clamping or `.get(index)`.
- **User Error Toasting**: Surface errors via `MainWindow::set_error(msg)` which populates `ui.error_message`. Do not introduce new `eprintln!` error paths.
- **NDI & Multi-Screen Parity**: Layer clears (`ClearAll`, `ClearSlide`, `ClearMedia`, `ClearProps`, `ClearMessages`) and blackouts must synchronize across both physical displays and live NDI streams.
- **Strict Linting**: CI checks `cargo clippy --no-default-features -- -D warnings` and `cargo fmt --check`. Keep both clean.

## Testing Guidelines

Run tests for specific layers or full suites:

```bash
# Full test suite
cargo test --no-default-features

# Domain unit tests
cargo test --lib domain --no-default-features

# Render pipeline tests
cargo test --lib render --no-default-features

# Output and multi-screen routing tests
cargo test --lib output --no-default-features

# Database integration tests
cargo test --test db_integration --no-default-features
```

## Code of Conduct

By participating, you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).
