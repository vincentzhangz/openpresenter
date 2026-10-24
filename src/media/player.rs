use crate::media::{AudioPlayer, VideoDecoder, decoder::RgbaFrame};
use anyhow::Result;
use ffmpeg_next as ffmpeg;

/// Whether a media file carries an audio track.
///
/// A clip that is silent by design is not an error, so the audio is only
/// reported as missing when the file really has sound in it. A file that
/// cannot be probed is assumed to have audio, so a real fault is still
/// reported.
pub fn has_audio_track(path: &str) -> bool {
    ffmpeg::init().ok();
    match ffmpeg::format::input(path) {
        Ok(input) => input
            .streams()
            .any(|stream| stream.parameters().medium() == ffmpeg::media::Type::Audio),
        Err(_) => true,
    }
}
use std::time::Duration;

pub struct MediaPlayer {
    pub video: VideoDecoder,
    audio: Option<AudioPlayer>,
    /// The file this player was opened with.
    path: String,
    looping: bool,
    volume: f32,
    muted: bool,
    /// Non-fatal audio problem detected at open time, surfaced once by the UI.
    audio_error: Option<String>,
}

impl MediaPlayer {
    /// The file this player is playing.
    ///
    /// The editor's transport belongs to the slide being edited, so the UI
    /// has to be able to tell whether the player is the one for that
    /// slide's file.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Open a file, with the clip looping or not.
    ///
    /// The loop state is part of the open, because the sound is built with
    /// it: setting the loop afterwards would decode the soundtrack a second
    /// time, on the thread that draws the window.
    pub fn open_with_loop(path: &str, looping: bool) -> Result<Self> {
        let video = VideoDecoder::open(path)?;

        // Distinguish "file genuinely has no audio track" (fine — many clips are
        // silent) from "audio backend failed" (an operator-visible fault). Only
        // the latter is worth reporting.
        let (audio, audio_error) = match AudioPlayer::new() {
            Err(e) => (None, Some(format!("audio output unavailable: {e}"))),
            Ok(mut player) => match player.load_with_loop(path, looping) {
                Ok(()) => (Some(player), None),
                Err(e) => {
                    if has_audio_track(path) {
                        (
                            None,
                            Some(format!("could not load audio from '{path}': {e}")),
                        )
                    } else {
                        // The file carries no sound: that is a silent clip, not
                        // a fault, so the operator is not shown an error for
                        // every video they play.
                        (None, None)
                    }
                }
            },
        };

        Ok(Self {
            video,
            audio,
            path: path.to_string(),
            looping,
            volume: 1.0,
            muted: false,
            audio_error,
        })
    }

    /// Open a file without a loop.
    pub fn open(path: &str) -> Result<Self> {
        Self::open_with_loop(path, false)
    }

    pub fn play(&self) {
        self.video.play();
        if let Some(ref a) = self.audio {
            a.play();
        }
    }

    pub fn pause(&self) {
        self.video.pause();
        if let Some(ref a) = self.audio {
            a.pause();
        }
    }

    pub fn stop(&self) {
        self.video.stop();
        if let Some(ref a) = self.audio {
            a.stop();
        }
    }

    pub fn toggle_play_pause(&self) {
        if self.video.is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn set_loop(&mut self, looping: bool) {
        if self.looping == looping {
            return;
        }
        self.looping = looping;
        self.video.set_loop(looping);
        // The sound loops with the picture: a video that repeats silently
        // after the first run is a broken show.
        if let Some(ref mut a) = self.audio
            && let Err(e) = a.set_looping(looping)
        {
            // The sound that does not loop with the picture is a broken show,
            // so the operator is told.
            self.audio_error = Some(format!("could not set the sound to loop: {e}"));
        }
    }

    /// Whether the clip ran to its end.
    ///
    /// The decode thread stops at the end of the file, so a finished clip
    /// has to be opened again before it can play a second time.
    pub fn has_ended(&self) -> bool {
        self.video.has_ended()
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Take a pending fatal decode error, if the video decoder reported one.
    pub fn take_decode_error(&self) -> Option<String> {
        self.video.take_error()
    }

    /// Take the non-fatal audio problem detected when this player was opened.
    pub fn take_audio_error(&self) -> Option<String> {
        self.audio_error.clone()
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if !self.muted
            && let Some(ref mut a) = self.audio
        {
            a.set_volume(self.volume);
        }
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_mute(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(ref mut a) = self.audio {
            a.set_volume(if muted { 0.0 } else { self.volume });
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.video.set_speed(speed);
        // The sound follows the picture's speed, or it drifts against it.
        if let Some(ref mut a) = self.audio {
            a.set_speed(speed);
        }
    }

    pub fn speed(&self) -> f64 {
        self.video.speed
    }

    pub fn is_playing(&self) -> bool {
        self.video.is_playing()
    }

    pub fn current_frame(&self) -> Option<RgbaFrame> {
        self.video.current_frame()
    }

    /// The identity of the current frame, without copying its pixels.
    pub fn current_frame_identity(&self) -> Option<u64> {
        self.video.current_frame_identity()
    }

    pub fn width(&self) -> u32 {
        self.video.width
    }

    pub fn height(&self) -> u32 {
        self.video.height
    }

    /// Move the picture and the sound to a new position.
    ///
    /// The track follows the picture. Stopping the audio here used to leave
    /// the video playing silently for the rest of the show. A seek the
    /// sound cannot make is reported: a clip whose container cannot be
    /// seeked would otherwise restart its sound from the beginning while
    /// the picture jumped.
    pub fn seek(&mut self, secs: f64) -> Result<()> {
        self.video.seek(secs);
        if let Some(ref mut a) = self.audio {
            a.seek(Duration::from_secs_f64(secs.max(0.0)))?;
        }
        Ok(())
    }

    pub fn position_secs(&self) -> f64 {
        self.video.position_secs()
    }

    pub fn duration_secs(&self) -> f64 {
        self.video.duration_secs
    }
}
