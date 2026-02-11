use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub db_path: PathBuf,
    pub ndi: NdiConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NdiConfig {
    pub source_name: String,
    pub enabled: bool,
    pub frame_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub display_index: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            ndi: NdiConfig {
                source_name: "OpenPresenter".to_string(),
                enabled: true,
                frame_rate: 60,
            },
            output: OutputConfig {
                width: 1920,
                height: 1080,
                display_index: None,
            },
        }
    }
}

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openpresenter")
        .join("library.db")
}
