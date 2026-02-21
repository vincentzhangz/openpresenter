use crate::media::MediaPlayer;
use crate::slides::SlideContent;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;
use iced::widget::image::Handle as ImageHandle;

pub(crate) fn video_frame_tick(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref player) = w.media_player {
        if let Some(frame) = player.current_frame() {
            let handle = ImageHandle::from_rgba(frame.width, frame.height, frame.data);
            w.video_frame = Some(handle);
        }
        w.video_position = player.position_secs();
    }
    Task::none()
}

pub(crate) fn open_video(w: &mut MainWindow, path: &str) {
    if let Some(ref player) = w.media_player {
        player.stop();
    }
    w.media_player = None;
    w.video_frame = None;
    w.video_position = 0.0;

    if path.is_empty() {
        return;
    }

    match MediaPlayer::open(path) {
        Ok(mut player) => {
            player.set_loop(w.video_looping);
            player.set_volume(w.video_volume);
            player.set_mute(w.video_muted);
            player.set_speed(w.video_speed);
            player.play();
            if let Some(ref ndi) = w.ndi_output {
                ndi.set_video_frame_source(Some(player.video.shared_frame_arc()));
            }
            w.media_player = Some(player);
        }
        Err(e) => eprintln!("MediaPlayer: failed to open '{path}': {e}"),
    }
}

pub(crate) fn stop_video(w: &mut MainWindow) {
    if let Some(ref player) = w.media_player {
        player.stop();
    }
    if let Some(ref ndi) = w.ndi_output {
        ndi.set_video_frame_source(None);
    }
    w.media_player = None;
    w.video_frame = None;
    w.video_position = 0.0;
}

pub(crate) fn video_play_toggled(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref player) = w.media_player {
        player.toggle_play_pause();
    }
    Task::none()
}

pub(crate) fn video_loop_toggled(w: &mut MainWindow, looping: bool) -> Task<Message> {
    w.video_looping = looping;
    if let Some(ref mut player) = w.media_player {
        player.set_loop(looping);
    }
    Task::none()
}

pub(crate) fn video_volume_changed(w: &mut MainWindow, volume: f32) -> Task<Message> {
    w.video_volume = volume;
    if let Some(ref mut player) = w.media_player {
        player.set_volume(volume);
    }
    Task::none()
}

pub(crate) fn video_seek_changed(w: &mut MainWindow, secs: f64) -> Task<Message> {
    w.video_position = secs;
    if let Some(ref player) = w.media_player {
        player.seek(secs);
    }
    Task::none()
}

pub(crate) fn video_mute_toggled(w: &mut MainWindow) -> Task<Message> {
    w.video_muted = !w.video_muted;
    if let Some(ref mut player) = w.media_player {
        player.set_mute(w.video_muted);
    }
    Task::none()
}

pub(crate) fn video_speed_changed(w: &mut MainWindow, speed: f64) -> Task<Message> {
    w.video_speed = speed.max(0.1);
    if let Some(ref mut player) = w.media_player {
        player.set_speed(w.video_speed);
    }
    Task::none()
}

pub(crate) fn on_presenter_slide_changed(w: &mut MainWindow, slide_index: usize) {
    let path = w
        .presenting_presentation
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
