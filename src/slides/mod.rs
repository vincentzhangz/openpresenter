use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub verses: Vec<Verse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Song {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            artist: None,
            copyright: None,
            verses: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verse {
    pub id: String,
    pub label: String,
    pub content: String,
    pub order_index: usize,
}

impl Verse {
    pub fn new(label: String, content: String, order_index: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label,
            content,
            order_index,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presentation {
    pub id: String,
    pub name: String,
    pub slides: Vec<Slide>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Presentation {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            slides: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub id: String,
    pub content: SlideContent,
    pub background: Background,
    pub transition: Transition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlideContent {
    Text { text: String, style: TextStyle },
    Image { path: String },
    Video { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f32,
    pub color: Color,
    pub alignment: TextAlignment,
    pub shadow: bool,
    pub outline: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Background {
    Solid(Color),
    Image(String),
    Video(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Transition {
    Cut,
    Fade { duration_ms: u64 },
    Slide { duration_ms: u64 },
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Arial".to_string(),
            font_size: 72.0,
            color: Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            alignment: TextAlignment::Center,
            shadow: true,
            outline: false,
        }
    }
}
