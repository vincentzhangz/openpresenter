use crate::media::AudioPlayer;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::state::AudioState;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Task,
    widget::{Space, button, container, row, slider, text},
};
use iced_font_awesome::fa_icon_solid;

/// Messages owned by the Audio feature module (see `AGENTS.md`).
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
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Audio(msg)
}

/// Render the audio transport bar.
pub(crate) fn view<'a>(audio: &'a AudioState) -> Element<'a, RootMessage> {
    audio_bar(
        audio.track.as_deref(),
        audio.playing,
        audio.volume,
        audio.looping,
    )
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
    }
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
    if w.audio.player.is_none() {
        match AudioPlayer::new() {
            Ok(player) => w.audio.player = Some(player),
            Err(e) => {
                w.ui.error_message = Some(format!("Audio device unavailable: {e}"));
                return Task::none();
            }
        }
    }

    if let Some(ref player) = w.audio.player {
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
    w.audio.looping = !w.audio.looping;
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.audio.panel_visible = !w.audio.panel_visible;
    Task::none()
}

pub fn audio_bar<'a>(
    track_name: Option<&'a str>,
    is_playing: bool,
    volume: f32,
    looping: bool,
) -> Element<'a, RootMessage> {
    let track_label = track_name
        .map(|t| text(t).size(12))
        .unwrap_or_else(|| text("No track loaded").size(12));

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
