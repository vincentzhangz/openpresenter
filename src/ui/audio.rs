//! Audio playback panel (right dock Audio tab).
//!
//! Ported from the iced-era `ui/audio/mod.rs`: the transport bar (open /
//! play / pause / stop / loop / volume) plus the playback queue. The 250 ms
//! audio tick bridge (see `MainWindow::start_bridges`) refreshes the position
//! readout and auto-advances the queue while a track plays.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::slider::Slider;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, ParentElement, SharedString, Styled, Window, div, px};

use crate::media::AudioPlayer;
use crate::media::audio::AudioError;
use crate::ui::components::truncate;
use crate::ui::main_window::MainWindow;
use crate::ui::state::QueueTrack;
use crate::ui::theme;

/// Mirror the master volume into the volume slider.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let volume = w.audio.volume;
    let current = w.audio_volume_slider.read(cx).value().start();
    if (current - volume).abs() > 0.0001 {
        w.audio_volume_slider.update(cx, |state, slider_cx| {
            state.set_value(volume, window, slider_cx);
        });
    }
}

/// Messages owned by the Audio feature module.
#[derive(Debug, Clone)]
pub enum Message {
    PickFile,
    Load(String),
    Play,
    Pause,
    Stop,
    VolumeChanged(f32),
    ToggleLoop,
    /// Pick multiple audio files and append them to the playback queue.
    QueueAdd,
    QueuePlayAt(usize),
    QueueRemove(usize),
    QueueClear,
}

/// Dispatch an audio message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::PickFile => pick_file(w, cx),
        Message::Load(path) => load(w, path, None, cx),
        Message::Play => play(w, cx),
        Message::Pause => pause(w, cx),
        Message::Stop => stop_and_notify(w, cx),
        Message::VolumeChanged(v) => volume_changed(w, v, cx),
        Message::ToggleLoop => toggle_loop(w, cx),
        Message::QueueAdd => queue_add(w, cx),
        Message::QueuePlayAt(i) => queue_play_at(w, i, cx),
        Message::QueueRemove(i) => queue_remove(w, i, cx),
        Message::QueueClear => queue_clear(w, cx),
    }
}

/// Stop playback and reset the transport state. Called by the
/// `ClearAudio`/`ClearAll` presenting actions.
pub(crate) fn stop(w: &mut MainWindow) {
    if let Some(player) = w.audio.player.as_ref() {
        player.stop();
    }
    w.audio.playing = false;
    // The readout follows the transport, so a stopped transport reads zero
    // rather than freezing at the position it had.
    w.audio.position_secs = 0;
}

/// Stop playback as a dispatched message (with a redraw).
fn stop_and_notify(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    stop(w);
    cx.notify();
}

/// Periodic tick while a track is playing: refreshes the position readout and
/// auto-advances the queue when the current track finishes. Looping tracks
/// never end, so they repeat in place by design.
pub(crate) fn audio_tick(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let ended = w
        .audio
        .player
        .as_ref()
        .is_some_and(|p| !p.is_paused() && p.is_empty());
    if ended {
        queue_advance(w, cx);
        return;
    }
    if let Some(ref player) = w.audio.player {
        w.audio.position_secs = player.position().as_secs();
    }
    cx.notify();
}

fn pick_file(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let path = rfd::FileDialog::new()
        .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "aac", "m4a"])
        .set_title("Open Audio File")
        .pick_file();

    let Some(path) = path else {
        return;
    };

    let path_str = path.to_string_lossy().to_string();
    load(w, path_str, None, cx);
}

