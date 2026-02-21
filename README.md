# OpenPresenter

> A modern, open-source live presentation application built in Rust — designed for houses of worship, live events, and broadcast workflows.

[![CI](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml/badge.svg)](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/vincentzhangz/98482a1448f7b055a193c05f9acf683b/raw/openpresenter-junit-tests.json)](https://github.com/vincentzhangz/openpresenter/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.93%2B-orange.svg)](https://www.rust-lang.org/)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)

> [!WARNING]
> **Early Development** — OpenPresenter is currently in active early development. APIs, data formats, and features may change without notice between versions. Expect rough edges, missing features, and occasional crashes. **Not recommended for production use yet.**

OpenPresenter is a ProPresenter-inspired presentation tool built entirely in Rust. It targets live production environments where reliability, low latency, and NDI video output matter. The entire stack — UI, renderer, database, media pipeline, and HTTP/OSC trigger system — lives in a single codebase with no Electron or web runtime.

---

## Contents

- [Features](#features)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Development](#development)
- [Contributing](#contributing)
- [Tech Stack](#tech-stack)
- [License](#license)

---

## Features

### Implemented

| Area                  | Details                                                                             |
| --------------------- | ----------------------------------------------------------------------------------- |
| **Editor UI**         | Dark three-panel layout: slide thumbnail sidebar, canvas editor, tabbed inspector   |
| **Presenter view**    | Full-screen show mode with visual slide sidebar and toolbar                         |
| **Library**           | SQLite-backed presentation and song library with FTS5 full-text search              |
| **CRUD**              | Create, rename, delete presentations, slides, and songs                             |
| **Text slides**       | Configurable font, size, color, alignment, text transforms per slide                |
| **Layers**            | Per-slide layer stack: text, background color/image, video                          |
| **Songs & lyrics**    | Song library with verse/chorus structure; OpenLyrics XML import/export              |
| **Import / Export**   | OpenLyrics (`.xml`), OpenPresenter Package (`.opp` — zip bundle)                    |
| **Transitions**       | Cut, Fade, and Slide (horizontal wipe) — duration per slide                         |
| **Props & Looks**     | Lower-third and logo overlays; save/restore visibility "Looks"                      |
| **NDI output**        | Real-time 30 fps NDI stream via NDI SDK v6 FFI bindings                             |
| **GPU text**          | Hardware-accelerated text via [glyphon](https://github.com/grovesNL/glyphon) + wgpu |
| **Software renderer** | CPU BGRA rasteriser with shadow, outline, and alpha blending (fallback)             |
| **Video decoding**    | FFmpeg-backed frame decoder with hardware-accelerated paths where available         |
| **Audio playback**    | rodio-based audio player with load/play/pause/stop/volume                           |
| **Recording**         | H.264 video recording pipeline via FFmpeg encoder with bounded backpressure         |
| **HTTP triggers**     | axum 0.8 REST API for remote slide control (`/api/slides/next`, etc.)               |
| **OSC triggers**      | rosc-powered Open Sound Control listener (`/slide/next`, `/black`, etc.)            |
| **Macros**            | Scheduled trigger sequences with optional looping                                   |
| **Themes**            | Reusable visual themes stored in the database                                       |
| **Service planning**  | Service plan with ordered items backed by SQLite                                    |
| **Bible**             | Bible verse database with FTS5 search (translations, books, chapters)               |
| **CI**                | GitHub Actions pipeline: fmt → clippy → test → Linux build check                    |

### In Progress

- Stage display (secondary monitor audience view)
- Multi-monitor output management
- Slide compositor with full layer blending

### Planned

- Windows and Linux packaging
- Hotkey / MIDI trigger backend
- Thumbnail generation for video media
- ProPresenter 6/7 import

---

## Architecture

```
src/
├── slides/         # Core data model — Presentation, Slide, Song, Verse, Layer, Prop
├── db/             # SQLite persistence (rusqlite) with versioned migrations
├── render/         # Software BGRA rasteriser + GPU glyphon/wgpu text renderer
├── ndi/            # FFI bindings to NDI SDK v6; async sender loop
├── import/         # OpenLyrics XML + OPP zip import/export
├── media/          # FFmpeg video decoder, rodio audio player
├── recording/      # H.264 recording pipeline via FFmpeg encoder
├── triggers/       # HTTP (axum), OSC (rosc), and automation macro subsystems
├── output/         # Output window management
├── config.rs       # TOML config with platform-aware data directories
└── ui/             # iced 0.14 application — all UI components and message handlers
```

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
- **Rust 1.93+** — [rustup.rs](https://rustup.rs/)
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
- Rust 1.93+ via rustup
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

Contributions are welcome. Please read the guidelines below before opening a pull request.

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
- **Security issues**: Contact the maintainer directly rather than filing a public issue.

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
| Media decoding | [ffmpeg-next](https://docs.rs/ffmpeg-next/)    | 8       |
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

---

*OpenPresenter is not affiliated with nor endorsed by Renewed Vision (ProPresenter).*
