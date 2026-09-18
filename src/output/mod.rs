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
}