/// Load a track and reflect its metadata into the audio state. `queue_index`
/// links the playback to a queue slot; `None` detaches it (manual open).
fn load_track(
    w: &mut MainWindow,
    path: String,
    queue_index: Option<usize>,
    cx: &mut Context<MainWindow>,
) {
    if w.audio.player.is_none() {
        match AudioPlayer::new() {
            Ok(player) => w.audio.player = Some(player),
            Err(e) => {
                w.set_error(format!("Audio device unavailable: {e}"));
                return;
            }
        }
    }

    // The master volume is read before the player is borrowed: the player
    // is held mutably while the track is loaded.
    let master_volume = effective_volume(w);

    if let Some(ref mut player) = w.audio.player {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // The panel's loop state is the operator's choice, so the track is
        // decoded with it: a loop toggled before the file was opened still
        // repeats, and the button and the sound agree.
        let looping = w.audio.looping;
        match player.load_with_loop(&path, looping) {
            Ok(()) => {
                // The panel's volume is the operator's choice, so the track
                // starts at it rather than at the player's own default —
                // with the master volume on top of it.
                player.set_volume(master_volume);
                w.audio.track = Some(name);
                w.audio.path = path;
                w.audio.playing = false;
                w.audio.position_secs = 0;
                w.audio.duration_secs = player.duration().map(|d| d.as_secs()).unwrap_or(0);
                w.audio.queue_index = queue_index;
            }
            Err(e) => w.set_error(audio_failure_message("audio load", &e)),
        }
    }
    cx.notify();
}

/// Public load entry point (manual open; detached from the queue).
pub(crate) fn load(
    w: &mut MainWindow,
    path: String,
    queue_index: Option<usize>,
    cx: &mut Context<MainWindow>,
) {
    load_track(w, path, queue_index, cx);
}

/// Load a track and start it playing.
///
/// A media cue in the service plan plays the file: loading it and leaving it
/// silent is a cue the room does not hear.
pub(crate) fn load_and_play(
    w: &mut MainWindow,
    path: String,
    queue_index: Option<usize>,
    cx: &mut Context<MainWindow>,
) {
    load_track(w, path, queue_index, cx);
    if w.audio.playing {
        return;
    }
    if let Some(ref player) = w.audio.player {
        player.play();
        w.audio.playing = true;
    }
    cx.notify();
}

fn play(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // A track that has run out has nothing left to play, so it is loaded
    // again first: pressing play after the end would otherwise be silent.
    let ended = w
        .audio
        .player
        .as_ref()
        .is_some_and(crate::media::AudioPlayer::is_empty);
    if ended && !w.audio.path.is_empty() {
        let path = w.audio.path.clone();
        let queue_index = w.audio.queue_index;
        load(w, path, queue_index, cx);
    }
    if let Some(ref player) = w.audio.player {
        player.play();
        w.audio.playing = true;
    }
    cx.notify();
}

fn pause(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(ref player) = w.audio.player {
        player.pause();
        w.audio.playing = false;
    }
    cx.notify();
}

fn volume_changed(w: &mut MainWindow, vol: f32, cx: &mut Context<MainWindow>) {
    w.audio.volume = vol;
    let volume = effective_volume(w);
    if let Some(ref mut player) = w.audio.player {
        player.set_volume(volume);
    }
    cx.notify();
}

/// The level the room hears.
///
/// The master volume is on top of the clip's own volume: the operator can
/// turn the whole service down without touching every clip.
fn effective_volume(w: &MainWindow) -> f32 {
    let master = f32::from(w.app_config.audio.master_volume) / 100.0;
    w.audio.volume * master
}

/// Apply the master volume to the sound that is playing.
///
/// The master is turned while the service runs as often as before it, so
/// the sound that is playing follows it at once.
pub(crate) fn apply_master_volume(w: &mut MainWindow) {
    let volume = effective_volume(w);
    if let Some(ref mut player) = w.audio.player {
        player.set_volume(volume);
    }
}

fn toggle_loop(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let looping = !w.audio.looping;
    let result = match w.audio.player.as_mut() {
        Some(player) => player.set_looping(looping),
        None => Ok(()),
    };
    // The button mirrors the player: it shows what the sound does, so it
    // follows the state the player ended up with — a failed rebuild leaves
    // the old state alone, a failed seek keeps the new one.
    w.audio.looping = effective_loop_state(looping, w.audio.player.as_ref().map(|p| p.looping()));
    if let Err(e) = result {
        w.set_error(audio_failure_message("audio loop", &e));
    }
    cx.notify();
}

/// The loop state the panel shows.
///
/// The player is the truth when there is one (a failed rebuild leaves the
/// old state alone); without a player the panel's own state is the truth,
/// and the track that is loaded next is decoded with it.
fn effective_loop_state(panel_looping: bool, player_looping: Option<bool>) -> bool {
    player_looping.unwrap_or(panel_looping)
}

