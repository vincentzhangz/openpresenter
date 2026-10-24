use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::output::NamedOutput;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Where the library lives.
    ///
    /// The path is optional too: a configuration file that only sets the
    /// remote control up is completed with the defaults rather than thrown
    /// away.
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,
    /// The broadcast settings.
    ///
    /// The section is optional: a configuration file that only sets the
    /// remote control up is completed with the defaults rather than thrown
    /// away — a file the operator wrote by hand must not cost them their
    /// whole show environment.
    #[serde(default)]
    pub ndi: NdiConfig,
    /// The output settings.
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub general: GeneralConfig,
    /// The sound the app plays.
    #[serde(default)]
    pub audio: AudioConfig,
    /// Where the app looks for updates.
    #[serde(default)]
    pub updates: UpdateConfig,
    /// The labels a slide carries.
    #[serde(default)]
    pub slide_labels: SlideLabelsConfig,
    /// The recording presets.
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub triggers: TriggerConfig,
    #[serde(default)]
    pub transitions: TransitionConfig,
    #[serde(default)]
    pub ui: UiConfig,
    /// The outputs the operator set up: their destinations, their routes and
    /// their resolutions.
    ///
    /// The outputs are the operator's environment, so they are saved with
    /// the rest of it. An empty list means the built-in set.
    #[serde(default)]
    pub outputs: Vec<NamedOutput>,
    /// The file these settings live in.
    ///
    /// The path is not part of the file: it says where the settings are, not
    /// what they are. It is remembered so a save writes back to the file the
    /// settings came from — a test that points the settings at a scratch file
    /// must not write over the operator's own.
    #[serde(skip)]
    pub config_path: PathBuf,
}

/// App-general behaviour settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralConfig {
    /// Workspace opened at launch: `"show"` (live) or `"edit"`.
    #[serde(default = "default_startup_mode")]
    pub startup_mode: String,
}

fn default_startup_mode() -> String {
    "show".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            startup_mode: default_startup_mode(),
        }
    }
}

/// The sound the app plays.
///
/// The master volume is the level the room hears: a clip is played at its
/// own volume, and the master volume is on top of it, so the operator can
/// turn the whole service down without touching every clip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioConfig {
    /// The master volume, from 0 to 100.
    #[serde(default = "default_master_volume")]
    pub master_volume: u8,
}

fn default_master_volume() -> u8 {
    100
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            master_volume: default_master_volume(),
        }
    }
}

/// Where the app looks for updates.
///
/// The channel says which builds the operator wants: the stable ones, or the
/// ones that are still being made.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateConfig {
    /// The update channel: `"stable"` or `"beta"`.
    #[serde(default = "default_update_channel")]
    pub channel: String,
}

fn default_update_channel() -> String {
    "stable".to_string()
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: default_update_channel(),
        }
    }
}

/// The labels a slide carries.
///
/// The default label is the one a new slide gets: a slide that is a repeat
/// of the chorus before it is marked as one.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SlideLabelsConfig {
    /// The label a new slide is given.
    #[serde(default)]
    pub default_label: String,
}

/// A named set of recording settings.
///
/// A service is recorded the same way every week, so the settings are
/// chosen once and kept: the folder the take goes into, the name it is
/// given, and the frame rate it is made at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapturePreset {
    /// The preset's id.
    pub id: String,
    /// The name the operator gave the preset.
    pub name: String,
    /// The folder the recording goes into.
    ///
    /// An empty folder means the system's video folder.
    #[serde(default)]
    pub destination: String,
    /// The name of the file, without the extension.
    ///
    /// The placeholders are filled in when the take starts: `{date}` and
    /// `{time}` are the moment, and `{name}` is the preset's own name.
    #[serde(default)]
    pub file_name: String,
    /// The frame rate the recording is made at.
    #[serde(default = "default_capture_fps")]
    pub fps: u32,
    /// Whether the operator is asked before the recording stops.
    #[serde(default)]
    pub confirm_before_stopping: bool,
    /// The codec the take is made in.
    #[serde(default)]
    pub codec: CaptureCodec,
}

