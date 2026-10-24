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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Whether the announcement is drawn on this screen.
    ///
    /// An announcement is what the app says to the room on its own, so a
    /// screen that shows the slide can leave it out.
    #[serde(default = "default_true")]
    pub announcements_enabled: bool,
    #[serde(default = "default_true")]
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
            announcements_enabled: true,
            mask_enabled: true,
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
                announcements_enabled: true,
                screen_id: "stream".to_string(),
                media_enabled: false,
                slide_enabled: true,
                props_enabled: true,
                messages_enabled: true,
                mask_enabled: false,
                theme_id: None,
            },
            ScreenLookTarget {
                announcements_enabled: true,
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
    /// The announcement the app is showing.
    ///
    /// An announcement is what the app says to the room on its own: the
    /// service starts in five minutes, the room is being recorded. It is
    /// separate from the alert, which is the operator's own note, so the two
    /// can be on the screen at once.
    #[serde(default)]
    pub announcement: Option<LiveAlertMessage>,
}

impl Default for PropManager {
    fn default() -> Self {
        Self {
            props: Vec::new(),
            looks: Vec::new(),
            active_mask: Mask::None,
            active_look_id: None,
            live_message: None,
            announcement: None,
        }
    }
}

impl PropManager {
    pub fn with_default_looks() -> Self {
        let default_look = Look::with_default_screens("Default");
        let active_id = default_look.id.clone();
        Self {
            announcement: None,
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
    /// Turn a prop on or off.
    ///
    /// The live look decides which props are on the output, so the prop
    /// joins or leaves it: a panel that says "On" while nothing is drawn is
    /// a lie the operator finds out on the projector.
    pub fn toggle_prop(&mut self, id: &str) {
        let mut now_visible = false;
        if let Some(prop) = self.props.iter_mut().find(|p| p.id == id) {
            prop.visible = !prop.visible;
            now_visible = prop.visible;
        }
        if let Some(look) = self.active_look_mut() {
            let present = look.active_prop_ids.iter().any(|prop_id| prop_id == id);
            if now_visible && !present {
                look.active_prop_ids.push(id.to_string());
            } else if !now_visible && present {
                look.active_prop_ids.retain(|prop_id| prop_id != id);
            }
        }
    }

    pub fn remove_prop(&mut self, id: &str) {
        self.props.retain(|p| p.id != id);
        // The prop is gone, so it cannot be part of the live look either.
        if let Some(look) = self.active_look_mut() {
            look.active_prop_ids.retain(|prop_id| prop_id != id);
        }
    }

    /// Add a prop to the library.
    ///
    /// A new prop is on, so it joins the live look: a lower third that the
    /// operator created would otherwise not appear on the output.
    pub fn add_prop(&mut self, prop: Prop) {
        let id = prop.id.clone();
        let visible = prop.visible;
        self.props.push(prop);
        if visible
            && let Some(look) = self.active_look_mut()
            && !look.active_prop_ids.iter().any(|prop_id| prop_id == &id)
        {
            look.active_prop_ids.push(id);
        }
    }

    /// The live look, when there is one.
    fn active_look_mut(&mut self) -> Option<&mut Look> {
        let active_id = self.active_look_id.clone()?;
        self.looks.iter_mut().find(|look| look.id == active_id)
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
        // The per-screen configuration belongs to the look. A saved look
        // that dropped it would turn every layer back on for every screen
        // when it was applied: the stage monitor would grow lower thirds and
        // the broadcast would start carrying the mask.
        if let Some(current) = self.current_look() {
            look.screens = current.screens.clone();
            look.theme_id = current.theme_id.clone();
        }
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

    /// The look that is live.
    ///
    /// It is the look the output is showing. When there is none, there is
    /// no live look: falling back to the first one would name a look as
    /// current that is not on the output.
    pub fn current_look(&self) -> Option<&Look> {
        self.active_look_id
            .as_deref()
            .and_then(|id| self.looks.iter().find(|l| l.id == id))
    }

    pub fn set_message(&mut self, text: impl Into<String>) {
        let t = text.into();
        if t.trim().is_empty() {
            self.live_message = None;
        } else {
            self.live_message = Some(LiveAlertMessage::new(t));
        }
    }

    /// Show an announcement.
    pub fn set_announcement(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text.trim().is_empty() {
            self.announcement = None;
        } else {
            self.announcement = Some(LiveAlertMessage::new(text));
        }
    }

    /// Take the announcement off the screen.
    pub fn clear_announcement(&mut self) {
        self.announcement = None;
    }

    pub fn clear_message(&mut self) {
        self.live_message = None;
    }

    /// Remove a look.
    ///
    /// Removing the live look takes everything it put on the output with
    /// it: the mask and the props it showed would otherwise stay on the
    /// projector while the panel named a different look as current.
    pub fn remove_look(&mut self, id: &str) {
        self.looks.retain(|l| l.id != id);
        if self.active_look_id.as_deref() == Some(id) {
            self.active_look_id = None;
            self.active_mask = Mask::None;
            for prop in &mut self.props {
                prop.visible = false;
            }
        }
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        // Written atomically: an interrupted write must not leave a truncated
        // prop library behind.
        crate::atomic::write_atomic(path, json.as_bytes())?;
        Ok(())
    }

    /// Load the prop library.
    ///
    /// An unparseable file is kept as `props.json.bak`: the next save would
    /// otherwise destroy the library without a word.
    pub fn load_from_file(path: &std::path::Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&contents) {
            Ok(manager) => manager,
            Err(e) => {
                let bak = path.with_extension("json.bak");
                if let Err(rename_error) = std::fs::rename(path, &bak) {
                    log::warn!(
                        "could not back up the corrupt prop library to {bak:?}: {rename_error}"
                    );
                }
                log::warn!(
                    "prop library parse error ({path:?}): {e} — using defaults (original kept as .bak)"
                );
                Self::default()
            }
        }
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

    /// A prop the panel shows as "On" is on the output.
    ///
    /// The live look decides which props are drawn, so a prop that is
    /// turned on has to join it: otherwise the operator creates a lower
    /// third, the panel says "On" and nothing appears on the projector.
    #[test]
    fn a_prop_that_is_on_is_drawn() {
        let mut manager = PropManager::with_default_looks();
        let prop = Prop {
            id: "prop-1".to_string(),
            name: "Lower Third".to_string(),
            content: PropContent::Rectangle {
                color: [1.0, 0.0, 0.0, 1.0],
                corner_radius: 0.0,
            },
            x: 0.1,
            y: 0.8,
            width: 0.4,
            height: 0.1,
            visible: true,
        };
        manager.add_prop(prop.clone());

        let overlay = crate::ui::props::overlay_for_screen(&manager, "audience");
        assert!(
            overlay.props.iter().any(|p| p.id == "prop-1"),
            "a new prop is on the output"
        );

        // Turning it off takes it off the output.
        manager.toggle_prop("prop-1");
        let overlay = crate::ui::props::overlay_for_screen(&manager, "audience");
        assert!(
            !overlay.props.iter().any(|p| p.id == "prop-1"),
            "a prop that is off is not on the output"
        );
    }

    /// Removing the live look takes everything it put on the output with it.
    #[test]
    fn removing_the_live_look_clears_the_output() {
        let mut manager = PropManager::with_default_looks();
        let active = manager.active_look_id.clone().expect("a look is live");
        manager.active_mask = Mask::Letterbox { bar_fraction: 0.1 };
        let prop = Prop {
            id: "prop-1".to_string(),
            name: "Lower Third".to_string(),
            content: PropContent::Rectangle {
                color: [1.0, 0.0, 0.0, 1.0],
                corner_radius: 0.0,
            },
            x: 0.1,
            y: 0.8,
            width: 0.4,
            height: 0.1,
            visible: true,
        };
        manager.add_prop(prop);

        manager.remove_look(&active);

        let overlay = crate::ui::props::overlay_for_screen(&manager, "audience");
        assert!(
            overlay.props.is_empty(),
            "the props of the removed look are not on the output"
        );
        assert!(
            matches!(overlay.mask, Mask::None),
            "the mask of the removed look is not on the output"
        );
        assert!(manager.current_look().is_none(), "no look is live any more");
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

    /// A saved look keeps the per-screen configuration it was saved from.
    ///
    /// The rows of the Looks & Screens matrix belong to the look: a saved
    /// look that dropped them would turn every layer back on for every
    /// screen when it was applied.
    #[test]
    fn a_saved_look_keeps_its_per_screen_configuration() {
        let mut manager = PropManager::with_default_looks();
        let look_id = manager.active_look_id.clone().expect("a look is active");
        {
            let look = manager
                .looks
                .iter_mut()
                .find(|look| look.id == look_id)
                .expect("the look exists");
            let mut target = look.target_or_default("stream");
            target.props_enabled = false;
            target.mask_enabled = false;
            look.set_screen_target(target);
        }

        manager.save_look("Broadcast only");

        let saved = manager.looks.last().expect("the look was saved");
        let target = saved.target_for_screen("stream").expect("the row is there");
        assert!(
            !target.props_enabled,
            "the saved look keeps the screen's own configuration"
        );
        assert!(!target.mask_enabled);
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

    /// The announcement is the app's own word, so it is its own thing: it
    /// goes up and comes down without touching the operator's alert.
    #[test]
    fn the_announcement_is_its_own_message() {
        let mut manager = PropManager::default();
        manager.set_announcement("Service starts in 5 minutes");
        assert!(manager.announcement.is_some());
        assert!(manager.live_message.is_none());

        manager.set_message("Parent in the foyer");
        assert!(manager.live_message.is_some());
        assert!(manager.announcement.is_some());

        manager.clear_announcement();
        assert!(manager.announcement.is_none());
        // The operator's alert is still there: taking the announcement away
        // does not take the alert with it.
        assert!(manager.live_message.is_some());
    }

    /// An announcement with no text is no announcement.
    #[test]
    fn an_empty_announcement_is_none() {
        let mut manager = PropManager::default();
        manager.set_announcement("   ");
        assert!(manager.announcement.is_none());
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