/// The operator-facing message for an audio failure.
fn audio_failure_message(context: &str, e: &AudioError) -> String {
    format!("{context}: {e}")
}

/// Pick multiple audio files and append them to the queue. If nothing is
/// loaded yet, the first added track starts playing immediately.
fn queue_add(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(paths) = rfd::FileDialog::new()
        .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "aac", "m4a"])
        .set_title("Add Audio to Queue")
        .pick_files()
    else {
        return;
    };

    let mut tracks: Vec<QueueTrack> = Vec::new();
    for path in paths {
        let path_str = path.to_string_lossy().to_string();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        tracks.push(QueueTrack {
            path: path_str,
            name,
        });
    }

    let nothing_loaded = w.audio.track.is_none();
    let first_path = tracks.first().map(|t| t.path.clone());
    let first_index = w.audio.queue.len();
    w.audio.queue.extend(tracks);
    if nothing_loaded && let Some(p) = first_path {
        load_track(w, p, Some(first_index), cx);
        play(w, cx);
        return;
    }
    cx.notify();
}

/// Start playing the queue entry at `index` (it becomes the current track).
fn queue_play_at(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    let Some(track) = w.audio.queue.get(index).cloned() else {
        return;
    };
    load_track(w, track.path, Some(index), cx);
    play(w, cx);
}

/// Remove a queue entry, keeping the current-playback index consistent.
fn queue_remove(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    if index >= w.audio.queue.len() {
        return;
    }
    let was_current = w.audio.queue_index == Some(index);
    w.audio.queue.remove(index);
    w.audio.queue_index = queue_index_after_remove(w.audio.queue_index, index, w.audio.queue.len());
    if was_current {
        // The playing entry is gone from the queue, so the queue moves on:
        // the entry that took its place starts playing and the marker
        // follows it. Without this the sound would keep playing a track
        // that is no longer queued and the queue would never advance.
        let was_playing = w.audio.playing;
        match w.audio.queue_index {
            Some(next) => {
                queue_play_at(w, next, cx);
                // The transport keeps the state the operator left it in.
                if !was_playing {
                    pause(w, cx);
                }
            }
            None => {
                stop(w);
                cx.notify();
            }
        }
        return;
    }
    cx.notify();
}

/// The queue index that follows a removal.
///
/// Removing an entry before the current one shifts the current entry down.
/// Removing the current entry hands the queue to the entry that slid into
/// its place, so the queue can still advance; when the queue has ended,
/// there is no current entry and the playback stops.
fn queue_index_after_remove(
    current: Option<usize>,
    removed: usize,
    len_after: usize,
) -> Option<usize> {
    match current {
        Some(cur) if cur == removed => (removed < len_after).then_some(removed),
        Some(cur) if cur > removed => Some(cur - 1),
        other => other,
    }
}

fn queue_clear(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let was_queued = w.audio.queue_index.is_some();
    w.audio.queue.clear();
    w.audio.queue_index = None;
    if was_queued {
        // The playing entry went with the queue, so the sound stops rather
        // than playing a track that is no longer queued — and the marker
        // and the sound agree again.
        stop(w);
    }
    cx.notify();
}

/// Play the next queued track after the current one ends; stop when the queue
/// is exhausted.
fn queue_advance(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let next = match w.audio.queue_index {
        Some(i) if i + 1 < w.audio.queue.len() => Some(i + 1),
        _ => None,
    };
    match next {
        Some(i) => queue_play_at(w, i, cx),
        None => {
            w.audio.playing = false;
            w.audio.queue_index = None;
            cx.notify();
        }
    }
}

// ---------------------------------------------------------------------------
// View (right dock Audio tab)
// ---------------------------------------------------------------------------

/// Render the audio tab: transport bar plus the playback queue.
pub(crate) fn audio_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(audio_bar(w, cx))
        .child(queue_panel(w, cx))
}