/// The codec a recording is made in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureCodec {
    /// H.264 — the codec everything plays.
    #[default]
    H264,
    /// H.265 — a smaller file for the same picture, and the one the
    /// streaming services take.
    H265,
}

impl CaptureCodec {
    /// The name the codec is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265",
        }
    }

    /// Every codec the operator can pick.
    pub const ALL: [CaptureCodec; 2] = [CaptureCodec::H264, CaptureCodec::H265];
}

fn default_capture_fps() -> u32 {
    30
}

impl Default for CapturePreset {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::from("Service"),
            destination: String::new(),
            file_name: String::from("{date}-{time}"),
            fps: default_capture_fps(),
            confirm_before_stopping: false,
            codec: CaptureCodec::default(),
        }
    }
}

impl CapturePreset {
    /// The file name the take is given.
    ///
    /// The placeholders are filled in with the moment the take started, so
    /// every week of a service has its own file and none of them overwrites
    /// another.
    pub fn resolve_file_name(&self, at_epoch: u64) -> String {
        let moment = chrono::DateTime::from_timestamp(at_epoch as i64, 0)
            .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
        let name = self
            .file_name
            .replace("{date}", &moment.format("%Y-%m-%d").to_string())
            .replace("{time}", &moment.format("%H-%M-%S").to_string())
            .replace("{name}", &self.name);
        if name.trim().is_empty() {
            return format!(
                "{}-{}",
                moment.format("%Y-%m-%d"),
                moment.format("%H-%M-%S")
            );
        }
        name
    }
}

/// The recording presets the operator has.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// The presets, in the order they are shown.
    #[serde(default)]
    pub presets: Vec<CapturePreset>,
    /// The preset the recordings are made with.
    #[serde(default)]
    pub active: Option<String>,
}

impl CaptureConfig {
    /// The preset the recordings are made with.
    pub fn active_preset(&self) -> Option<&CapturePreset> {
        let id = self.active.as_ref()?;
        self.presets.iter().find(|preset| &preset.id == id)
    }

    /// The preset the recordings are made with, for editing.
    pub fn active_preset_mut(&mut self) -> Option<&mut CapturePreset> {
        let id = self.active.as_ref()?;
        self.presets.iter_mut().find(|preset| &preset.id == id)
    }
}

/// Remote-control listener settings.
///
/// The HTTP and OSC listeners accept unauthenticated control commands on all
/// interfaces, so they are **off by default**; enabling
/// `http_enabled`/`osc_enabled` (settings UI or config file) starts them at
/// launch on trusted networks only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TriggerConfig {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_osc_port")]
    pub osc_port: u16,
    #[serde(default)]
    pub http_enabled: bool,
    #[serde(default)]
    pub osc_enabled: bool,
    /// The operator's macros.
    ///
    /// A macro is automation the operator built, so it is saved with the
    /// rest of the configuration: a macro that is gone the next morning
    /// cannot be used.
    #[serde(default)]
    pub macros: Vec<crate::triggers::automation::Macro>,
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
            http_enabled: false,
            osc_enabled: false,
            macros: Vec::new(),
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
    #[serde(default = "default_ndi_source_name")]
    pub source_name: String,
    /// Whether the broadcast is allowed to run.
    ///
    /// The setting is honoured when the operator starts the broadcast: a
    /// machine that is not allowed to send video says so instead of
    /// silently doing nothing.
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ndi_frame_rate")]
    pub frame_rate: u32,
}

fn default_ndi_source_name() -> String {
    "OpenPresenter".to_string()
}

fn default_ndi_frame_rate() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

