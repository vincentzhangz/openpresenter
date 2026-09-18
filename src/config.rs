use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub db_path: PathBuf,
    pub ndi: NdiConfig,
    pub output: OutputConfig,
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub triggers: TriggerConfig,
    #[serde(default)]
    pub transitions: TransitionConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralConfig {
    #[serde(default = "default_startup_mode")]
    pub startup_mode: String,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
}

fn default_startup_mode() -> String {
    "show".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            startup_mode: default_startup_mode(),
            auto_reconnect: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerConfig {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_osc_port")]
    pub osc_port: u16,
    #[serde(default = "default_true")]
    pub http_enabled: bool,
    #[serde(default = "default_true")]
    pub osc_enabled: bool,
}

fn default_http_port() -> u16 {
    9090
}

fn default_osc_port() -> u16 {
    9000
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            http_port: default_http_port(),
            osc_port: default_osc_port(),
            http_enabled: true,
            osc_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionConfig {
    #[serde(default = "default_transition_type")]
    pub default_type: String,
    #[serde(default = "default_transition_duration")]
    pub default_duration_ms: u64,
}

fn default_transition_type() -> String {
    "fade".to_string()
}

fn default_transition_duration() -> u64 {
    500
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            default_type: default_transition_type(),
            default_duration_ms: default_transition_duration(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default = "default_grid_cols")]
    pub default_grid_cols: usize,
}

fn default_grid_cols() -> usize {
    4
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            reduce_motion: false,
            default_grid_cols: default_grid_cols(),
        }
    }
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
    #[serde(default)]
    pub screen_x: f32,
    #[serde(default)]
    pub screen_y: f32,
    #[serde(default)]
    pub auto_fullscreen: bool,
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
                screen_x: 0.0,
                screen_y: 0.0,
                auto_fullscreen: false,
            },
            general: GeneralConfig::default(),
            triggers: TriggerConfig::default(),
            transitions: TransitionConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            eprintln!(
                "warning: could not determine data directory; storing DB in current directory"
            );
            PathBuf::from(".")
        })
        .join("openpresenter")
        .join("library.db")
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("config parse error ({path:?}): {e} — using defaults");
                Self::default()
            }),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                eprintln!("config read error ({path:?}): {e} — using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                eprintln!("warning: could not determine config directory; using current directory");
                PathBuf::from(".")
            })
            .join("openpresenter")
            .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ndi_source_name() {
        let cfg = Config::default();
        assert_eq!(cfg.ndi.source_name, "OpenPresenter");
    }

    #[test]
    fn default_output_resolution_is_1080p() {
        let cfg = Config::default();
        assert_eq!(cfg.output.width, 1920);
        assert_eq!(cfg.output.height, 1080);
    }

    #[test]
    fn default_db_path_contains_openpresenter() {
        let cfg = Config::default();
        assert!(cfg.db_path.to_string_lossy().contains("openpresenter"));
    }

    #[test]
    fn config_path_ends_with_config_toml() {
        let p = Config::config_path();
        assert!(p.ends_with("config.toml"));
    }

    #[test]
    fn load_returns_default_when_file_absent() {
        let _cfg = Config::load();
    }

    #[test]
    fn roundtrip_toml_serialisation() {
        let original = Config::default();
        let toml_str = toml::to_string_pretty(&original).expect("serialize");
        let restored: Config = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(restored.output.width, original.output.width);
        assert_eq!(restored.output.height, original.output.height);
        assert_eq!(restored.ndi.source_name, original.ndi.source_name);
        assert_eq!(restored.general.startup_mode, "show");
        assert_eq!(restored.triggers.http_port, 9090);
        assert_eq!(restored.transitions.default_duration_ms, 500);
        assert_eq!(restored.ui.default_grid_cols, 4);
    }
}
