# OpenPresenter Architecture & Internals

This document details the architectural layout, threading model, and conventions of OpenPresenter.

---

## 1. Architectural Layers

The OpenPresenter codebase follows a strict one-way, acyclic dependency model:

```
ui → services → db / domain
ui → triggers
ui → render / media / ndi / recording / output
```

```
src/
├── domain/       # Pure data models. No I/O, no iced dependencies.
│                 # Includes Presentation, Slide, Object, Cue, Action, Playlist,
│                 # Prop, Look, Mask, SlideTheme, Song, Bible.
├── db/           # SQLite persistence layer using rusqlite.
│                 # Handles aggregate repositories and versioned schema migrations.
├── services/     # Authoritative business logic and use cases.
│                 # Coordinates repositories and domain logic; pure (no iced).
├── render/       # Software BGRA rasteriser and GPU glyphon / wgpu text renderer.
├── ndi/          # FFI bindings to NDI SDK v6 with asynchronous sender loop.
├── media/        # Video decoding via FFmpeg 9 and audio playback via rodio.
├── recording/    # H.264 video recording pipeline with bounded backpressure.
├── triggers/     # HTTP (axum), OSC (rosc), and automated macro execution.
├── output/       # Multi-screen output state, Look routing matrix, and screen destinations.
├── import/       # OpenLyrics XML and OpenPresenter Package (.opp zip) format engines.
└── ui/           # iced 0.14 GUI shell and feature modules.
```

---

## 2. UI Decomposition & State Ownership

OpenPresenter decomposes application state across cohesive feature states held on `MainWindow`:

| State Struct | Owned Concerns |
| :--- | :--- |
| `ShellState` | Active mode (`Live` / `Edit`), active sidebar tab, active inspector tab, dock visibility |
| `EditorState` | Presentation documents, slide index, undo/redo stacks, style clipboard (`copied_style`) |
| `LayerState` | Active layer index, text content, font sizes, geometries, opacity |
| `PresentingState` | Live slide, NDI stream handle, stage display status, timer timestamps |
| `OutputState` | Output destinations manager, Looks Matrix open state, resolution settings |
| `PropsState` | Active props overlays, masks, active look ID |
| `RecordingFeatureState`| Recording manager, target paths, capture state |
| `TriggersState` | Remote HTTP/OSC listener handles, macro executor tasks |
| `SongState` | Song library cache, active editing song, verse editor |
| `BibleState` | Bible translation selections, search queries, book/chapter index |

### Feature Messages & Dispatching
Feature modules (e.g. `slides`, `layers`, `output`, `songs`, `playlist`) own their child `Message` enums.
Each child message auto-wraps into root `Message` via `impl From<feature::Message> for Message`.
The root `MainWindow::update` function routes each variant cleanly to `feature::update(self, msg)`.

---

## 3. Concurrency & Threading Model

```
                    ┌─────────────────────────┐
                    │      iced Main Loop     │ (60 FPS UI Thread)
                    └───────────┬─────────────┘
                                │
       ┌────────────────────────┼────────────────────────┐
       ▼                        ▼                        ▼
┌──────────────┐        ┌──────────────┐        ┌────────────────┐
│  NDI Sender  │        │ H.264 Encoder│        │ Trigger Engine │
│ 30 FPS async │        │ Bounded mpsc │        │ Axum & OSC     │
│ FFI buffer   │        │ FFmpeg pipe  │        │ Tokio tasks    │
└──────────────┘        └──────────────┘        └────────────────┘
```

1. **Main UI Thread**: Runs the `iced` event loop, processes user interactions, updates authoritative presentation state, and handles window rendering.
2. **NDI Sender Thread**: Emits high-quality video frames asynchronously at 30 fps without blocking the UI thread.
3. **Recording Pipeline**: Receives BGRA framebuffers through a bounded mpsc queue to prevent memory blow-up under heavy GPU/disk load, encoding to H.264 via FFmpeg.
4. **Triggers Subsystem**: Axum HTTP server and OSC UDP listener run on Tokio worker threads, sending triggering actions into the UI's subscription channel.

---

## 4. Multi-Screen & Looks Matrix Engine

The Looks Matrix decouples live slide triggers from physical output destinations:
- **Layer Masking**: Supports independent suppression of Slides, Backgrounds, Props, and Masks per output.
- **Theme Overrides**: Allows assigning clean, high-contrast themes for confidence / stage monitors while rendering full graphic backgrounds to the main audience output.
- **Output Targets**: Routes to multi-monitor physical displays (via iced secondary windows) and network endpoints (via NDI streams).
