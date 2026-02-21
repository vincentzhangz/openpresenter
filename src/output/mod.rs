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
        m.add(NamedOutput::new_window("stage", "Stage Monitor"));
        m
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
