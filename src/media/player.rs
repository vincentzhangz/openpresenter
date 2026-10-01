use crate::media::{AudioPlayer, VideoDecoder, decoder::RgbaFrame};
use anyhow::Result;

pub struct MediaPlayer {
    pub video: VideoDecoder,
    audio: Option<AudioPlayer>,
    looping: bool,
    volume: f32,
    muted: bool,
    /// Non-fatal audio problem detected at open time, surfaced once by the UI.
    audio_error: Option<String>,
}

impl MediaPlayer {
    pub fn open(path: &str) -> Result<Self> {
        let video = VideoDecoder::open(path)?;

        // Distinguish "file genuinely has no audio track" (fine — many clips are
        // silent) from "audio backend failed" (an operator-visible fault). Only
        // the latter is worth reporting.
        let (audio, audio_error) = match AudioPlayer::new() {
            Err(e) => (None, Some(format!("audio output unavailable: {e}"))),
            Ok(mut player) => match player.load(path) {
                Ok(()) => (Some(player), None),
                Err(e) => (
                    None,
                    Some(format!("could not load audio from '{path}': {e}")),
                ),
            },
        };

        Ok(Self {
            video,
            audio,
            looping: false,
            volume: 1.0,
            muted: false,
            audio_error,
        })
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
        self.looping = looping;
        self.video.set_loop(looping);
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

    pub fn width(&self) -> u32 {
        self.video.width
    }

    pub fn height(&self) -> u32 {
        self.video.height
    }

    pub fn seek(&self, secs: f64) {
        self.video.seek(secs);
        if let Some(ref a) = self.audio {
            a.stop();
        }
    }

    pub fn position_secs(&self) -> f64 {
        self.video.position_secs()
    }

    pub fn duration_secs(&self) -> f64 {
        self.video.duration_secs
    }
}
