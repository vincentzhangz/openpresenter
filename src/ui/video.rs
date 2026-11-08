//! Video playback logic for the edited/live slide's video content.
//!
//! Ported from the iced-era `ui/video/mod.rs`. The transport controls render
//! inline inside the inspector's video content tab (see `ui/inspector.rs`);
//! this module owns the message dispatch, the player lifecycle
//! ([`open_video`] / [`stop_video`] / the presenter follow-hook
//! [`on_presenter_slide_changed`]) and the 33 ms frame tick that pulls
//! decoded frames into the GPUI render cache ([`VideoState::frame`]).

use std::sync::Arc;

use gpui_kit::{Context, RenderImage};

use crate::media::MediaPlayer;
use crate::media::decoder::RgbaFrame;
use crate::ui::main_window::MainWindow;

/// A player that was opened on a worker thread, waiting to be installed.
///
/// Opening a media file takes long enough to be felt: the file, the video
/// decoder and the audio device all have to be opened. The work happens off
/// the UI thread, and the player is installed when it arrives — but only if
/// the operator is still on the same file.
pub(crate) struct OpenedPlayer {
    /// The file the player was opened with.
    pub(crate) path: String,
    /// The player itself.
    pub(crate) player: Option<MediaPlayer>,
    /// Why the file could not be opened.
    pub(crate) error: Option<String>,
}

/// Background slot for players opened on a worker thread.
#[allow(clippy::type_complexity)]
fn background_slot() -> Arc<std::sync::Mutex<Option<(u64, OpenedPlayer)>>> {
    static SLOT: std::sync::OnceLock<Arc<std::sync::Mutex<Option<(u64, OpenedPlayer)>>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| Arc::new(std::sync::Mutex::new(None)))
        .clone()
}

/// Open the media file on a worker thread and leave the player in the slot.
fn open_in_background(w: &mut MainWindow, path: &str, play: bool) {
    let slot = background_slot();
    let path = path.to_string();
    let looping = w.video.looping;
    w.video.pending_path = Some(path.clone());
    w.video.pending_play = play;
    // The open is stamped: a file that takes a long time to open must not
    // overwrite a newer one.
    w.video.opening_generation = w.video.opening_generation.wrapping_add(1);
    let generation = w.video.opening_generation;
    std::thread::spawn(move || {
        let opened = match MediaPlayer::open_with_loop(&path, looping) {
            Ok(player) => OpenedPlayer {
                path: path.clone(),
                player: Some(player),
                error: None,
            },
            Err(e) => OpenedPlayer {
                path: path.clone(),
                player: None,
                error: Some(format!("MediaPlayer: failed to open '{path}': {e}")),
            },
        };
        if let Ok(mut guard) = slot.lock() {
            *guard = Some((generation, opened));
        }
    });
}

/// Install a player that was opened on a worker thread.
///
/// The player is only installed if the operator is still on the same file:
/// one that arrives after the operator moved on would play the wrong thing.
fn install_pending_player(w: &mut MainWindow) {
    let pending = background_slot()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    let Some((generation, pending)) = pending else {
        return;
    };
    if generation != w.video.opening_generation {
        // A newer open is on its way: this one is dropped.
        return;
    }
    if w.video.pending_path.as_deref() != Some(pending.path.as_str()) {
        // The operator moved on: the player is dropped.
        return;
    }
    w.video.pending_path = None;
    let play = w.video.pending_play;
    w.video.pending_play = false;

    let Some(mut player) = pending.player else {
        if let Some(e) = pending.error {
            w.set_error(e);
        }
        return;
    };
    player.set_volume(w.video.volume);
    player.set_mute(w.video.muted);
    player.set_speed(w.video.speed);
    for ndi in w.presenting.ndi_outputs.values() {
        ndi.set_video_frame_source(Some(player.video.shared_frame_arc()));
    }
    if let Some(e) = player.take_audio_error() {
        w.set_error(format!("MediaPlayer: {e}"));
    }
    if play {
        player.play();
    }
    w.video.player = Some(player);
}

/// Messages owned by the Video feature module.
///
/// `VideoFrameTick` stays as a root variant (it is injected by the frame-tick
/// bridge, not produced by a UI widget).
#[derive(Debug, Clone)]
pub enum Message {
    PlayToggled,
    LoopToggled(bool),
    VolumeChanged(f32),
    /// The operator is dragging the seek slider: the position moves, but the
    /// player is left alone until the slider is released.
    SeekPreview(f64),
    /// The operator let go of the seek slider: the player jumps there.
    SeekChanged(f64),
    MuteToggled,
    SpeedChanged(f64),
}

