use anyhow::{Result, anyhow};
use rodio::{Decoder as RodioDecoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

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
    pub fn new() -> Result<Self> {
        let sink =
            DeviceSinkBuilder::open_default_sink().map_err(|e| anyhow!("audio output: {e:?}"))?;
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

    pub fn load(&mut self, path: &str) -> Result<()> {
        let (source, duration) = Self::decode(path, self.looping)?;
        self.player.clear();
        self.player.append(source);
        self.player.pause();
        self.path = Some(path.to_string());
        self.duration = duration;
        Ok(())
    }

    /// Enable or disable endless repeat of the loaded track.
    ///
    /// Switching rebuilds the playback source from the stored file and resumes
    /// at the previous position, so toggling mid-song does not restart it.
    pub fn set_looping(&mut self, looping: bool) -> Result<()> {
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
        self.player.try_seek(position).ok();
        if was_paused {
            self.player.pause();
        } else {
            self.player.play();
        }
        self.looping = looping;
        Ok(())
    }

    pub fn looping(&self) -> bool {
        self.looping
    }

    fn decode(
        path: &str,
        looping: bool,
    ) -> Result<(Box<dyn rodio::Source<Item = f32> + Send>, Option<Duration>)> {
        let file = File::open(path).map_err(|e| anyhow!("open '{path}': {e}"))?;
        let source =
            RodioDecoder::new(BufReader::new(file)).map_err(|e| anyhow!("decode '{path}': {e}"))?;
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