impl Default for NdiConfig {
    fn default() -> Self {
        Self {
            source_name: default_ndi_source_name(),
            enabled: true,
            frame_rate: default_ndi_frame_rate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_width")]
    pub width: u32,
    #[serde(default = "default_output_height")]
    pub height: u32,
    #[serde(default)]
    pub display_index: Option<usize>,
    #[serde(default)]
    pub screen_x: f32,
    #[serde(default)]
    pub screen_y: f32,
    #[serde(default)]
    pub auto_fullscreen: bool,
}

fn default_output_width() -> u32 {
    1920
}

fn default_output_height() -> u32 {
    1080
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            width: default_output_width(),
            height: default_output_height(),
            display_index: None,
            screen_x: 0.0,
            screen_y: 0.0,
            auto_fullscreen: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            updates: UpdateConfig::default(),
            slide_labels: SlideLabelsConfig::default(),
            capture: CaptureConfig::default(),
            db_path: default_db_path(),
            ndi: NdiConfig::default(),
            output: OutputConfig::default(),
            general: GeneralConfig::default(),
            triggers: TriggerConfig::default(),
            transitions: TransitionConfig::default(),
            ui: UiConfig::default(),
            outputs: Vec::new(),
            config_path: Self::config_path(),
        }
    }
}

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            log::warn!("could not determine data directory; storing DB in current directory");
            PathBuf::from(".")
        })
        .join("openpresenter")
        .join("library.db")
}

impl Config {
    /// Load the persisted configuration.
    ///
    /// A missing config file is normal on first run and yields defaults. A
    /// *corrupt* or unreadable one is a different situation: the user has real
    /// settings (output routing, resolution, triggers) that we must not silently
    /// destroy by overwriting them with defaults. In that case the old file is
    /// preserved as `config.toml.bak` and defaults are returned.
    pub fn load() -> Self {
        Self::load_from(Self::config_path())
    }

