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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum Mask {
    #[default]
    None,
    Letterbox { bar_fraction: f32 },
    Oval { feather: f32 },
    FrameBorder { thickness: f32, color: [f32; 4] },
    Custom { path: String },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Look {
    pub id: String,
    pub name: String,
    pub mask: Mask,
    pub active_prop_ids: Vec<String>,
    pub theme_id: Option<String>,
}

impl Look {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            mask: Mask::None,
            active_prop_ids: Vec::new(),
            theme_id: None,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PropManager {
    pub props: Vec<Prop>,
    pub looks: Vec<Look>,
    pub active_mask: Mask,
}

impl PropManager {
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

    pub fn apply_look(&mut self, look_id: &str) {
        if let Some(look) = self.looks.iter().find(|l| l.id == look_id).cloned() {
            self.active_mask = look.mask.clone();
            for prop in &mut self.props {
                prop.visible = look.active_prop_ids.contains(&prop.id);
            }
        }
    }

    pub fn remove_look(&mut self, id: &str) {
        self.looks.retain(|l| l.id != id);
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
        assert!(loaded.looks.is_empty());
    }
}
