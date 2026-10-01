# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0]

### Added

- Initial public preview of OpenPresenter, a live
  presentation application written in Rust with the `iced` GUI framework.
- Professional multi-dock UI: left library/playlist rail, center stage and
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

### Fixed

- The navbar Search button and the global search field now show results in
  Show mode too. Previously typing a query only produced visible results while
  in Edit mode; in the default Show view the results list was never rendered,
  so search appeared dead.
- Video decode failures no longer terminate the decode thread silently. The
  error is propagated to the UI, which stops playback, clears the stale frame,
  and surfaces a toast instead of freezing the last frame on the output.
- Removed panic paths in the presenter render loop and the video decode loop.
- A file that cannot be decoded for audio now reports why, rather than playing
  silently with no indication that sound is missing.
- Unreadable rows in the database are no longer silently dropped. Slides,
  assets, themes, songs, verses, and playlist items that fail to deserialize
  now surface an error instead of disappearing from the loaded data.
- A corrupt `config.toml` is preserved as `config.toml.bak` instead of being
  overwritten with defaults, so existing output routing settings are not lost.
- Settings writes that fail are reported to the operator rather than discarded.
- A background or object image that cannot be loaded is now remembered as
  failed, instead of being re-read from disk and re-logged on every frame.
- The audio Loop toggle now actually repeats the loaded track (gapless, keeping
  the playback position when toggled mid-song); previously the button only
  flipped a flag and had no effect on playback.
- Starting the elapsed timer from the Timers dock or a remote trigger after a
  pause no longer jumps forward by the paused duration.
- The keyboard-shortcuts overlay now matches the real Show-mode bindings
  (digits 1–9 jump to slides, V/C jump to Verse/Chorus groups, Escape does not
  leave the library).
- Persisted settings are restored at launch: the default transition and its
  duration, the startup workspace (Show/Edit), reduced motion, and the default
  slide-grid column count no longer reset to built-ins on every restart.
- The NDI Send button reports that the NDI SDK runtime is missing instead of
  toggling state that silently sends nothing.

### Changed

- The recording panel now displays the dropped-frame count while recording, so
  encoder stalls are visible rather than silently degrading the output.
- Startup and background informational messages now go through the `log`
  facade rather than raw `eprintln!`/`println!`. The bundled `env_logger` is
  now actually installed at startup (default level `info`, override with
  `RUST_LOG`); previously `log::error!` calls — including trigger-server
  failures — were discarded because no logger was ever initialized.
- The HTTP and OSC remote-control listeners now start at launch only when
  enabled in settings (`triggers.http_enabled` / `triggers.osc_enabled`), and
  the default for both is **off**. They accept unauthenticated commands, so
  enabling them is an explicit choice; the triggers panel shows the "At
  launch" toggles and a security note. Previously the flags existed in the
  config file but were dead: servers only started from the panel, and the
  defaults were on.
- "Use Display" in output settings now remembers the chosen display index, and
  reopening the output window re-resolves that display's current origin, so
  output routing survives restarts and display rearrangement.
- The confidence monitor mirrors the live video frame while a video plays on
  the media layer, instead of always showing the slide.
- Removed the unused `general.auto_reconnect` config field and the never-
  rendered import/export panel flag; import/export remains available from the
  library menu.

### Added

- Dedicated stage-display window: pop the speaker view out into its own OS
  window (from the stage toolbar or the Show Controls dock) and drag it to a
  second monitor or a Downstream Keyer feed. It mirrors the in-app stage view
  — clock, countdown, elapsed timer, current/next slide, notes, and live
  alerts — and updates live.
- Audio playback queue: add multiple tracks to a queue in the Audio dock, click
  any entry to play it, and tracks auto-advance when they finish (loop repeats
  the current track). The transport bar now shows elapsed / total time for the
  loaded track.
- Stage countdown: arm or extend the countdown from the Timers dock (+1/+5/+10
  min, Clear) and it ticks down on the stage display header, turning red in the
  final minute and at zero.

### Known Limitations

> [!NOTE]
> OpenPresenter is usable in production for single-machine presentation, but
> has not been validated on multi-machine distributed setups. APIs and data
> formats may still change without notice.

- No undo/redo across editing sessions.
- No crash reporting or telemetry.

[0.1.0]: https://github.com/vincentzhangz/openpresenter/releases/tag/v0.1.0
