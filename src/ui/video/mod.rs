use crate::domain::SlideContent;
use crate::media::MediaPlayer;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::Task;
use iced::widget::image::Handle as ImageHandle;

/// Messages owned by the Video feature module (see `AGENTS.md`).
///
/// `VideoFrameTick` stays as a root variant (it is injected by the per-frame
/// subscription, not produced by a UI widget).
///
/// NOTE: the video transport controls are rendered inline inside the editor
/// inspector; this module only owns the message enum and dispatch logic.
#[derive(Debug, Clone)]
pub enum Message {
    PlayToggled,
    LoopToggled(bool),
    VolumeChanged(f32),
    SeekChanged(f64),
    MuteToggled,
    SpeedChanged(f64),
}

/// Dispatch a video message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::PlayToggled => video_play_toggled(w),
        Message::LoopToggled(l) => video_loop_toggled(w, l),
        Message::VolumeChanged(v) => video_volume_changed(w, v),
        Message::SeekChanged(s) => video_seek_changed(w, s),
        Message::MuteToggled => video_mute_toggled(w),
        Message::SpeedChanged(s) => video_speed_changed(w, s),
    }
}

pub(crate) fn video_frame_tick(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref player) = w.video.player {
        if let Some(frame) = player.current_frame() {
            let handle = ImageHandle::from_rgba(frame.width, frame.height, frame.data);
            w.video.frame = Some(handle);
        }
        w.video.position = player.position_secs();
    }
    Task::none()
}

pub(crate) fn open_video(w: &mut MainWindow, path: &str) {
    if let Some(ref player) = w.video.player {
        player.stop();
    }
    w.video.player = None;
    w.video.frame = None;
    w.video.position = 0.0;

    if path.is_empty() {
        return;
    }

    match MediaPlayer::open(path) {
        Ok(mut player) => {
            player.set_loop(w.video.looping);
            player.set_volume(w.video.volume);
            player.set_mute(w.video.muted);
            player.set_speed(w.video.speed);
            player.play();
            if let Some(ref ndi) = w.presenting.ndi_output {
                ndi.set_video_frame_source(Some(player.video.shared_frame_arc()));
            }
            w.video.player = Some(player);
        }
        Err(e) => w.set_error(format!("MediaPlayer: failed to open '{path}': {e}")),
    }
}

pub(crate) fn stop_video(w: &mut MainWindow) {
    if let Some(ref player) = w.video.player {
        player.stop();
    }
    if let Some(ref ndi) = w.presenting.ndi_output {
        ndi.set_video_frame_source(None);
    }
    w.video.player = None;
    w.video.frame = None;
    w.video.position = 0.0;
}

pub(crate) fn video_play_toggled(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref player) = w.video.player {
        player.toggle_play_pause();
    }
    Task::none()
}

pub(crate) fn video_loop_toggled(w: &mut MainWindow, looping: bool) -> Task<RootMessage> {
    w.video.looping = looping;
    if let Some(ref mut player) = w.video.player {
        player.set_loop(looping);
    }
    Task::none()
}

pub(crate) fn video_volume_changed(w: &mut MainWindow, volume: f32) -> Task<RootMessage> {
    w.video.volume = volume;
    if let Some(ref mut player) = w.video.player {
        player.set_volume(volume);
    }
    Task::none()
}

pub(crate) fn video_seek_changed(w: &mut MainWindow, secs: f64) -> Task<RootMessage> {
    w.video.position = secs;
    if let Some(ref player) = w.video.player {
        player.seek(secs);
    }
    Task::none()
}

pub(crate) fn video_mute_toggled(w: &mut MainWindow) -> Task<RootMessage> {
    w.video.muted = !w.video.muted;
    if let Some(ref mut player) = w.video.player {
        player.set_mute(w.video.muted);
    }
    Task::none()
}

pub(crate) fn video_speed_changed(w: &mut MainWindow, speed: f64) -> Task<RootMessage> {
    w.video.speed = speed.max(0.1);
    if let Some(ref mut player) = w.video.player {
        player.set_speed(w.video.speed);
    }
    Task::none()
}

pub(crate) fn on_presenter_slide_changed(w: &mut MainWindow, slide_index: usize) {
    let path = w
        .presenting
        .presentation
        .as_ref()
        .and_then(|p| p.slides.get(slide_index))
        .and_then(|s| {
            if let SlideContent::Video { path, .. } = &s.content {
                Some(path.clone())
            } else {
                None
            }
        });

    match path {
        Some(p) => open_video(w, &p),
        None => stop_video(w),
    }
}
