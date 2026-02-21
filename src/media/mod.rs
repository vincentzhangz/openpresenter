pub mod audio;
pub mod decoder;
pub mod player;

pub use audio::AudioPlayer;
pub use decoder::{VideoDecoder, extract_thumbnail};
pub use player::MediaPlayer;