/// Dispatch a video message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::PlayToggled => video_play_toggled(w),
        Message::LoopToggled(l) => video_loop_toggled(w, l),
        Message::VolumeChanged(v) => video_volume_changed(w, v),
        Message::SeekPreview(s) => video_seek_preview(w, s),
        Message::SeekChanged(s) => video_seek_changed(w, s),
        Message::MuteToggled => video_mute_toggled(w),
        Message::SpeedChanged(s) => video_speed_changed(w, s),
    }
    cx.notify();
}

/// Periodic tick while the video player is actively playing (driven by the
/// 33 ms bridge in `MainWindow::start_bridges`): surfaces fatal decode
/// failures as a toast, pulls the newest decoded frame into the GPUI render
/// cache, and advances the transport position.
pub(crate) fn video_frame_tick(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // A player that was opened on a worker thread is installed here.
    install_pending_player(w);
    if let Some(ref player) = w.video.player
        && let Some(e) = player.take_decode_error()
    {
        // Surface fatal decode failures instead of letting the last frame
        // silently freeze on the output.
        player.stop();
        w.video.player = None;
        w.video.frame = None;
        w.video.frame_fingerprint = 0;
        w.video.frame_identity = 0;
        // The broadcast composites the slide itself, so it holds its own copy
        // of the picture: without this the stream keeps showing the last
        // frame of a video that is no longer playing.
        for ndi in w.presenting.ndi_outputs.values() {
            ndi.set_video_frame_source(None);
        }
        w.set_error(format!("Video playback stopped: {e}"));
        cx.notify();
        return;
    }

    if let Some(ref player) = w.video.player {
        // The decoded frame is a multi-megabyte buffer, so its identity is
        // asked for first and the pixels are only copied when the decoder
        // really produced a new frame. A paused video would otherwise copy
        // the same picture on every tick.
        let identity = player.current_frame_identity().unwrap_or(0);
        if identity != w.video.frame_identity
            && let Some(frame) = player.current_frame()
        {
            let fingerprint = frame_fingerprint(&frame);
            let needs_rebuild = w.video.frame_fingerprint != fingerprint || w.video.frame.is_none();
            if needs_rebuild && let Some(image) = build_render_image(&frame) {
                w.video.frame = Some(Arc::new(image));
                w.video.frame_fingerprint = fingerprint;
            }
            w.video.frame_identity = identity;
        }
        // The position the player reports is the truth, unless a seek is
        // still on its way: the player catches up a moment later, and
        // writing its old position would snap the slider back.
        let position = player.position_secs();
        match w.video.seek_target {
            Some(target) if (position - target).abs() <= 0.5 => {
                // The seek has taken effect.
                w.video.seek_target = None;
                w.video.position = position;
            }
            Some(_) => {}
            None => w.video.position = position,
        }
    }
    cx.notify();
}

/// Start playback of the given media file, tearing down any previous player
/// first. Applies the panel's loop/volume/mute/speed preferences and wires the
/// decoder's shared frame arc into a running NDI output.
pub(crate) fn open_video(w: &mut MainWindow, path: &str) {
    if let Some(ref player) = w.video.player {
        player.stop();
    }
    w.video.player = None;
    w.video.frame = None;
    w.video.frame_fingerprint = 0;
    w.video.frame_identity = 0;
    w.video.position = 0.0;
    w.video.seek_target = None;

    if path.is_empty() {
        return;
    }

    // The file is opened off the UI thread: the window stays responsive
    // while the decoder and the audio device are opened.
    open_in_background(w, path, true);
}

/// Point the player at a file without starting it.
///
/// The editor's transport belongs to the slide being edited, so the player
/// has to be the one for that slide's file: a transport that drove another
/// file would play the wrong thing. The file is opened paused, so the
/// decision to play stays with the operator.
pub(crate) fn load_video(w: &mut MainWindow, path: &str) {
    if path.is_empty() {
        return;
    }
    // Already on this file: leave it alone, so the position survives.
    if w.video
        .player
        .as_ref()
        .is_some_and(|player| player.path() == path)
        || w.video.pending_path.as_deref() == Some(path)
    {
        return;
    }
    if let Some(ref player) = w.video.player {
        player.stop();
    }
    w.video.player = None;
    w.video.frame = None;
    w.video.frame_fingerprint = 0;
    w.video.frame_identity = 0;
    w.video.position = 0.0;
    w.video.seek_target = None;

    // The file is opened off the UI thread: the window stays responsive
    // while the decoder and the audio device are opened.
    open_in_background(w, path, false);
}

