use crate::media::AudioPlayer;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::state::{AudioState, QueueTrack};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Task,
    widget::{Space, button, column, container, row, scrollable, slider, text},
};
use iced_font_awesome::fa_icon_solid;

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
    TogglePanel,
    /// Pick multiple audio files and append them to the playback queue.
    QueueAdd,
    QueuePlayAt(usize),
    QueueRemove(usize),
    QueueClear,
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Audio(msg)
}

/// Render the audio tab: transport bar plus the playback queue.
pub(crate) fn view<'a>(audio: &'a AudioState) -> Element<'a, RootMessage> {
    column![
        audio_bar(
            audio.track.as_deref(),
            audio.playing,
            audio.volume,
            audio.looping,
            audio.position_secs,
            audio.duration_secs,
        ),
        queue_panel(audio),
    ]
    .spacing(6)
    .into()
}

/// Queue list below the transport bar: pending tracks, add/remove/clear.
fn queue_panel<'a>(audio: &'a AudioState) -> Element<'a, RootMessage> {
    let header = row![
        text(format!("QUEUE ({})", audio.queue.len()))
            .size(10)
            .color(theme::TEXT_MUTED),
        Space::new().width(Length::Fill),
        button(text("Add…").size(10))
            .on_press(wrap(Message::QueueAdd))
            .padding([3, 8])
            .style(theme::ghost_button),
        button(text("Clear").size(10))
            .on_press(wrap(Message::QueueClear))
            .padding([3, 8])
            .style(theme::ghost_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let mut list = column![header].spacing(4).padding([4, 8]);

    if audio.queue.is_empty() {
        list = list.push(
            text("Queue is empty — add tracks and they play one after another.")
                .size(10)
                .color(theme::TEXT_MUTED),
        );
    } else {
        let rows: Vec<Element<'a, RootMessage>> = audio
            .queue
            .iter()
            .enumerate()
            .map(|(i, track)| {
                let is_current = audio.queue_index == Some(i);
                let name_text = if is_current {
                    text(format!("▶ {}", track.name))
                        .size(11)
                        .color(theme::LIVE_GREEN)
                } else {
                    text(format!("{}· {}", i + 1, track.name))
                        .size(11)
                        .color(theme::TEXT_SECONDARY)
                };
                row![
                    button(name_text)
                        .on_press(wrap(Message::QueuePlayAt(i)))
                        .padding([3, 6])
                        .style(theme::ghost_button),
                    Space::new().width(Length::Fill),
                    button(fa_icon_solid("xmark").size(9.0_f32))
                        .on_press(wrap(Message::QueueRemove(i)))
                        .padding([3, 6])
                        .style(theme::ghost_button),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
            })
            .collect();
        list = list.push(column(rows).spacing(2));
    }

    container(scrollable(list).height(Length::Fill))
        .height(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

/// Dispatch an audio message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::PickFile => pick_file(w),
        Message::Load(path) => load(w, path),
        Message::Play => play(w),
        Message::Pause => pause(w),
        Message::Stop => stop(w),
        Message::VolumeChanged(v) => volume_changed(w, v),
        Message::ToggleLoop => toggle_loop(w),
        Message::TogglePanel => toggle_panel(w),
        Message::QueueAdd => queue_add(w),
        Message::QueuePlayAt(i) => queue_play_at(w, i),
        Message::QueueRemove(i) => queue_remove(w, i),
        Message::QueueClear => queue_clear(w),
    }
}

/// Periodic tick while a track is playing: refreshes the position readout and
/// auto-advances the queue when the current track finishes. Looping tracks
/// never end, so they repeat in place by design.
pub(crate) fn audio_tick(w: &mut MainWindow) -> Task<RootMessage> {
    let ended = w
        .audio
        .player
        .as_ref()
        .is_some_and(|p| !p.is_paused() && p.is_empty());
    if ended {
        return queue_advance(w);
    }
    if let Some(ref player) = w.audio.player {
        w.audio.position_secs = player.position().as_secs();
    }
    Task::none()
}

pub(crate) fn pick_file(w: &mut MainWindow) -> Task<RootMessage> {
    let path = rfd::FileDialog::new()
        .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "aac", "m4a"])
        .set_title("Open Audio File")
        .pick_file();

    let Some(path) = path else {
        return Task::none();
    };

    let path_str = path.to_string_lossy().to_string();
    load(w, path_str)
}

pub(crate) fn load(w: &mut MainWindow, path: String) -> Task<RootMessage> {
    load_track(w, path, None)
}

