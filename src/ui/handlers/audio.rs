use crate::media::AudioPlayer;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub(crate) fn pick_file(w: &mut MainWindow) -> Task<Message> {
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

pub(crate) fn load(w: &mut MainWindow, path: String) -> Task<Message> {
    if w.audio.player.is_none() {
        match AudioPlayer::new() {
            Ok(player) => w.audio.player = Some(player),
            Err(e) => {
                w.error_message = Some(format!("Audio device unavailable: {e}"));
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
            Err(e) => w.error_message = Some(format!("audio load: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn play(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref player) = w.audio.player {
        player.play();
        w.audio.playing = true;
    }
    Task::none()
}

pub(crate) fn pause(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref player) = w.audio.player {
        player.pause();
        w.audio.playing = false;
    }
    Task::none()
}

pub(crate) fn stop(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref player) = w.audio.player {
        player.stop();
        w.audio.playing = false;
    }
    Task::none()
}

pub(crate) fn volume_changed(w: &mut MainWindow, vol: f32) -> Task<Message> {
    w.audio.volume = vol;
    if let Some(ref mut player) = w.audio.player {
        player.set_volume(vol);
    }
    Task::none()
}

pub(crate) fn toggle_loop(w: &mut MainWindow) -> Task<Message> {
    w.audio.looping = !w.audio.looping;
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<Message> {
    w.audio.panel_visible = !w.audio.panel_visible;
    Task::none()
}
