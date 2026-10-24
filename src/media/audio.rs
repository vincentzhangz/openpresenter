use rodio::decoder::DecoderError;
use rodio::source::SeekError;
use rodio::{
    Decoder as RodioDecoder, DeviceSinkBuilder, DeviceSinkError, MixerDeviceSink, Player, Source,
};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

/// A failure in the audio playback layer.
///
/// The layer cannot talk to the operator, so it reports a typed error and
/// the UI turns it into a message.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// The audio output device could not be opened.
    #[error("{0}")]
    Output(#[source] DeviceSinkError),

    /// The file could not be opened.
    #[error("open '{path}': {source}")]
    Open {
        /// The file that could not be opened.
        path: String,
        /// The reason the file system gave.
        #[source]
        source: std::io::Error,
    },

    /// The file could not be decoded.
    #[error("decode '{path}': {source}")]
    Decode {
        /// The file that could not be decoded.
        path: String,
        /// The reason the decoder gave.
        #[source]
        source: DecoderError,
    },

    /// The track could not be moved to the requested position.
    ///
    /// A container whose decoder cannot seek would restart from the
    /// beginning, so the failure is reported instead of being swallowed.
    #[error("cannot seek to {position:?}: {source}")]
    Seek {
        /// The position the track was asked to move to.
        position: Duration,
        /// The reason the source gave.
        #[source]
        source: SeekError,
    },
}

/// A decoded audio source and the length of the track, when the decoder
/// reports it.
type DecodedSource = (Box<dyn rodio::Source<Item = f32> + Send>, Option<Duration>);

pub struct AudioPlayer {
    player: Player,
    _sink: MixerDeviceSink,
    volume: f32,
    looping: bool,
    path: Option<String>,
    /// Total length of the loaded track, when the decoder reports it.
    duration: Option<Duration>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, AudioError> {
        let sink = DeviceSinkBuilder::open_default_sink().map_err(AudioError::Output)?;
        let player = Player::connect_new(sink.mixer());
        Ok(Self {
            player,
            _sink: sink,
            volume: 1.0,
            looping: false,
            path: None,
            duration: None,
        })
    }

    /// Load a track, replacing whatever was playing.
    ///
    /// The track repeats when the player is set to loop.
    pub fn load(&mut self, path: &str) -> Result<(), AudioError> {
        let looping = self.looping;
        self.load_with_loop(path, looping)
    }

    /// Load a track and repeat it when `looping` is set.
    ///
    /// The loop state is passed in instead of being read from the player,
    /// because the loop button can be flipped before a file is opened: the
    /// button and the sound have to agree on the track that is loaded next.
    pub fn load_with_loop(&mut self, path: &str, looping: bool) -> Result<(), AudioError> {
        let (source, duration) = Self::decode(path, looping)?;
        self.player.clear();
        self.player.append(source);
        self.player.pause();
        self.path = Some(path.to_string());
        self.duration = duration;
        self.looping = looping;
        Ok(())
    }

    /// Enable or disable endless repeat of the loaded track.
    ///
    /// Switching rebuilds the playback source from the stored file and resumes
    /// at the previous position, so toggling mid-song does not restart it.
    pub fn set_looping(&mut self, looping: bool) -> Result<(), AudioError> {
        if looping == self.looping {
            return Ok(());
        }
        let Some(ref path) = self.path else {
            self.looping = looping;
            return Ok(());
        };
        let position = self.position();
        let was_paused = self.is_paused();
        let (source, _) = Self::decode(path, looping)?;
        self.player.clear();
        self.player.append(source);
        // The rebuilt source carries the new state, so it belongs to the
        // player even when the seek to the old position fails afterwards.
        self.looping = looping;
        let seek = self
            .player
            .try_seek(position)
            .map_err(|source| AudioError::Seek { position, source });
        if was_paused {
            self.player.pause();
        } else {
            self.player.play();
        }
        seek
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    /// Seek the loaded track to a position.
    ///
    /// rodio cannot move a source that is already playing, so the track is
    /// rebuilt from the file and the playback continues from the new
    /// position. It keeps playing (or stays paused) as it was.
    ///
    /// A decoder that cannot seek reports the failure instead of silently
    /// restarting the track from the beginning.
    pub fn seek(&mut self, position: Duration) -> Result<(), AudioError> {
        let Some(ref path) = self.path else {
            return Ok(());
        };
        let was_paused = self.is_paused();
        let (source, _) = Self::decode(path, self.looping)?;
        self.player.clear();
        self.player.append(source);
        let seek = self
            .player
            .try_seek(position)
            .map_err(|source| AudioError::Seek { position, source });
        if was_paused {
            self.player.pause();
        } else {
            self.player.play();
        }
        seek
    }

    /// The playback speed of the track.
    pub fn set_speed(&mut self, speed: f64) {
        self.player.set_speed(speed.max(0.1) as f32);
    }

    fn decode(path: &str, looping: bool) -> Result<DecodedSource, AudioError> {
        let file = File::open(path).map_err(|source| AudioError::Open {
            path: path.to_string(),
            source,
        })?;
        let source =
            RodioDecoder::new(BufReader::new(file)).map_err(|source| AudioError::Decode {
                path: path.to_string(),
                source,
            })?;
        let duration = source.total_duration();
        if looping {
            Ok((Box::new(source.repeat_infinite()), duration))
        } else {
            Ok((Box::new(source), duration))
        }
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos()
    }

    /// Total length of the loaded track, if known.
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }
}