    /// Load the configuration from a specific file.
    ///
    /// The file the settings came from is remembered, so a later save writes
    /// back to it rather than to the default location.
    pub fn load_from(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut config = match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(cfg) => cfg,
                Err(e) => {
                    Self::backup_corrupt_config(&path);
                    log::warn!(
                        "config parse error ({path:?}): {e} — using defaults (original kept as .bak)"
                    );
                    // The defaults point at a different database, which would
                    // make the operator's library look empty. The backup still
                    // knows where the library lives.
                    let mut config = Self::default();
                    if let Some(db_path) = Self::recover_db_path(&path) {
                        log::warn!("recovered the database location from the backup: {db_path:?}");
                        config.db_path = db_path;
                    }
                    config
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                log::warn!("config read error ({path:?}): {e} — using defaults");
                Self::default()
            }
        };
        config.config_path = path;
        config
    }

    /// The database location recorded in the backed-up config.
    fn recover_db_path(path: &std::path::Path) -> Option<PathBuf> {
        let bak = path.with_extension("toml.bak");
        let contents = std::fs::read_to_string(&bak).ok()?;
        let config: Self = toml::from_str(&contents).ok()?;
        Some(config.db_path)
    }

    /// Move an unparseable config aside so the next save cannot destroy it.
    fn backup_corrupt_config(path: &std::path::Path) {
        let bak = path.with_extension("toml.bak");
        if let Err(e) = std::fs::rename(path, &bak) {
            log::warn!("could not back up corrupt config to {bak:?}: {e}");
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = &self.config_path;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        // Written atomically: an interrupted write must not leave a truncated
        // config behind.
        crate::atomic::write_atomic(path, contents.as_bytes())?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                log::warn!("could not determine config directory; using current directory");
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

    /// A configuration file the operator wrote by hand is completed, not
    /// thrown away.
    ///
    /// The file is documented as editable, so a partial one is the normal
    /// case: it must not cost the operator their whole show environment.
    #[test]
    fn a_partial_config_is_completed_rather_than_rejected() {
        let partial = r#"
            [triggers]
            http_enabled = true
            http_port = 9091
        "#;
        let config: Config = toml::from_str(partial).expect("a partial file is completed");
        assert!(config.triggers.http_enabled);
        assert_eq!(config.triggers.http_port, 9091);
        assert_eq!(
            config.output.width,
            OutputConfig::default().width,
            "the output settings keep their defaults"
        );
        assert_eq!(
            config.ndi.source_name,
            NdiConfig::default().source_name,
            "the broadcast settings keep their defaults"
        );
        assert!(
            !config.db_path.as_os_str().is_empty(),
            "the library keeps its place"
        );
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
        // Remote-control listeners must stay off unless explicitly enabled.
        assert!(!restored.triggers.http_enabled);
        assert!(!restored.triggers.osc_enabled);
        assert_eq!(restored.transitions.default_duration_ms, 500);
        assert_eq!(restored.ui.default_grid_cols, 4);
    }

    /// The placeholders in a file name are filled in with the moment.
    ///
    /// A service is recorded every week, so the name has to say which week:
    /// a name that is the same every week is a name that overwrites the
    /// take before it.
    #[test]
    fn a_preset_names_the_take() {
        let preset = CapturePreset {
            name: String::from("Sunday Service"),
            file_name: String::from("{date}-{time}-{name}"),
            ..CapturePreset::default()
        };
        // The first of January 2024, at one second past midnight.
        let name = preset.resolve_file_name(1_704_067_200);
        assert!(name.starts_with("2024-01-01"), "got {name}");
        assert!(name.ends_with("Sunday Service"), "got {name}");
    }

    /// A preset with no name of its own still names the take.
    #[test]
    fn a_preset_with_no_file_name_still_names_the_take() {
        let preset = CapturePreset {
            file_name: String::new(),
            ..CapturePreset::default()
        };
        let name = preset.resolve_file_name(1_704_067_200);
        assert!(!name.trim().is_empty());
    }

    /// The chosen preset is the one the recordings are made with.
    #[test]
    fn the_chosen_preset_is_the_active_one() {
        let first = CapturePreset {
            name: String::from("First"),
            ..CapturePreset::default()
        };
        let second = CapturePreset {
            name: String::from("Second"),
            ..CapturePreset::default()
        };
        let config = CaptureConfig {
            presets: vec![first.clone(), second.clone()],
            active: Some(second.id.clone()),
        };
        assert_eq!(
            config.active_preset().map(|preset| preset.name.clone()),
            Some(String::from("Second"))
        );
    }

    /// The codec is the operator's choice, and it has a name.
    #[test]
    fn the_codec_has_a_name() {
        assert_eq!(CaptureCodec::H264.label(), "H.264");
        assert_eq!(CaptureCodec::H265.label(), "H.265");
        // H.264 is the default: it is what everything plays.
        assert_eq!(CaptureCodec::default(), CaptureCodec::H264);
    }

    /// Every setting has to survive the round trip to the file.
    ///
    /// The config is the operator's environment: a setting that is not
    /// persisted silently resets the next time the app starts.
    #[test]
    fn every_setting_survives_the_round_trip() {
        let original = Config {
            capture: CaptureConfig::default(),
            audio: AudioConfig::default(),
            updates: UpdateConfig::default(),
            slide_labels: SlideLabelsConfig::default(),
            db_path: std::path::PathBuf::from("/tmp/somewhere/library.db"),
            ndi: NdiConfig {
                source_name: "Church Stream".to_string(),
                enabled: true,
                frame_rate: 50,
            },
            output: OutputConfig {
                width: 2560,
                height: 1440,
                display_index: Some(2),
                screen_x: 120.5,
                screen_y: -40.25,
                auto_fullscreen: true,
            },
            general: GeneralConfig {
                startup_mode: "edit".to_string(),
            },
            triggers: TriggerConfig {
                http_port: 8181,
                osc_port: 8282,
                http_enabled: true,
                osc_enabled: true,
                macros: Vec::new(),
            },
            transitions: TransitionConfig {
                default_type: "wipe_180".to_string(),
                default_duration_ms: 1234,
            },
            ui: UiConfig {
                reduce_motion: true,
                default_grid_cols: 6,
            },
            outputs: Vec::new(),
            config_path: std::path::PathBuf::from("/tmp/config.toml"),
        };

        let toml_str = toml::to_string_pretty(&original).expect("serialize");
        let restored: Config = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(restored.db_path, original.db_path);
        assert_eq!(restored.ndi.source_name, original.ndi.source_name);
        assert_eq!(restored.ndi.enabled, original.ndi.enabled);
        assert_eq!(restored.ndi.frame_rate, original.ndi.frame_rate);
        assert_eq!(restored.output.width, original.output.width);
        assert_eq!(restored.output.height, original.output.height);
        assert_eq!(restored.output.display_index, original.output.display_index);
        assert_eq!(restored.output.screen_x, original.output.screen_x);
        assert_eq!(restored.output.screen_y, original.output.screen_y);
        assert_eq!(
            restored.output.auto_fullscreen,
            original.output.auto_fullscreen
        );
        assert_eq!(restored.general.startup_mode, original.general.startup_mode);
        assert_eq!(restored.triggers.http_port, original.triggers.http_port);
        assert_eq!(restored.triggers.osc_port, original.triggers.osc_port);
        assert_eq!(
            restored.triggers.http_enabled,
            original.triggers.http_enabled
        );
        assert_eq!(restored.triggers.osc_enabled, original.triggers.osc_enabled);
        assert_eq!(
            restored.transitions.default_type,
            original.transitions.default_type
        );
        assert_eq!(
            restored.transitions.default_duration_ms,
            original.transitions.default_duration_ms
        );
        assert_eq!(restored.ui.reduce_motion, original.ui.reduce_motion);
        assert_eq!(restored.ui.default_grid_cols, original.ui.default_grid_cols);
    }

    /// The outputs are the operator's environment, so they have to survive
    /// the round trip to the file: a second projector that vanished on the
    /// next launch would be a broken setup.
    #[test]
    fn the_outputs_survive_the_round_trip() {
        use crate::output::{NamedOutput, OutputContentRoute, OutputType};

        let mut projector = NamedOutput::new_window("projector_b", "Second projector");
        projector.content = OutputContentRoute::Mirror {
            source_id: "audience".to_string(),
        };
        projector.width = 1280;
        projector.height = 720;
        projector.active = true;
        // The second projector sits on the second display, a little to the
        // right of its corner: a placement that is lost puts it on top of
        // the first one.
        projector.placement = crate::output::OutputPlacement {
            display_index: Some(1),
            screen_x: 40.0,
            screen_y: 12.5,
        };
        let mut stream = NamedOutput::new_ndi("stream", "Broadcast", "Church");
        stream.active = true;

        let original = Config {
            capture: CaptureConfig::default(),
            audio: AudioConfig::default(),
            updates: UpdateConfig::default(),
            slide_labels: SlideLabelsConfig::default(),
            db_path: std::path::PathBuf::from("/tmp/library.db"),
            ndi: NdiConfig::default(),
            output: OutputConfig::default(),
            general: GeneralConfig::default(),
            triggers: TriggerConfig::default(),
            transitions: TransitionConfig::default(),
            ui: UiConfig::default(),
            outputs: vec![projector, stream],
            config_path: std::path::PathBuf::from("/tmp/config.toml"),
        };

        let toml_str = toml::to_string_pretty(&original).expect("serialize");
        let restored: Config = toml::from_str(&toml_str).expect("deserialize");

        assert_eq!(restored.outputs.len(), 2);
        let projector = restored
            .outputs
            .iter()
            .find(|output| output.id == "projector_b")
            .expect("the second projector is saved");
        assert_eq!(projector.width, 1280);
        assert_eq!(projector.height, 720);
        assert!(projector.active);
        assert_eq!(
            projector.placement.display_index,
            Some(1),
            "the second projector keeps its display"
        );
        assert_eq!(projector.placement.screen_x, 40.0);
        assert_eq!(projector.placement.screen_y, 12.5);
        assert!(matches!(
            projector.content,
            OutputContentRoute::Mirror { ref source_id } if source_id == "audience"
        ));
        assert!(matches!(
            restored.outputs[1].output_type,
            OutputType::Ndi { ref stream_name } if stream_name == "Church"
        ));
    }

    #[test]
    fn load_ignores_removed_config_fields() {
        // Older config files may contain fields that no longer exist; they
        // must be skipped rather than rejected.
        let legacy = "\
db_path = '/tmp/library.db'
[ndi]
source_name = 'OpenPresenter'
enabled = true
frame_rate = 60
[output]
width = 1920
height = 1080
[general]
startup_mode = 'show'
auto_reconnect = true
";
        let parsed: Config = toml::from_str(legacy).expect("legacy config should parse");
        assert_eq!(parsed.general.startup_mode, "show");
        assert!(!parsed.triggers.http_enabled);
    }
}
