# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0]

### Added

- Initial public preview of OpenPresenter, a ProPresenter-inspired live
  presentation application written in Rust with the `iced` GUI framework.
- Unified ProPresenter-style UI: left library/playlist rail, center stage and
  slide thumbnails, and a right output/inspector dock, plus a collapsible bottom
  media bin.
- Dark charcoal theme with an orange selection accent.
- Presentation, slide, and song CRUD backed by SQLite.
- Per-slide layers (text, background color/image, video) and per-slide group
  labels.
- Text styling (font, size, color, alignment, transforms) and transitions
  (Cut, Fade, Slide) with per-slide durations.
- Library with FTS5 full-text search and image/video assets.
- Songs & lyrics library with OpenLyrics XML import/export.
- Props, Looks, lower-third overlays, and reusable slide themes.
- NDI video output (via NDI SDK v6, gated behind the `ndi-sdk` feature) and an
  optional software renderer fallback.
- FFmpeg-backed video decoding and rodio-based audio playback.
- H.264 recording pipeline via FFmpeg.
- HTTP (axum) and OSC (rosc) trigger subsystems, scheduled macros, and per-slide
  cues.
- Bible verse database with FTS5 search.
- Service planning (playlists) backed by SQLite.
- GitHub Actions CI: fmt, clippy (`--no-default-features`, warnings as errors),
  tests with coverage, and a Linux stub build check.

### Known Limitations

> [!WARNING]
> OpenPresenter is in **early development** and is **not recommended for
> production use** yet. APIs, data formats, and features may change without
> notice.

- Stage display and multi-monitor output management are incomplete.
- Hotkey / MIDI trigger backend is not yet implemented.
- ProPresenter 6/7 import is planned but not available.
- Windows and Linux packaging is not yet provided.

[0.1.0]: https://github.com/vincentzhangz/openpresenter/releases/tag/v0.1.0
