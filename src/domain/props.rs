use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropContent {
    Text {
        text: String,
        font_size: f32,
        color: [f32; 4],
        bold: bool,
        italic: bool,
    },
    Image {
        path: String,
    },
    Rectangle {
        color: [f32; 4],
        corner_radius: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prop {
    pub id: String,
    pub name: String,
    pub content: PropContent,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub visible: bool,
}

impl Prop {
    pub fn new_lower_third(title: impl Into<String>, subtitle: impl Into<String>) -> Vec<Prop> {
        let title_str = title.into();
        let sub_str = subtitle.into();
        vec![
            Prop {
                id: Uuid::new_v4().to_string(),
                name: "LT Background".to_string(),
                content: PropContent::Rectangle {
                    color: [0.0, 0.0, 0.0, 0.7],
                    corner_radius: 4.0,
                },
                x: 0.05,
                y: 0.78,
                width: 0.9,
                height: 0.18,
                visible: true,
            },
            Prop {
                id: Uuid::new_v4().to_string(),
                name: format!("LT Title: {title_str}"),
                content: PropContent::Text {
                    text: title_str,
                    font_size: 36.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                    bold: true,
                    italic: false,
                },
                x: 0.08,
                y: 0.80,
                width: 0.84,
                height: 0.08,
                visible: true,
            },
            Prop {
                id: Uuid::new_v4().to_string(),
                name: format!("LT Subtitle: {sub_str}"),
                content: PropContent::Text {
                    text: sub_str,
                    font_size: 24.0,
                    color: [0.85, 0.85, 0.85, 1.0],
                    bold: false,
                    italic: true,
                },
                x: 0.08,
                y: 0.88,
                width: 0.84,
                height: 0.07,
                visible: true,
            },
        ]
    }

    pub fn new_logo(image_path: impl Into<String>) -> Prop {
        Prop {
            id: Uuid::new_v4().to_string(),
            name: "Logo".to_string(),
            content: PropContent::Image {
                path: image_path.into(),
            },
            x: 0.80,
            y: 0.03,
            width: 0.15,
            height: 0.10,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Mask {
    #[default]
    None,
    Letterbox {
        bar_fraction: f32,
    },
    Oval {
        feather: f32,
    },
    FrameBorder {
        thickness: f32,
        color: [f32; 4],
    },
    Custom {
        path: String,
    },
}

impl std::fmt::Display for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mask::None => write!(f, "None"),
            Mask::Letterbox { .. } => write!(f, "Letterbox"),
            Mask::Oval { .. } => write!(f, "Oval"),
            Mask::FrameBorder { .. } => write!(f, "Frame Border"),
            Mask::Custom { path } => write!(f, "Custom: {path}"),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenLookTarget {
    pub screen_id: String,
    #[serde(default = "default_true")]
    pub media_enabled: bool,
    #[serde(default = "default_true")]
    pub slide_enabled: bool,
    #[serde(default = "default_true")]
    pub props_enabled: bool,
    #[serde(default = "default_true")]
    pub messages_enabled: bool,
    #[serde(default)]
    pub mask_enabled: bool,
    #[serde(default)]
    pub theme_id: Option<String>,
}

impl ScreenLookTarget {
    pub fn default_for_screen(screen_id: impl Into<String>) -> Self {
        Self {
            screen_id: screen_id.into(),
            media_enabled: true,
            slide_enabled: true,
            props_enabled: true,
            messages_enabled: true,
            mask_enabled: false,
            theme_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveAlertMessage {
    pub id: String,
    pub text: String,
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl LiveAlertMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text: text.into(),
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Look {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub mask: Mask,
    #[serde(default)]
    pub active_prop_ids: Vec<String>,
    #[serde(default)]
    pub theme_id: Option<String>,
    #[serde(default)]
    pub screens: Vec<ScreenLookTarget>,
}

impl Look {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            mask: Mask::None,
            active_prop_ids: Vec::new(),
            theme_id: None,
            screens: Vec::new(),
        }
    }

    pub fn target_for_screen(&self, screen_id: &str) -> Option<&ScreenLookTarget> {
        self.screens.iter().find(|s| s.screen_id == screen_id)
    }

    pub fn target_for_screen_mut(&mut self, screen_id: &str) -> Option<&mut ScreenLookTarget> {
        self.screens.iter_mut().find(|s| s.screen_id == screen_id)
    }

    pub fn target_or_default(&self, screen_id: &str) -> ScreenLookTarget {
        self.target_for_screen(screen_id)
            .cloned()
            .unwrap_or_else(|| ScreenLookTarget::default_for_screen(screen_id))
    }

    pub fn set_screen_target(&mut self, target: ScreenLookTarget) {
        if let Some(pos) = self
            .screens
            .iter()
            .position(|s| s.screen_id == target.screen_id)
        {
            self.screens[pos] = target;
        } else {
            self.screens.push(target);
        }
    }

    pub fn with_default_screens(name: impl Into<String>) -> Self {
        let mut look = Self::new(name);
        look.screens = vec![
            ScreenLookTarget::default_for_screen("main"),
            ScreenLookTarget {
                screen_id: "stream".to_string(),
                media_enabled: false,
                slide_enabled: true,
                props_enabled: true,
                messages_enabled: true,
                mask_enabled: false,
                theme_id: None,
            },
            ScreenLookTarget {
                screen_id: "stage".to_string(),
                media_enabled: false,
                slide_enabled: true,
                props_enabled: false,
                messages_enabled: true,
                mask_enabled: false,
                theme_id: None,
            },
        ];
        look
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropManager {
    pub props: Vec<Prop>,
    pub looks: Vec<Look>,
    #[serde(default)]
    pub active_mask: Mask,
    #[serde(default)]
    pub active_look_id: Option<String>,
    #[serde(default)]
    pub live_message: Option<LiveAlertMessage>,
}

impl Default for PropManager {
    fn default() -> Self {
        Self {
            props: Vec::new(),
            looks: Vec::new(),
            active_mask: Mask::None,
            active_look_id: None,
            live_message: None,
        }
    }
}

impl PropManager {
    pub fn with_default_looks() -> Self {
        let default_look = Look::with_default_screens("Default");
        let active_id = default_look.id.clone();
        Self {
            props: Vec::new(),
            looks: vec![
                default_look,
                Look::with_default_screens("Broadcast / Lower Third"),
            ],
            active_mask: Mask::None,
            active_look_id: Some(active_id),
            live_message: None,
        }
    }
    pub fn toggle_prop(&mut self, id: &str) {
        if let Some(prop) = self.props.iter_mut().find(|p| p.id == id) {
            prop.visible = !prop.visible;
        }
    }

    pub fn remove_prop(&mut self, id: &str) {
        self.props.retain(|p| p.id != id);
    }

    pub fn add_prop(&mut self, prop: Prop) {
        self.props.push(prop);
    }

    pub fn visible_props(&self) -> impl Iterator<Item = &Prop> {
        self.props.iter().filter(|p| p.visible)
    }

    pub fn save_look(&mut self, name: impl Into<String>) {
        let active_ids: Vec<String> = self
            .props
            .iter()
            .filter(|p| p.visible)
            .map(|p| p.id.clone())
            .collect();
        let mut look = Look::new(name);
        look.mask = self.active_mask.clone();
        look.active_prop_ids = active_ids;
        self.looks.push(look);
    }

    pub fn add_look(&mut self, look: Look) {
        self.looks.push(look);
    }

    pub fn apply_look(&mut self, look_id: &str) {
        if let Some(look) = self.looks.iter().find(|l| l.id == look_id).cloned() {
            self.active_mask = look.mask.clone();
            self.active_look_id = Some(look.id.clone());
            for prop in &mut self.props {
                prop.visible = look.active_prop_ids.contains(&prop.id);
            }
        }
    }

    pub fn current_look(&self) -> Option<&Look> {
        self.active_look_id
            .as_deref()
            .and_then(|id| self.looks.iter().find(|l| l.id == id))
            .or_else(|| self.looks.first())
    }

    pub fn set_message(&mut self, text: impl Into<String>) {
        let t = text.into();
        if t.trim().is_empty() {
            self.live_message = None;
        } else {
            self.live_message = Some(LiveAlertMessage::new(t));
        }
    }

    pub fn clear_message(&mut self) {
        self.live_message = None;
    }

    pub fn remove_look(&mut self, id: &str) {
        self.looks.retain(|l| l.id != id);
        if self.active_look_id.as_deref() == Some(id) {
            self.active_look_id = None;
        }
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_default_is_none() {
        assert_eq!(Mask::default(), Mask::None);
    }

    #[test]
    fn mask_display_none() {
        assert_eq!(Mask::None.to_string(), "None");
    }

    #[test]
    fn mask_display_letterbox() {
        assert_eq!(
            Mask::Letterbox { bar_fraction: 0.1 }.to_string(),
            "Letterbox"
        );
    }

    #[test]
    fn mask_display_oval() {
        assert_eq!(Mask::Oval { feather: 0.5 }.to_string(), "Oval");
    }

    #[test]
    fn mask_display_frame_border() {
        assert_eq!(
            Mask::FrameBorder {
                thickness: 2.0,
                color: [1.0, 0.0, 0.0, 1.0]
            }
            .to_string(),
            "Frame Border"
        );
    }

    #[test]
    fn mask_display_custom() {
        assert_eq!(
            Mask::Custom {
                path: "/foo/bar.png".to_string()
            }
            .to_string(),
            "Custom: /foo/bar.png"
        );
    }

    #[test]
    fn look_new_is_empty() {
        let look = Look::new("Stage");
        assert_eq!(look.name, "Stage");
        assert_eq!(look.mask, Mask::None);
        assert!(look.active_prop_ids.is_empty());
        assert!(look.theme_id.is_none());
    }

    fn sample_prop(id: &str, visible: bool) -> Prop {
        Prop {
            id: id.to_string(),
            name: id.to_string(),
            content: PropContent::Rectangle {
                color: [0.0; 4],
                corner_radius: 0.0,
            },
            x: 0.0,
            y: 0.0,
            width: 0.1,
            height: 0.1,
            visible,
        }
    }

    #[test]
    fn prop_manager_add_and_remove() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("a", true));
        pm.add_prop(sample_prop("b", false));
        assert_eq!(pm.props.len(), 2);
        pm.remove_prop("a");
        assert_eq!(pm.props.len(), 1);
        assert_eq!(pm.props[0].id, "b");
    }

    #[test]
    fn prop_manager_toggle_changes_visibility() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("x", true));
        pm.toggle_prop("x");
        assert!(!pm.props[0].visible);
        pm.toggle_prop("x");
        assert!(pm.props[0].visible);
    }

    #[test]
    fn prop_manager_visible_props_filters_invisible() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("show", true));
        pm.add_prop(sample_prop("hide", false));
        let visible: Vec<_> = pm.visible_props().collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, "show");
    }

    #[test]
    fn prop_manager_save_look_captures_visible_ids() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("banner", true));
        pm.add_prop(sample_prop("logo", false));
        pm.save_look("With Banner");
        assert_eq!(pm.looks.len(), 1);
        assert_eq!(pm.looks[0].active_prop_ids, vec!["banner"]);
    }

    #[test]
    fn prop_manager_apply_look_restores_visibility() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("banner", true));
        pm.add_prop(sample_prop("logo", false));
        pm.save_look("With Banner");
        let look_id = pm.looks[0].id.clone();

        pm.toggle_prop("banner");
        pm.toggle_prop("logo");
        pm.apply_look(&look_id);

        assert!(pm.props.iter().find(|p| p.id == "banner").unwrap().visible);
        assert!(!pm.props.iter().find(|p| p.id == "logo").unwrap().visible);
    }

    #[test]
    fn prop_manager_remove_look() {
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("p", true));
        pm.save_look("Test");
        let id = pm.looks[0].id.clone();
        pm.remove_look(&id);
        assert!(pm.looks.is_empty());
    }

    #[test]
    fn new_lower_third_creates_three_props() {
        let props = Prop::new_lower_third("Speaker", "Pastor John");
        assert_eq!(props.len(), 3);
    }

    #[test]
    fn new_lower_third_title_content_is_correct() {
        let props = Prop::new_lower_third("Alice", "Elder");
        if let PropContent::Text {
            text,
            font_size,
            bold,
            ..
        } = &props[1].content
        {
            assert_eq!(text, "Alice");
            assert!((font_size - 36.0).abs() < 0.01);
            assert!(bold);
        } else {
            panic!("expected Text content for title");
        }
    }

    #[test]
    fn new_lower_third_subtitle_is_italic() {
        let props = Prop::new_lower_third("Alice", "Elder");
        if let PropContent::Text { italic, text, .. } = &props[2].content {
            assert!(italic);
            assert_eq!(text, "Elder");
        } else {
            panic!("expected Text content for subtitle");
        }
    }

    #[test]
    fn new_lower_third_background_is_rectangle() {
        let props = Prop::new_lower_third("A", "B");
        assert!(matches!(props[0].content, PropContent::Rectangle { .. }));
    }

    #[test]
    fn new_logo_sets_path_and_name() {
        let logo = Prop::new_logo("/assets/logo.png");
        assert_eq!(logo.name, "Logo");
        assert!(logo.visible);
        if let PropContent::Image { path } = &logo.content {
            assert_eq!(path, "/assets/logo.png");
        } else {
            panic!("expected Image content");
        }
    }

    #[test]
    fn prop_manager_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("props.json");
        let mut pm = PropManager::default();
        pm.add_prop(sample_prop("saved", true));
        pm.save_to_file(&path).unwrap();

        let loaded = PropManager::load_from_file(&path);
        assert_eq!(loaded.props.len(), 1);
        assert_eq!(loaded.props[0].id, "saved");
    }

    #[test]
    fn prop_manager_load_from_nonexistent_returns_default() {
        let loaded =
            PropManager::load_from_file(std::path::Path::new("/nonexistent/nowhere/props.json"));
        assert!(loaded.props.is_empty());
    }

    #[test]
    fn screen_look_target_default_and_overrides() {
        let mut target = ScreenLookTarget::default_for_screen("stream");
        assert_eq!(target.screen_id, "stream");
        assert!(target.media_enabled);
        assert!(target.slide_enabled);

        target.media_enabled = false;
        target.theme_id = Some("lower-third-id".to_string());

        let mut look = Look::new("Broadcast");
        look.set_screen_target(target);

        let retrieved = look.target_for_screen("stream").unwrap();
        assert!(!retrieved.media_enabled);
        assert_eq!(retrieved.theme_id.as_deref(), Some("lower-third-id"));

        let unconfigured = look.target_or_default("stage");
        assert_eq!(unconfigured.screen_id, "stage");
        assert!(unconfigured.media_enabled);
    }

    #[test]
    fn live_alert_message_toggle_and_clear() {
        let mut pm = PropManager::default();
        assert!(pm.live_message.is_none());

        pm.set_message("Parent #402");
        assert!(pm.live_message.is_some());
        assert_eq!(pm.live_message.as_ref().unwrap().text, "Parent #402");

        pm.clear_message();
        assert!(pm.live_message.is_none());
    }
}
