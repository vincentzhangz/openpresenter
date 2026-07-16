use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

pub mod props;
pub use props::{Look, Mask, Prop, PropContent, PropManager};

/// A presentation action that can be triggered by cues, macros, HTTP, or OSC.
///
/// Mirrors ProPresenter's "Actions" — the operations an operator or automation
/// can perform against the live output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    NextSlide,
    PrevSlide,
    GotoSlide(usize),
    BlackScreen(bool),
    ClearOutput,
    TriggerProp(String),
    StartTimer,
    StopTimer,
    ResetTimer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    #[serde(default)]
    pub ccli_number: Option<String>,
    #[serde(default)]
    pub key_signature: Option<String>,
    #[serde(default)]
    pub bpm: Option<u32>,
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
            ccli_number: None,
            key_signature: None,
            bpm: None,
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
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub layers: Vec<Object>,
    #[serde(default)]
    pub cues: Vec<Cue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cue {
    pub id: String,
    pub name: String,
    pub action: Action,
    #[serde(default)]
    pub delay_ms: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Cue {
    pub fn new(name: impl Into<String>, action: Action) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            action,
            delay_ms: 0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlideContent {
    Text {
        text: String,
        style: TextStyle,
    },
    Image {
        path: String,
        #[serde(default)]
        fit: ImageFit,
    },
    Video {
        path: String,
        #[serde(default)]
        thumbnail: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    Fill,
    #[default]
    Fit,
    Stretch,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: String,
    pub content: ObjectContent,
    #[serde(default = "default_half")]
    pub position_x: f32,
    #[serde(default = "default_half")]
    pub position_y: f32,
    #[serde(default = "default_layer_width")]
    pub width: f32,
    #[serde(default = "default_layer_height")]
    pub height: f32,
    #[serde(default)]
    pub z_order: i32,
    #[serde(default = "default_one")]
    pub opacity: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Object {
    pub fn new_text(text: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: ObjectContent::Text {
                text,
                style: TextStyle::default(),
                runs: Vec::new(),
            },
            position_x: 0.5,
            position_y: 0.5,
            width: 0.9,
            height: 0.6,
            z_order: 0,
            opacity: 1.0,
            locked: false,
            visible: true,
        }
    }

    pub fn new_shape(shape_type: ShapeType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: ObjectContent::Shape {
                shape_type,
                fill: Color {
                    r: 70,
                    g: 130,
                    b: 200,
                    a: 204,
                },
                stroke_color: Color::white(),
                stroke_width: 0.0,
                corner_radius: 0.0,
            },
            position_x: 0.5,
            position_y: 0.5,
            width: 0.4,
            height: 0.25,
            z_order: 0,
            opacity: 1.0,
            locked: false,
            visible: true,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match &self.content {
            ObjectContent::Text { .. } => "Text",
            ObjectContent::Image { .. } => "Image",
            ObjectContent::Video { .. } => "Video",
            ObjectContent::Shape { shape_type, .. } => match shape_type {
                ShapeType::Rectangle => "Rectangle",
                ShapeType::Ellipse => "Ellipse",
                ShapeType::Triangle => "Triangle",
                ShapeType::Line => "Line",
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectContent {
    Text {
        text: String,
        style: TextStyle,
        #[serde(default)]
        runs: Vec<TextRun>,
    },
    Image {
        path: String,
        #[serde(default)]
        fit: ImageFit,
    },
    Video {
        path: String,
        #[serde(default)]
        thumbnail: Option<String>,
    },
    Shape {
        shape_type: ShapeType,
        fill: Color,
        stroke_color: Color,
        stroke_width: f32,
        corner_radius: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Triangle,
    Line,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub color: Option<Color>,
    #[serde(default)]
    pub font_size: Option<f32>,
    #[serde(default)]
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextTransform {
    #[default]
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

impl TextTransform {
    pub fn apply(&self, s: &str) -> String {
        match self {
            Self::None => s.to_owned(),
            Self::Uppercase => s.to_uppercase(),
            Self::Lowercase => s.to_lowercase(),
            Self::Capitalize => s
                .split_whitespace()
                .map(|word| {
                    let mut c = word.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: f32,
    pub color: Color,
    pub alignment: TextAlignment,
    pub shadow: bool,
    pub outline: bool,
    #[serde(default = "default_half")]
    pub position_x: f32,
    #[serde(default = "default_half")]
    pub position_y: f32,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default = "default_line_height_multiplier")]
    pub line_height_multiplier: f32,
    #[serde(default)]
    pub letter_spacing: f32,
    #[serde(default)]
    pub text_transform: TextTransform,
    #[serde(default)]
    pub glow_enabled: bool,
    #[serde(default = "Color::white")]
    pub glow_color: Color,
    #[serde(default = "default_glow_radius")]
    pub glow_radius: f32,
    #[serde(default)]
    pub text_stroke_width: f32,
    #[serde(default = "Color::black")]
    pub text_stroke_color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Solid(Color),
    Image(String),
    Video(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    #[default]
    Cut,
    Fade {
        duration_ms: u64,
    },
    Slide {
        duration_ms: u64,
    },
    Dissolve {
        duration_ms: u64,
    },
    Push {
        duration_ms: u64,
        direction: u8,
    },
    Zoom {
        duration_ms: u64,
    },
    Flip {
        duration_ms: u64,
    },
    Clock {
        duration_ms: u64,
    },
    Wipe {
        duration_ms: u64,
        angle_deg: i32,
    },
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
            position_x: 0.5,
            position_y: 0.5,
            bold: false,
            italic: false,
            line_height_multiplier: 1.2,
            letter_spacing: 0.0,
            text_transform: TextTransform::None,
            glow_enabled: false,
            glow_color: Color::white(),
            glow_radius: 8.0,
            text_stroke_width: 0.0,
            text_stroke_color: Color::black(),
        }
    }
}

fn default_line_height_multiplier() -> f32 {
    1.2
}

fn default_glow_radius() -> f32 {
    8.0
}

fn default_half() -> f32 {
    0.5
}

fn default_one() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_layer_width() -> f32 {
    0.9
}

fn default_layer_height() -> f32 {
    0.6
}

impl Default for Background {
    fn default() -> Self {
        Background::Solid(Color::black())
    }
}

impl Slide {
    pub fn new_text(text: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: SlideContent::Text {
                text,
                style: TextStyle::default(),
            },
            background: Background::default(),
            transition: Transition::default(),
            group: None,
            notes: None,
            layers: Vec::new(),
            cues: Vec::new(),
        }
    }

    pub fn new_text_in_group(text: String, group: String) -> Self {
        let mut s = Self::new_text(text);
        s.group = Some(group);
        s
    }

    pub fn effective_layers(&self) -> Cow<'_, [Object]> {
        if !self.layers.is_empty() {
            return Cow::Borrowed(&self.layers);
        }
        let (content, pos_x, pos_y, w, h) = match &self.content {
            SlideContent::Text { text, style } => (
                ObjectContent::Text {
                    text: text.clone(),
                    style: style.clone(),
                    runs: Vec::new(),
                },
                style.position_x,
                style.position_y,
                0.9_f32,
                0.6_f32,
            ),
            SlideContent::Image { path, fit } => (
                ObjectContent::Image {
                    path: path.clone(),
                    fit: *fit,
                },
                0.5,
                0.5,
                1.0,
                1.0,
            ),
            SlideContent::Video { path, thumbnail } => (
                ObjectContent::Video {
                    path: path.clone(),
                    thumbnail: thumbnail.clone(),
                },
                0.5,
                0.5,
                1.0,
                1.0,
            ),
        };
        Cow::Owned(vec![Object {
            id: format!("legacy-{}", self.id),
            content,
            position_x: pos_x,
            position_y: pos_y,
            width: w,
            height: h,
            z_order: 0,
            opacity: 1.0,
            locked: false,
            visible: true,
        }])
    }

    pub fn migrate_to_objects(&mut self) -> bool {
        if !self.layers.is_empty() {
            return false;
        }
        self.layers = self.effective_layers().into_owned();
        true
    }
}

impl Color {
    pub const fn black() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub const fn white() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }

    pub const fn red() -> Self {
        Self {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub const fn blue() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        }
    }

    pub const fn yellow() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        }
    }

    pub const fn green() -> Self {
        Self {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideTheme {
    pub id: String,
    pub name: String,
    pub background: Background,
    pub default_text_style: TextStyle,
    pub default_transition: Transition,
    pub created_at: DateTime<Utc>,
}

impl SlideTheme {
    pub fn new(
        name: String,
        background: Background,
        default_text_style: TextStyle,
        default_transition: Transition,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            background,
            default_text_style,
            default_transition,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryAsset {
    pub id: String,
    pub name: String,
    pub path: String,
    pub media_type: String,
    pub created_at: DateTime<Utc>,
}

impl LibraryAsset {
    pub fn is_image(&self) -> bool {
        self.media_type == "image"
    }

    pub fn is_video(&self) -> bool {
        self.media_type == "video"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlaylistItem {
    Presentation { id: String, name: String },
    Song { id: String, title: String },
    MediaCue { path: String },
    Header { text: String },
    Blank,
}

impl PlaylistItem {
    pub fn display_name(&self) -> String {
        match self {
            Self::Presentation { name, .. } => name.clone(),
            Self::Song { title, .. } => title.clone(),
            Self::MediaCue { path } => std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string(),
            Self::Header { text } => format!("— {text} —"),
            Self::Blank => String::from("(blank)"),
        }
    }

    pub fn type_label(&self) -> &'static str {
        match self {
            Self::Presentation { .. } => "PRES",
            Self::Song { .. } => "SONG",
            Self::MediaCue { .. } => "MEDIA",
            Self::Header { .. } => "HDR",
            Self::Blank => "BLANK",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub items: Vec<PlaylistItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Playlist {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            items: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BibleTranslation {
    pub id: String,
    pub name: String,
    pub abbr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BibleVerse {
    pub id: String,
    pub translation_id: String,
    pub book: String,
    pub chapter: i32,
    pub verse: i32,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct BibleImportRecord {
    pub book: String,
    pub chapter: i32,
    pub verse: i32,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct BibleImportFile {
    pub name: String,
    pub abbr: String,
    pub verses: Vec<BibleImportRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_transform_none_is_identity() {
        assert_eq!(TextTransform::None.apply("Hello World"), "Hello World");
    }

    #[test]
    fn text_transform_uppercase() {
        assert_eq!(TextTransform::Uppercase.apply("hello world"), "HELLO WORLD");
    }

    #[test]
    fn text_transform_lowercase() {
        assert_eq!(TextTransform::Lowercase.apply("HELLO WORLD"), "hello world");
    }

    #[test]
    fn text_transform_capitalize_each_word() {
        assert_eq!(
            TextTransform::Capitalize.apply("amazing grace how sweet"),
            "Amazing Grace How Sweet"
        );
    }

    #[test]
    fn text_transform_capitalize_empty_string() {
        assert_eq!(TextTransform::Capitalize.apply(""), "");
    }

    #[test]
    fn text_transform_capitalize_single_word() {
        assert_eq!(TextTransform::Capitalize.apply("grace"), "Grace");
    }

    #[test]
    fn color_black() {
        let c = Color::black();
        assert_eq!((c.r, c.g, c.b, c.a), (0, 0, 0, 255));
    }

    #[test]
    fn color_white() {
        let c = Color::white();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 255, 255, 255));
    }

    #[test]
    fn color_red() {
        let c = Color::red();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 0, 0, 255));
    }

    #[test]
    fn color_equality() {
        assert_eq!(Color::white(), Color::white());
        assert_ne!(Color::black(), Color::white());
    }

    #[test]
    fn transition_default_is_cut() {
        assert_eq!(Transition::default(), Transition::Cut);
    }

    #[test]
    fn image_fit_default_is_fit() {
        assert_eq!(ImageFit::default(), ImageFit::Fit);
    }

    #[test]
    fn slide_new_text_starts_with_empty_cues() {
        let slide = Slide::new_text("Hello".to_string());
        assert!(slide.cues.is_empty());
    }

    #[test]
    fn slide_cue_new_defaults_enabled_without_delay() {
        let cue = Cue::new("Black", Action::BlackScreen(true));
        assert_eq!(cue.name, "Black");
        assert!(cue.enabled);
        assert_eq!(cue.delay_ms, 0);
    }

    fn make_text_slide() -> Slide {
        Slide {
            id: "test-id".to_string(),
            content: SlideContent::Text {
                text: "Hello".to_string(),
                style: TextStyle::default(),
            },
            background: Background::Solid(Color::black()),
            transition: Transition::Cut,
            group: None,
            notes: None,
            layers: Vec::new(),
            cues: Vec::new(),
        }
    }

    #[test]
    fn effective_layers_with_no_layers_synthesises_one() {
        let slide = make_text_slide();
        let layers = slide.effective_layers();
        assert_eq!(layers.len(), 1);
        match &layers[0].content {
            ObjectContent::Text { text, .. } => assert_eq!(text, "Hello"),
            _ => panic!("expected text layer"),
        }
    }

    #[test]
    fn effective_layers_with_layers_borrows_in_place() {
        let mut slide = make_text_slide();
        slide.layers.push(Object::new_text("Layer A".to_string()));
        let layers = slide.effective_layers();
        assert_eq!(layers.len(), 1);
        match &layers[0].content {
            ObjectContent::Text { text, .. } => assert_eq!(text, "Layer A"),
            _ => panic!("expected text layer"),
        }
    }

    #[test]
    fn effective_layers_with_no_layers_is_cow_owned() {
        let slide = make_text_slide();
        let layers = slide.effective_layers();
        assert!(matches!(layers, Cow::Owned(_)));
    }

    #[test]
    fn effective_layers_with_layers_is_cow_borrowed() {
        let mut slide = make_text_slide();
        slide.layers.push(Object::new_text("x".to_string()));
        let layers = slide.effective_layers();
        assert!(matches!(layers, Cow::Borrowed(_)));
    }

    #[test]
    fn song_new_sets_title() {
        let song = Song::new("Test Song".to_string());
        assert_eq!(song.title, "Test Song");
        assert!(song.verses.is_empty());
    }

    #[test]
    fn verse_new_sets_fields() {
        let v = Verse::new("v1".to_string(), "content".to_string(), 0);
        assert_eq!(v.label, "v1");
        assert_eq!(v.content, "content");
        assert_eq!(v.order_index, 0);
    }

    #[test]
    fn presentation_new_has_no_slides() {
        let p = Presentation::new("My Show".to_string());
        assert_eq!(p.name, "My Show");
        assert!(p.slides.is_empty());
    }
}