/// The transport bar: track name + time readout, transport buttons, volume.
fn audio_bar(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let playing = w.audio.playing;
    let looping = w.audio.looping;
    let volume = w.audio.volume;
    let track_label = w
        .audio
        .track
        .clone()
        .unwrap_or_else(|| String::from("No track loaded"));
    let time_label = format!(
        "{} / {}",
        format_secs(w.audio.position_secs),
        format_secs(w.audio.duration_secs)
    );

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(8.))
        .bg(theme::c(theme::BG_DARK))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("AUDIO"),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(truncate(&track_label, 34)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(time_label),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    Button::new("audio-open")
                        .label("Open…")
                        .ghost()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Audio(Message::PickFile),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("audio-stop")
                        .label("Stop")
                        .ghost()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Audio(Message::Stop),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("audio-play")
                        .icon(Icon::new(if playing {
                            IconName::Pause
                        } else {
                            IconName::Play
                        }))
                        .primary()
                        .small()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            let msg = if playing {
                                Message::Pause
                            } else {
                                Message::Play
                            };
                            w.handle_message(crate::ui::messages::Message::Audio(msg), window, cx);
                        })),
                )
                .child(
                    Button::new("audio-loop")
                        .icon(Icon::new(IconName::RefreshCw))
                        .when(looping, |b| b.primary())
                        .when(!looping, |b| b.ghost())
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Audio(Message::ToggleLoop),
                                window,
                                cx,
                            );
                        })),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .child(Slider::new(&w.audio_volume_slider.clone())),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(format!("Vol {:.0}%", volume * 100.0)),
                ),
        )
}

/// Queue list below the transport bar: pending tracks, add/remove/clear.
fn queue_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let queue = w.audio.queue.clone();
    let queue_index = w.audio.queue_index;

    let mut panel = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(8.))
        .bg(theme::c(theme::BG_DARK))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("QUEUE ({})", queue.len())),
                )
                .child(div().flex_1())
                .child(
                    Button::new("queue-add")
                        .label("Add…")
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Audio(Message::QueueAdd),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("queue-clear")
                        .label("Clear")
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Audio(Message::QueueClear),
                                window,
                                cx,
                            );
                        })),
                ),
        );

    if queue.is_empty() {
        panel = panel.child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Queue is empty — add tracks and they play one after another."),
        );
    } else {
        for (i, track) in queue.iter().enumerate() {
            let is_current = queue_index == Some(i);
            let name = if is_current {
                format!("▶ {}", track.name)
            } else {
                format!("{}· {}", i + 1, track.name)
            };
            panel = panel.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        Button::new(SharedString::from(format!("queue-play-{i}")))
                            .label(truncate(&name, 36))
                            .ghost()
                            .xsmall()
                            .flex_1()
                            .on_click(cx.listener(move |w, _, window, cx| {
                                w.handle_message(
                                    crate::ui::messages::Message::Audio(Message::QueuePlayAt(i)),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("queue-remove-{i}")))
                            .label("×")
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |w, _, window, cx| {
                                w.handle_message(
                                    crate::ui::messages::Message::Audio(Message::QueueRemove(i)),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            );
        }
    }

    panel
}

/// `m:ss` readout for the transport bar; `0:00` while unknown.
fn format_secs(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removing_the_current_entry_hands_the_queue_to_the_next_one() {
        // The entry that slid into the removed slot takes over, so the
        // queue can still advance.
        assert_eq!(queue_index_after_remove(Some(0), 0, 2), Some(0));
        assert_eq!(queue_index_after_remove(Some(1), 1, 2), Some(1));
    }

    #[test]
    fn removing_the_last_current_entry_ends_the_queue() {
        assert_eq!(queue_index_after_remove(Some(2), 2, 2), None);
    }

    #[test]
    fn removing_an_earlier_entry_shifts_the_current_one_down() {
        assert_eq!(queue_index_after_remove(Some(2), 0, 3), Some(1));
    }

    #[test]
    fn removing_an_unrelated_entry_keeps_the_current_one() {
        assert_eq!(queue_index_after_remove(Some(0), 1, 3), Some(0));
        assert_eq!(queue_index_after_remove(None, 0, 3), None);
    }

    #[test]
    fn the_loop_button_mirrors_the_player() {
        // Without a player the panel's own state is the truth.
        assert!(effective_loop_state(true, None));
        assert!(!effective_loop_state(false, None));
        // With a player, the player is the truth.
        assert!(!effective_loop_state(true, Some(false)));
        assert!(effective_loop_state(false, Some(true)));
    }
}
