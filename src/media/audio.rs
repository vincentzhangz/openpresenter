use anyhow::{Result, anyhow};
use rodio::{Decoder as RodioDecoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::File;
use std::io::BufReader;

pub struct AudioPlayer {
    player: Player,
    _sink: MixerDeviceSink,
    volume: f32,
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
        })
    }

    pub fn load(&self, path: &str) -> Result<()> {
        let file = File::open(path).map_err(|e| anyhow!("open '{path}': {e}"))?;
        let source =
            RodioDecoder::new(BufReader::new(file)).map_err(|e| anyhow!("decode '{path}': {e}"))?;
        self.player.clear();
        self.player.append(source);
        self.player.pause();
        Ok(())
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
