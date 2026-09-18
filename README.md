# OpenPresenter

> A modern, open-source live presentation application built in Rust — designed for houses of worship, live events, and broadcast workflows.

[![CI](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml/badge.svg)](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/endpoint?url=https%3A%2F%2Fgist.githubusercontent.com%2Fvincentzhangz%2F98482a1448f7b055a193c05f9acf683b%2Fraw%2Fopenpresenter-junit-tests.json)](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https%3A%2F%2Fgist.githubusercontent.com%2Fvincentzhangz%2F98482a1448f7b055a193c05f9acf683b%2Fraw%2Fopenpresenter-lcov-coverage.json)](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.97%2B-orange.svg)](https://www.rust-lang.org/)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)

OpenPresenter is a live presentation tool built entirely in Rust. It targets live production environments where reliability, low latency, and NDI/multi-screen video output matter. The entire stack — UI, renderer, database, media pipeline, and HTTP/OSC trigger system — lives in a single codebase with no Electron or web runtime.

---

## Contents

- [Features](#features)
- [Operator Keyboard Shortcuts](#operator-keyboard-shortcuts)
- [Documentation](#documentation)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Development](#development)
- [Contributing](#contributing)
- [Security](#security)
- [Tech Stack](#tech-stack)
- [License](#license)

---

## Features

### Implemented

| Area                         | Details                                                                                     |
| ---------------------------- | ------------------------------------------------------------------------------------------- |
| **Unified Professional Shell**| Dark multi-dock layout: left library/playlist rail, center canvas & live view, right dock   |
| **Quick Clear & Blackout**   | Broadcast-grade quick clear actions (`Clear All`, `Slide`, `Media`, `Props`, `Messages`) and seamless Blackout with live NDI synchronization |
| **Stage Display / Confidence Monitor** | Dedicated stage display window with live wall clock, countdown timer, current/next slide text, and stage alerts |
| **Multi-Object Compositor**  | Canvas & frame rasteriser supporting multi-layer text, shapes (rectangle, ellipse, triangle, line), images, and video with z-ordering |
| **Multi-Screen & Looks Matrix** | Routing matrix for up to 8 outputs (Window / NDI); toggle layers per display; custom looks  |
| **Screen Output Management** | Configure screen destinations, 720p/1080p resolution changes, output routing modal        |
| **Editor UI & Object Strip** | Canvas editor with interactive bottom object chip strip for layer selection and reordering  |
| **Presenter View**           | Full-screen show mode with visual slide grid, live preview, transition controls, and timers |
| **Inspector & Typography**   | Text styling, font family selector, size stepper, style copy/paste clipboard, glow effects  |
| **Interactive Layers**       | Layer stack with per-object visibility toggle (eye), lock toggle, reordering, and deletion |
| **Color Presets & Swatches** | Presentation-optimized color palettes with instant one-click swatches                       |
| **Library & Assets**         | SQLite-backed presentation and song library with FTS5 search; seamless sidebar layout       |
| **Songs & Lyrics**           | Song editor with verse/chorus structure; inline OpenLyrics XML import/export                |
| **Package Import / Export**  | OpenPresenter Package (`.opp` zip bundle) import/export directly in presentation list       |
| **Transitions**              | Cut, Fade, and Slide (horizontal wipe) with per-slide duration                              |
| **Props & Overlays**         | Lower-thirds, logo bugs, alerts, and saved visibility Looks                                 |
| **NDI Output**               | Real-time 30 fps NDI stream via NDI SDK v6 FFI bindings with toggleable live navbar badge   |
| **GPU Text Rendering**       | Hardware-accelerated text via [glyphon](https://github.com/grovesNL/glyphon) + wgpu          |
| **Software Renderer**        | CPU BGRA rasteriser with shadow, outline, shape rasterisation, and alpha blending fallback |
| **Video Decoding**           | Modern FFmpeg 9-backed frame decoder with hardware acceleration paths where available       |
| **Audio Playback**           | rodio-based audio player with load/play/pause/stop/volume                                   |
| **Recording**                | H.264 video recording pipeline via FFmpeg encoder with bounded backpressure                 |
| **HTTP Triggers**            | axum 0.8 REST API for remote slide control (`/api/slides/next`, etc.)                       |
| **OSC Triggers**             | rosc-powered Open Sound Control listener (`/slide/next`, `/black`, etc.)                    |
| **Macros & Automation**      | Scheduled trigger sequences with optional looping                                           |
| **Slide Cues**               | Per-slide trigger actions (immediate or delayed) fired when a slide goes live               |
| **Themes**                   | Reusable slide visual themes stored in SQLite database with import/export capabilities      |
| **Service Planning**         | Service plans with ordered items backed by SQLite                                           |
| **Bible Database**           | Scripture database with FTS5 search (translations, books, chapters, verses)                 |
| **CI & Quality Assurance**   | Automated test suite (130+ unit tests), zero-warning clippy and rustfmt enforcement         |

---

## Operator Keyboard Shortcuts

OpenPresenter provides standard operator shortcuts for rapid live control:

| Key Combination | Action |
| --- | --- |
| <kbd>Space</kbd> / <kbd>→</kbd> / <kbd>↓</kbd> | Next Slide |
| <kbd>←</kbd> / <kbd>↑</kbd> | Previous Slide |
| <kbd>1</kbd> – <kbd>9</kbd> | Jump directly to Slide index |
| <kbd>F1</kbd> | **Clear All** (clears slide, media, props, and messages) |
| <kbd>F2</kbd> | **Clear Slide** (clears text and foreground slide content) |
| <kbd>F3</kbd> | **Clear Background / Media** |
| <kbd>F4</kbd> | **Clear Props** |
| <kbd>F5</kbd> | **Clear Messages** |
| <kbd>F6</kbd> / <kbd>B</kbd> | Toggle **Blackout** (instant black screen across outputs & NDI) |
| <kbd>V</kbd> | Toggle View Mode (Show / Edit / Stage / Unified) |
| <kbd>C</kbd> | Clear Active Output |
| <kbd>Cmd</kbd> + <kbd>,</kbd> / <kbd>Ctrl</kbd> + <kbd>,</kbd> | Open Settings Modal |

---

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

- [**User Guide**](docs/user_guide.md) — Comprehensive guide for operators and worship media teams.
- [**Architecture & Internals**](docs/architecture.md) — Detailed architecture, data flow, and threading model.
- [**API & Remote Control**](docs/api_and_control.md) — REST API endpoints, OSC address map, and macro reference.
- [**Configuration & Outputs**](docs/configuration.md) — Screen setup, NDI streaming, and configuration options.

---

## Architecture

```
src/
├── domain/         # Core data model (was `slides/`) — Presentation, Slide, Object,
│                   #   Cue, Action, Playlist, Prop, Look, SlideTheme, Song, Bible*
├── db/             # SQLite persistence (rusqlite) with versioned migrations
├── services/       # Business logic over repositories; pure orchestration (no iced)
├── render/         # Software BGRA rasteriser + GPU glyphon/wgpu text renderer
├── ndi/            # FFI bindings to NDI SDK v6; async sender loop
├── media/          # FFmpeg video decoder, rodio audio player
├── recording/      # H.264 recording pipeline via FFmpeg encoder
├── triggers/       # HTTP (axum), OSC (rosc), and automation macro subsystems
├── output/         # Multi-screen output / NDI output-window management
├── import/         # OpenLyrics XML + OPP zip import/export
├── config.rs       # TOML config with platform-aware data directories
└── ui/             # iced 0.14 application
    ├── main_window.rs  # Root window: update / view / subscription / error handling
    ├── messages.rs     # Root Message + ViewMode / SidebarTab / RightDockTab enums
    ├── state.rs        # Per-feature State structs (Editor, Presenting, Shell, …)
    ├── theme.rs        # Dark charcoal palette + orange accent, container/button styles
    ├── shell/          # Multi-dock layout: left_rail, center,
    │                   #   right_dock, media_bin, show, edit, unified
    └── <feature>/      # Per-feature modules (playlist, slides, layers, presenter,
        (e.g. slides,    #   props, audio, library, themes, triggers, recording, bible,
         layers, …)     #   songs, output, ndi, stage, …) each owning a nested `Message`
                        #   enum that auto-wraps into the root via `impl From`
```

**Layer dependency direction (acyclic):** `ui → services → db / domain`;
`ui → triggers`; `ui → render / media / ndi`. Nothing outside `ui` may depend on
`iced`. New editor/feature messages use nested `Message` enums wired by `From`,
so the root `Message` enum is not expanded with new feature variants.

**Thread model**

| Thread       | Role                                                           |
| ------------ | -------------------------------------------------------------- |
| Main         | iced event loop + UI rendering                                 |
| NDI sender   | 30 fps async task — renders frame, sends via NDI, sleeps       |
| Encoder      | Bounded channel consumer — receives BGRA frames, encodes H.264 |
| HTTP server  | axum tokio task — sends `TriggerAction` over mpsc channel      |
| OSC listener | UDP tokio task — decodes OSC packets, sends `TriggerAction`    |

All cross-thread communication uses `tokio::sync::mpsc`; the UI receives trigger actions via a single subscription channel.

---

## Prerequisites

### macOS (primary platform)

- **Xcode Command Line Tools** — `xcode-select --install`
- **Rust 1.97+** — [rustup.rs](https://rustup.rs/)
- **FFmpeg 7+** — `brew install ffmpeg`
- **pkg-config** — `brew install pkgconf`
- **NDI SDK v6** _(optional — stub mode available without it)_ — [ndi.video/for-developers](https://ndi.video/for-developers/ndi-sdk/)

### Linux

```bash
sudo apt-get install -y \
    build-essential pkg-config \
    libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev \
    libasound2-dev libssl-dev
```

### Windows

- Visual Studio Build Tools 2022
- Rust 1.97+ via rustup
- [vcpkg](https://vcpkg.io/) for FFmpeg (`vcpkg install ffmpeg:x64-windows`)
- NDI SDK for Windows

---

## Installation

```bash
# Clone
git clone https://github.com/vincentzhangz/openpresenter.git
cd openpresenter

# macOS: install system dependencies
brew install ffmpeg pkgconf

# Build (release)
cargo build --release

# Run
cargo run --release
```

### Without NDI SDK

The NDI feature is compiled behind a Cargo feature flag. To build without the SDK:

```bash
cargo build --no-default-features
```

### NDI SDK Setup

1. Download the [NDI SDK](https://ndi.video/for-developers/ndi-sdk/) for your platform.
2. Install to the default location:
   - **macOS**: `/Library/NDI SDK for Apple/lib/macOS/libndi.dylib`
   - **Windows**: `C:\Program Files\NDI\NDI SDK\Lib\x64\Processing.NDI.Lib.x64.dll`
   - **Linux**: `/usr/lib/libndi.so`

### FFmpeg path (macOS / Homebrew)

Build configuration lives in `.cargo/config.toml`. For Apple Silicon Homebrew:

```toml
[env]
FFMPEG_DIR = "/opt/homebrew/opt/ffmpeg"
CPATH     = "/opt/homebrew/opt/ffmpeg/include"
```

Update for Intel (`/usr/local/opt/ffmpeg`) or custom installations as needed.

---

## Configuration

Application data is stored in platform-specific directories (created automatically on first run):

| Platform | Path                                           |
| -------- | ---------------------------------------------- |
| macOS    | `~/Library/Application Support/openpresenter/` |
| Windows  | `%APPDATA%\openpresenter\`                     |
| Linux    | `~/.config/openpresenter/`                     |

Config file: `config.toml` · Database: `library.db`

Key config options:

```toml
[output]
width  = 1920
height = 1080

[ndi]
source_name = "OpenPresenter"

[http_trigger]
port = 9090

[osc_trigger]
port = 9000
```

---

## Development

```bash
# Check for compile errors (no NDI SDK required)
cargo check --no-default-features

# Run all tests
cargo test --no-default-features

# Format
cargo fmt

# Lint (warnings are errors in CI)
cargo clippy --no-default-features -- -D warnings

# Build docs
cargo doc --no-deps --open
```

### HTTP Trigger API

| Method | Path                       | Action                                   |
| ------ | -------------------------- | ---------------------------------------- |
| `GET`  | `/api/status`              | Health check                             |
| `POST` | `/api/slides/next`         | Next slide                               |
| `POST` | `/api/slides/prev`         | Previous slide                           |
| `POST` | `/api/slides/goto/{index}` | Go to slide by index                     |
| `POST` | `/api/black/{on}`          | Black screen (`true`/`1`/`on` to enable) |
| `POST` | `/api/clear`               | Clear output                             |
| `POST` | `/api/timer/start`         | Start timer                              |
| `POST` | `/api/timer/stop`          | Stop timer                               |
| `POST` | `/api/timer/reset`         | Reset timer                              |

### OSC Addresses

| Address        | Args         | Action              |
| -------------- | ------------ | ------------------- |
| `/slide/next`  | —            | Next slide          |
| `/slide/prev`  | —            | Previous slide      |
| `/slide/goto`  | `int` index  | Go to slide         |
| `/black`       | `int`/`bool` | Black screen on/off |
| `/clear`       | —            | Clear output        |
| `/timer/start` | —            | Start timer         |
| `/timer/stop`  | —            | Stop timer          |
| `/timer/reset` | —            | Reset timer         |

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [Architecture Guide](docs/architecture.md) for architectural guidelines, message conventions, and state organization before opening a pull request.

### Getting Started

1. **Fork** the repository and clone your fork.
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature
   ```
3. **Make changes** — keep commits focused and atomic.
4. **Test and lint**:
   ```bash
   cargo test --no-default-features
   cargo clippy --no-default-features -- -D warnings
   cargo fmt --check
   ```
5. **Open a pull request** against `main` with a clear description of what and why.

### Filing Issues

- **Bug reports**: Include OS, `rustc --version`, steps to reproduce, and full error output.
- **Feature requests**: Describe the use case and the problem it solves.
- **Security issues**: Report privately via GitHub's security advisory workflow — see [SECURITY.md](SECURITY.md). Do **not** file a public issue.

### Pull Request Checklist

- [ ] `cargo test --no-default-features` passes
- [ ] `cargo clippy --no-default-features -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] New public items have doc comments
- [ ] Reviewer assigned via CODEOWNERS has approved

---

## Tech Stack

| Component      | Crate                                          | Version |
| -------------- | ---------------------------------------------- | ------- |
| UI framework   | [iced](https://iced.rs/)                       | 0.14    |
| GPU rendering  | [wgpu](https://wgpu.rs/)                       | 28      |
| Text rendering | [glyphon](https://docs.rs/glyphon/)            | 0.10    |
| Database       | [rusqlite](https://docs.rs/rusqlite/)          | 0.38    |
| Media decoding | [ffmpeg-next](https://docs.rs/ffmpeg-next/)    | 9       |
| Audio          | [rodio](https://docs.rs/rodio/)                | 0.22    |
| NDI output     | NDI SDK v6 (bindgen FFI)                       | 6       |
| HTTP triggers  | [axum](https://docs.rs/axum/)                  | 0.8     |
| OSC triggers   | [rosc](https://docs.rs/rosc/)                  | 0.11    |
| Serialization  | [serde](https://serde.rs/) + serde_json + toml | —       |
| Async runtime  | [tokio](https://tokio.rs/)                     | 1       |

---

## License

Copyright 2026 Vincent

Licensed under the [Apache License, Version 2.0](LICENSE).

You may not use this software except in compliance with the License.
A copy of the License is included in this repository as [`LICENSE`](LICENSE).