/// Stop playback and drop the player (slide changed away from video, the
/// presentation was closed, ...).
pub(crate) fn stop_video(w: &mut MainWindow) {
    if let Some(ref player) = w.video.player {
        player.stop();
    }
    for ndi in w.presenting.ndi_outputs.values() {
        ndi.set_video_frame_source(None);
    }
    w.video.player = None;
    w.video.frame = None;
    w.video.frame_fingerprint = 0;
    w.video.frame_identity = 0;
    w.video.position = 0.0;
    w.video.seek_target = None;
    // A player that is still being opened is dropped: the operator moved on.
    w.video.pending_path = None;
    w.video.pending_play = false;
}

/// Follow the presenting slide: open the slide's video (if any) or stop the
/// player. Called from every slide-activation path and the Show-mode switch.
pub(crate) fn on_presenter_slide_changed(w: &mut MainWindow, slide_index: usize) {
    let path = w
        .presenting
        .presentation
        .as_ref()
        .and_then(|p| p.slides.get(slide_index))
        .and_then(|s| s.video_path());

    match path {
        Some(p) => open_video(w, &p),
        None => stop_video(w),
    }
}

fn video_play_toggled(w: &mut MainWindow) {
    if let Some(ref player) = w.video.player {
        if player.has_ended() {
            // The clip ran to its end and the decode thread stopped with it:
            // the file is opened again, so the operator can watch it twice.
            let path = player.path().to_string();
            open_video(w, &path);
            return;
        }
        player.toggle_play_pause();
    } else if w.video.pending_path.is_some() {
        // The player is still being opened: the decision is remembered and
        // applied when it arrives.
        w.video.pending_play = !w.video.pending_play;
    }
}

fn video_loop_toggled(w: &mut MainWindow, looping: bool) {
    w.video.looping = looping;
    if let Some(ref mut player) = w.video.player {
        player.set_loop(looping);
    }
}

fn video_volume_changed(w: &mut MainWindow, volume: f32) {
    w.video.volume = volume;
    if let Some(ref mut player) = w.video.player {
        player.set_volume(volume);
    }
}

/// The operator is dragging the seek slider.
///
/// The position readout follows the slider, but the player is left alone:
/// a seek rebuilds the sound at the new position, so doing that on every
/// step of a drag would rebuild it dozens of times.
fn video_seek_preview(w: &mut MainWindow, secs: f64) {
    w.video.position = secs;
}

/// The operator let go of the seek slider: the player jumps there.
fn video_seek_changed(w: &mut MainWindow, secs: f64) {
    w.video.position = secs;
    // The seek is on its way: the player's own position is not the truth
    // until it catches up.
    w.video.seek_target = Some(secs);
    if let Some(ref mut player) = w.video.player
        && let Err(e) = player.seek(secs)
    {
        // The sound could not follow the picture: the operator has to know,
        // or the clip plays on from the beginning with the picture jumped.
        w.set_error(format!("MediaPlayer: {e}"));
    }
}

fn video_mute_toggled(w: &mut MainWindow) {
    w.video.muted = !w.video.muted;
    if let Some(ref mut player) = w.video.player {
        player.set_mute(w.video.muted);
    }
}

fn video_speed_changed(w: &mut MainWindow, speed: f64) {
    w.video.speed = speed.max(0.1);
    if let Some(ref mut player) = w.video.player {
        player.set_speed(w.video.speed);
    }
}

/// Fingerprint a decoded frame so the render cache is only rebuilt when the
/// decoder actually produced a new frame (same pts + size = same content).
fn frame_fingerprint(frame: &RgbaFrame) -> u64 {
    let mut hash = u64::from(frame.width);
    hash = hash.wrapping_mul(0x9E_37_79_B9_7F_4A_7C_15) ^ u64::from(frame.height);
    hash = hash.wrapping_mul(0x9E_37_79_B9_7F_4A_7C_15) ^ frame.pts_secs.to_bits();
    hash
}

/// Convert a decoded RGBA frame into a GPUI [`RenderImage`].
///
/// GPUI's image pipeline expects the frame buffer in **BGRA** byte order (its
/// own loaders swap RGBA sources before calling `RenderImage::new`), so the R
/// and B bytes are swapped here once per new frame rather than at render time.
fn build_render_image(frame: &RgbaFrame) -> Option<RenderImage> {
    let mut bgra = frame.data.clone();
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let buffer = image::RgbaImage::from_raw(frame.width, frame.height, bgra)?;
    Some(RenderImage::new(vec![image::Frame::new(buffer)]))
}
