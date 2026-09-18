use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputType {
    Window,
    Ndi { stream_name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputContentRoute {
    LiveSlide,
    Stage,
    Mirror { source_id: String },
    Blank,
}

impl std::fmt::Display for OutputContentRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputContentRoute::LiveSlide => write!(f, "Live Slide"),
            OutputContentRoute::Stage => write!(f, "Stage Monitor"),
            OutputContentRoute::Mirror { source_id } => write!(f, "Mirror: {source_id}"),
            OutputContentRoute::Blank => write!(f, "Blank"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedOutput {
    pub id: String,
    pub label: String,
    pub output_type: OutputType,
    pub content: OutputContentRoute,
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

impl NamedOutput {
    pub fn new_window(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            output_type: OutputType::Window,
            content: OutputContentRoute::LiveSlide,
            width: 1920,
            height: 1080,
            active: false,
        }
    }

    pub fn new_ndi(
        id: impl Into<String>,
        label: impl Into<String>,
        stream_name: impl Into<String>,
    ) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            label: label.into(),
            output_type: OutputType::Ndi {
                stream_name: stream_name.into(),
            },
            content: OutputContentRoute::LiveSlide,
            width: 1920,
            height: 1080,
            active: false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OutputManager {
    outputs: Vec<NamedOutput>,
    index: HashMap<String, usize>,
}

impl OutputManager {
    pub fn with_defaults() -> Self {
        let mut m = Self::default();
        let mut main = NamedOutput::new_window("main", "Main Output");
        main.active = true;
        m.add(main);
        let stream = NamedOutput::new_ndi("stream", "Broadcast Stream", "OpenPresenter");
        m.add(stream);
        let mut stage = NamedOutput::new_window("stage", "Stage Monitor");
        stage.content = OutputContentRoute::Stage;
        m.add(stage);
        m
    }

    pub fn get_look_config_or_default(
        &self,
        look: Option<&crate::domain::Look>,
        screen_id: &str,
    ) -> crate::domain::ScreenLookTarget {
        if let Some(l) = look {
            l.target_or_default(screen_id)
        } else {
            let mut def = crate::domain::ScreenLookTarget::default_for_screen(screen_id);
            if screen_id == "stream" {
                def.media_enabled = false;
            }
            def
        }
    }

    pub fn add(&mut self, output: NamedOutput) {
        if self.index.contains_key(&output.id) {
            return;
        }
        let idx = self.outputs.len();
        self.index.insert(output.id.clone(), idx);
        self.outputs.push(output);
    }

    pub fn remove(&mut self, id: &str) {
        if let Some(&idx) = self.index.get(id) {
            self.outputs.remove(idx);
            self.index.clear();
            for (i, o) in self.outputs.iter().enumerate() {
                self.index.insert(o.id.clone(), i);
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&NamedOutput> {
        self.index.get(id).map(|&i| &self.outputs[i])
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut NamedOutput> {
        if let Some(&i) = self.index.get(id) {
            Some(&mut self.outputs[i])
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &NamedOutput> {
        self.outputs.iter()
    }

    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    pub fn set_active(&mut self, id: &str, active: bool) {
        if let Some(o) = self.get_mut(id) {
            o.active = active;
        }
    }

    pub fn set_content(&mut self, id: &str, content: OutputContentRoute) {
        if let Some(o) = self.get_mut(id) {
            o.content = content;
        }
    }

    pub fn set_resolution(&mut self, id: &str, width: u32, height: u32) {
        if let Some(o) = self.get_mut(id) {
            o.width = width;
            o.height = height;
        }
    }

    pub fn set_label(&mut self, id: &str, label: impl Into<String>) {
        if let Some(o) = self.get_mut(id) {
            o.label = label.into();
        }
    }
}

/// Represents a physical display/monitor detected on the host system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedDisplay {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

impl DetectedDisplay {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        is_primary: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            x,
            y,
            width,
            height,
            is_primary,
        }
    }
}

/// Detect attached physical monitors/displays.
///
/// On macOS, queries `system_profiler SPDisplaysDataType -json`.
/// On other platforms or on failure, falls back to primary and secondary presets.
pub fn detect_displays() -> Vec<DetectedDisplay> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .arg("-json")
            .output()
            && output.status.success()
            && let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
        {
            let mut displays = Vec::new();
            let mut current_offset_x = 0;

            if let Some(gpus) = json.get("SPDisplaysDataType").and_then(|v| v.as_array()) {
                for gpu in gpus {
                    if let Some(ndrvs) = gpu.get("spdisplays_ndrvs").and_then(|v| v.as_array()) {
                        for disp in ndrvs {
                            let name = disp
                                .get("_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Display")
                                .to_string();
                            let id = disp
                                .get("_spdisplays_displayID")
                                .and_then(|v| v.as_str())
                                .unwrap_or("1")
                                .to_string();
                            let is_primary = disp
                                .get("spdisplays_main")
                                .and_then(|v| v.as_str())
                                .map(|v| v == "spdisplays_yes")
                                .unwrap_or(false);

                            let (width, height) = parse_resolution_from_display(disp);
                            let x = if is_primary { 0 } else { current_offset_x };
                            let y = 0;
                            if is_primary {
                                current_offset_x += width as i32;
                            }

                            displays.push(DetectedDisplay {
                                id,
                                name,
                                x,
                                y,
                                width,
                                height,
                                is_primary,
                            });
                        }
                    }
                }
            }

            if !displays.is_empty() {
                return displays;
            }
        }
    }

    fallback_displays()
}

fn parse_resolution_from_display(disp: &serde_json::Value) -> (u32, u32) {
    if let Some(res_str) = disp.get("_spdisplays_resolution").and_then(|v| v.as_str())
        && let Some((w, h)) = parse_w_x_h(res_str)
    {
        return (w, h);
    }
    if let Some(pix_str) = disp.get("_spdisplays_pixels").and_then(|v| v.as_str())
        && let Some((w, h)) = parse_w_x_h(pix_str)
    {
        return (w, h);
    }
    (1920, 1080)
}

fn parse_w_x_h(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('@').next()?.split('x').collect();
    if parts.len() >= 2 {
        let w = parts[0].trim().parse::<u32>().ok()?;
        let h = parts[1].trim().parse::<u32>().ok()?;
        return Some((w, h));
    }
    None
}

/// Fallback display presets when system detection is unavailable or in virtualized environments.
pub fn fallback_displays() -> Vec<DetectedDisplay> {
    vec![
        DetectedDisplay {
            id: "display-1".into(),
            name: "Primary Display".into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: true,
        },
        DetectedDisplay {
            id: "display-2".into(),
            name: "Secondary Projector / TV".into(),
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
            is_primary: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_manager_defaults() {
        let m = OutputManager::with_defaults();
        assert_eq!(m.len(), 3);
        assert!(!m.is_empty());

        let main = m.get("main").expect("main output exists");
        assert!(main.active);
        assert_eq!(main.content, OutputContentRoute::LiveSlide);

        let stream = m.get("stream").expect("stream output exists");
        assert_eq!(
            stream.output_type,
            OutputType::Ndi {
                stream_name: "OpenPresenter".into()
            }
        );

        let stage = m.get("stage").expect("stage output exists");
        assert_eq!(stage.content, OutputContentRoute::Stage);
    }

    #[test]
    fn output_manager_add_and_remove() {
        let mut m = OutputManager::default();
        assert!(m.is_empty());

        let out = NamedOutput::new_window("aux", "Aux Screen");
        m.add(out);
        assert_eq!(m.len(), 1);

        // Duplicate ID should not be added
        let dup = NamedOutput::new_window("aux", "Aux Screen Duplicate");
        m.add(dup);
        assert_eq!(m.len(), 1);

        m.set_active("aux", true);
        assert!(m.get("aux").unwrap().active);

        m.set_resolution("aux", 1280, 720);
        assert_eq!(m.get("aux").unwrap().width, 1280);
        assert_eq!(m.get("aux").unwrap().height, 720);

        m.set_label("aux", "Lobby Display");
        assert_eq!(m.get("aux").unwrap().label, "Lobby Display");

        m.set_content("aux", OutputContentRoute::Blank);
        assert_eq!(m.get("aux").unwrap().content, OutputContentRoute::Blank);

        m.remove("aux");
        assert!(m.is_empty());
        assert!(m.get("aux").is_none());
    }

    #[test]
    fn output_manager_look_config() {
        let m = OutputManager::with_defaults();
        let def_main = m.get_look_config_or_default(None, "main");
        assert!(def_main.slide_enabled);
        assert!(def_main.media_enabled);

        let def_stream = m.get_look_config_or_default(None, "stream");
        assert!(def_stream.slide_enabled);
        assert!(!def_stream.media_enabled); // stream defaults to no background media for key/fill
    }

    #[test]
    fn output_content_route_display() {
        assert_eq!(OutputContentRoute::LiveSlide.to_string(), "Live Slide");
        assert_eq!(OutputContentRoute::Stage.to_string(), "Stage Monitor");
        assert_eq!(OutputContentRoute::Blank.to_string(), "Blank");
        assert_eq!(
            OutputContentRoute::Mirror {
                source_id: "main".into()
            }
            .to_string(),
            "Mirror: main"
        );
    }

    #[test]
    fn test_display_detection_and_fallback() {
        let displays = detect_displays();
        assert!(
            !displays.is_empty(),
            "Display detection should return at least one display"
        );
        let primary = displays.iter().find(|d| d.is_primary);
        assert!(
            primary.is_some(),
            "At least one primary display should be present"
        );

        let fallbacks = fallback_displays();
        assert_eq!(fallbacks.len(), 2);
        assert!(fallbacks[0].is_primary);
        assert_eq!(fallbacks[0].width, 1920);
        assert_eq!(fallbacks[0].height, 1080);
        assert_eq!(fallbacks[1].x, 1920);
    }

    #[test]
    fn test_parse_w_x_h() {
        assert_eq!(parse_w_x_h("1920 x 1080"), Some((1920, 1080)));
        assert_eq!(parse_w_x_h("3840 x 2160 @ 60.00Hz"), Some((3840, 2160)));
        assert_eq!(parse_w_x_h("invalid"), None);
    }
}