/// Load a track and reflect its metadata into the audio state. `queue_index`
/// links the playback to a queue slot; `None` detaches it (manual open).
fn load_track(w: &mut MainWindow, path: String, queue_index: Option<usize>) -> Task<RootMessage> {
    if w.audio.player.is_none() {
        match AudioPlayer::new() {
            Ok(player) => w.audio.player = Some(player),
            Err(e) => {
                w.ui.error_message = Some(format!("Audio device unavailable: {e}"));
                return Task::none();
            }
        }
    }

    if let Some(ref mut player) = w.audio.player {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        match player.load(&path) {
            Ok(()) => {
                w.audio.track = Some(name);
                w.audio.path = path;
                w.audio.playing = false;
                w.audio.position_secs = 0;
                w.audio.duration_secs = player.duration().map(|d| d.as_secs()).unwrap_or(0);
                w.audio.queue_index = queue_index;
            }
            Err(e) => w.ui.error_message = Some(format!("audio load: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn play(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref player) = w.audio.player {
        player.play();
        w.audio.playing = true;
    }
    Task::none()
}

pub(crate) fn pause(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref player) = w.audio.player {
        player.pause();
        w.audio.playing = false;
    }
    Task::none()
}

pub(crate) fn stop(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref player) = w.audio.player {
        player.stop();
        w.audio.playing = false;
    }
    Task::none()
}

pub(crate) fn volume_changed(w: &mut MainWindow, vol: f32) -> Task<RootMessage> {
    w.audio.volume = vol;
    if let Some(ref mut player) = w.audio.player {
        player.set_volume(vol);
    }
    Task::none()
}

pub(crate) fn toggle_loop(w: &mut MainWindow) -> Task<RootMessage> {
    let looping = !w.audio.looping;
    if let Some(ref mut player) = w.audio.player
        && let Err(e) = player.set_looping(looping)
    {
        w.ui.error_message = Some(format!("audio loop: {e}"));
        return Task::none();
    }
    w.audio.looping = looping;
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.audio.panel_visible = !w.audio.panel_visible;
    Task::none()
}

/// Pick multiple audio files and append them to the queue. If nothing is
/// loaded yet, the first added track starts playing immediately.
pub(crate) fn queue_add(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(paths) = rfd::FileDialog::new()
        .add_filter("Audio", &["mp3", "wav", "flac", "ogg", "aac", "m4a"])
        .set_title("Add Audio to Queue")
        .pick_files()
    else {
        return Task::none();
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
        let load_task = load_track(w, p, Some(first_index));
        let play_task = play(w);
        return Task::batch([load_task, play_task]);
    }
    Task::none()
}

/// Start playing the queue entry at `index` (it becomes the current track).
pub(crate) fn queue_play_at(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    let Some(track) = w.audio.queue.get(index).cloned() else {
        return Task::none();
    };
    let load_task = load_track(w, track.path, Some(index));
    let play_task = play(w);
    Task::batch([load_task, play_task])
}

/// Remove a queue entry, keeping the current-playback index consistent.
pub(crate) fn queue_remove(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    if index >= w.audio.queue.len() {
        return Task::none();
    }
    w.audio.queue.remove(index);
    w.audio.queue_index = match w.audio.queue_index {
        Some(cur) if cur == index => None, // removed the playing entry
        Some(cur) if cur > index => Some(cur - 1), // shift entries after it
        other => other,
    };
    Task::none()
}

pub(crate) fn queue_clear(w: &mut MainWindow) -> Task<RootMessage> {
    w.audio.queue.clear();
    w.audio.queue_index = None;
    Task::none()
}

/// Play the next queued track after the current one ends; stop when the queue
/// is exhausted.
fn queue_advance(w: &mut MainWindow) -> Task<RootMessage> {
    let next = match w.audio.queue_index {
        Some(i) if i + 1 < w.audio.queue.len() => Some(i + 1),
        _ => None,
    };
    match next {
        Some(i) => queue_play_at(w, i),
        None => {
            w.audio.playing = false;
            w.audio.queue_index = None;
            Task::none()
        }
    }
}

pub fn audio_bar<'a>(
    track_name: Option<&'a str>,
    is_playing: bool,
    volume: f32,
    looping: bool,
    position_secs: u64,
    duration_secs: u64,
) -> Element<'a, RootMessage> {
    let track_label = track_name
        .map(|t| text(t).size(12))
        .unwrap_or_else(|| text("No track loaded").size(12));

    let time_label = text(format!(
        "{} / {}",
        format_secs(position_secs),
        format_secs(duration_secs)
    ))
    .size(11)
    .color(theme::TEXT_SECONDARY);

    let btn_size = 11.0f32;
    let open_btn = button(text("Open…").size(btn_size))
        .on_press(wrap(Message::PickFile))
        .padding([5u16, 10u16])
        .style(theme::ghost_button);

    let play_pause = if is_playing {
        button(fa_icon_solid("pause").size(btn_size)).on_press(wrap(Message::Pause))
    } else {
        button(fa_icon_solid("play").size(btn_size)).on_press(wrap(Message::Play))
    };
    let play_pause = play_pause
        .padding([5u16, 10u16])
        .style(theme::primary_button);

    let stop_btn = button(fa_icon_solid("stop").size(btn_size))
        .on_press(wrap(Message::Stop))
        .padding([5u16, 10u16])
        .style(theme::ghost_button);

    let loop_btn = button(fa_icon_solid("repeat").size(btn_size))
        .on_press(wrap(Message::ToggleLoop))
        .padding([5u16, 10u16])
        .style(if looping {
            theme::primary_button
        } else {
            theme::ghost_button
        });

    let vol_slider = slider(0.0..=1.0, volume, move |v| wrap(Message::VolumeChanged(v)))
        .step(0.01_f32)
        .width(80);

    let vol_label = text(format!("Vol {:.0}%", volume * 100.0)).size(11);

    container(
        row![
            open_btn,
            Space::new().width(8),
            track_label,
            Space::new().width(Length::Fill),
            time_label,
            stop_btn,
            play_pause,
            loop_btn,
            Space::new().width(8),
            vol_slider,
            vol_label,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([6u16, 12u16]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

/// `m:ss` readout for the transport bar; `0:00` while unknown.
fn format_secs(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}
