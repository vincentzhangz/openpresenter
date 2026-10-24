pub mod calendar;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

pub mod props;
pub use props::{LiveAlertMessage, Look, Mask, Prop, PropContent, PropManager, ScreenLookTarget};

/// A presentation action that can be triggered by cues, macros, HTTP, or OSC.
///
/// Defines the operations an operator or automation can perform against the live output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    NextSlide,
    PrevSlide,
    GotoSlide(usize),
    BlackScreen(bool),
    ClearOutput,
    ClearSlide,
    ClearMedia,
    ClearProps,
    ClearMessages,
    ClearAudio,
    TriggerProp(String),
    ApplyLook(String),
    ShowMessage(String),
    HideMessage,
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
    /// The show as a piece of time.
    #[serde(default)]
    pub timeline: Timeline,
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
            timeline: Timeline::default(),
        }
    }
}

/// The show as a piece of time.
///
/// The timeline is the slides in the order they play, and how long each one
/// is up. An operator can run the show from it — the slides advance when
/// their time is up — or leave it alone and advance them by hand, which is
/// what a service usually needs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    /// How long the whole timeline runs, in seconds.
    ///
    /// Zero means the timeline has no length of its own: the slides wait for
    /// the operator, as they always did.
    #[serde(default)]
    pub duration_secs: u64,
    /// Whether the timeline starts again when it reaches the end.
    #[serde(default)]
    pub loop_playback: bool,
    /// The timecode the timeline starts at.
    #[serde(default)]
    pub base_timecode_secs: u64,
    /// How long each slide is up, in seconds — one per slide of the
    /// presentation.
    ///
    /// A slide with no time of its own waits for the operator, so a service
    /// can have one timed slide among slides that are not.
    #[serde(default)]
    pub slide_durations: Vec<u64>,
}

impl Timeline {
    /// How long a slide is up.
    pub fn duration_for(&self, index: usize) -> u64 {
        self.slide_durations.get(index).copied().unwrap_or(0)
    }

    /// Set how long a slide is up.
    pub fn set_duration(&mut self, index: usize, seconds: u64) {
        if self.slide_durations.len() <= index {
            self.slide_durations.resize(index + 1, 0);
        }
        self.slide_durations[index] = seconds;
    }

    /// The second the slide starts at.
    pub fn start_of(&self, index: usize) -> u64 {
        let mut at = 0;
        for slide in 0..index {
            at += self.duration_for(slide);
        }
        at
    }

    /// The whole length of the timeline.
    pub fn total_secs(&self, slide_count: usize) -> u64 {
        if self.duration_secs > 0 {
            return self.duration_secs;
        }
        (0..slide_count).map(|index| self.duration_for(index)).sum()
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

    /// A cue that fires after a delay, in milliseconds.
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
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

/// The kind of content a slide carries.
///
/// A slide is a title, a picture or a clip. The editor's content-type
/// control shows which one the slide is, and converting a slide has to make
/// the picture match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlideKind {
    Text,
    Image,
    Video,
}

/// The media on a slide.
///
/// The file may be empty: the slide is a media slide that is waiting for one.
#[derive(Debug, Clone, Copy)]
pub enum SlideMedia<'a> {
    Image {
        path: &'a str,
        fit: ImageFit,
    },
    Video {
        path: &'a str,
        thumbnail: Option<&'a str>,
    },
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
    /// The object is drawn mirrored left to right.
    #[serde(default)]
    pub flip_horizontal: bool,
    /// The object is drawn mirrored top to bottom.
    #[serde(default)]
    pub flip_vertical: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Object {
    /// Move the object by a number of slide pixels.
    ///
    /// The step is a step on the *slide*, not on the screen: an object that
    /// moves one pixel on a 4K projector has to move one pixel on a 720p one
    /// too, so the step is measured against the resolution the slide was
    /// authored at.
    ///
    /// The two axes are measured against their own side: the vertical
    /// position is a fraction of the slide's height, so dividing it by the
    /// width would make the up and down keys move a different distance from
    /// the left and right ones.
    ///
    /// Returns `false` when the object did not move — a locked object, or a
    /// slide without a reference width.
    pub fn nudge(&mut self, dx: f32, dy: f32, reference_width: f32, reference_height: f32) -> bool {
        if self.locked || reference_width <= 0.0 || reference_height <= 0.0 {
            return false;
        }
        self.position_x = (self.position_x + dx / reference_width).clamp(0.0, 1.0);
        self.position_y = (self.position_y + dy / reference_height).clamp(0.0, 1.0);
        true
    }

    /// The text the object carries, if it is a text object.
    ///
    /// The draw order is a list of objects, so a test that says which object
    /// is where needs to be able to read what each one says.
    pub fn content_text(&self) -> Option<&str> {
        match &self.content {
            ObjectContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        }
    }

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
            flip_horizontal: false,
            flip_vertical: false,
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
            flip_horizontal: false,
            flip_vertical: false,
            visible: true,
        }
    }

    /// An image object covering the whole slide.
    pub fn new_image(path: impl Into<String>, fit: ImageFit) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: ObjectContent::Image {
                path: path.into(),
                fit,
            },
            position_x: 0.5,
            position_y: 0.5,
            width: 1.0,
            height: 1.0,
            z_order: 0,
            opacity: 1.0,
            locked: false,
            flip_horizontal: false,
            flip_vertical: false,
            visible: true,
        }
    }

    /// A video object covering the whole slide.
    pub fn new_video(path: impl Into<String>, thumbnail: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: ObjectContent::Video {
                path: path.into(),
                thumbnail,
            },
            position_x: 0.5,
            position_y: 0.5,
            width: 1.0,
            height: 1.0,
            z_order: 0,
            opacity: 1.0,
            locked: false,
            flip_horizontal: false,
            flip_vertical: false,
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

/// How the letters of a text object are cased.
///
/// The set is the one the operator finds in the Text menu: the text is shown
/// as typed, in all caps, in small caps, in title case, or with every word
/// started with a capital.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextTransform {
    #[default]
    None,
    /// Every letter is upper case.
    Uppercase,
    /// Every letter is lower case.
    Lowercase,
    /// Every word starts with a capital.
    StartCase,
    /// The letters are upper case but drawn smaller.
    SmallCaps,
    /// The words that carry the meaning start with a capital.
    TitleCase,
}

/// The words that stay lower case in a title, however the sentence is built.
///
/// They are the words that join the sentence together, so a title that
/// capitalised them would read as a row of shouts.
const TITLE_MINOR_WORDS: [&str; 16] = [
    "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "of", "on", "or", "the", "to",
    "up",
];

impl TextTransform {
    /// The name the Text tab and the Text menu show for this casing.
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Uppercase => "All Caps",
            Self::Lowercase => "Lower Case",
            Self::StartCase => "Start Case",
            Self::SmallCaps => "Small Caps",
            Self::TitleCase => "Title Case",
        }
    }

    /// Every casing the operator can choose, in the order the menu shows them.
    pub const ALL: [TextTransform; 6] = [
        TextTransform::None,
        TextTransform::Uppercase,
        TextTransform::SmallCaps,
        TextTransform::TitleCase,
        TextTransform::StartCase,
        TextTransform::Lowercase,
    ];

    /// Whether the letters are drawn as capitals but smaller.
    pub fn is_small_caps(&self) -> bool {
        matches!(self, Self::SmallCaps)
    }

    pub fn apply(&self, s: &str) -> String {
        match self {
            Self::None => s.to_owned(),
            Self::Uppercase | Self::SmallCaps => s.to_uppercase(),
            Self::Lowercase => s.to_lowercase(),
            Self::StartCase => s
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
            Self::TitleCase => {
                let mut first = true;
                s.split_whitespace()
                    .map(|word| {
                        let lower = word.to_lowercase();
                        // The first word of the text is always a major word,
                        // so a title that begins with a small word still
                        // reads as a title.
                        let major = first || !TITLE_MINOR_WORDS.contains(&lower.as_str());
                        first = false;
                        if major {
                            let mut c = lower.chars();
                            match c.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().to_string() + c.as_str(),
                            }
                        } else {
                            lower
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }
}

/// A change to the letters of the text itself.
///
/// The Transformations menu rewrites the words: a text that says "hello"
/// and is made upper case says "HELLO" afterwards. The change is in the
/// model and not in the style, so the words and the way they are drawn
/// cannot disagree about what the audience reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRewrite {
    /// Every letter becomes a capital.
    UpperCase,
    /// Every letter becomes a small letter.
    LowerCase,
    /// Every word starts with a capital.
    Capitalize,
}

impl TextRewrite {
    /// The name the Transformations menu shows for this change.
    pub fn label(&self) -> &'static str {
        match self {
            Self::UpperCase => "Make Upper Case",
            Self::LowerCase => "Make Lower Case",
            Self::Capitalize => "Capitalize",
        }
    }

    /// Every change the operator can make, in the order the menu shows them.
    pub const ALL: [TextRewrite; 3] = [
        TextRewrite::UpperCase,
        TextRewrite::LowerCase,
        TextRewrite::Capitalize,
    ];

    /// The words, rewritten.
    pub fn apply(&self, text: &str) -> String {
        match self {
            Self::UpperCase => text.to_uppercase(),
            Self::LowerCase => text.to_lowercase(),
            Self::Capitalize => capitalize_words(text),
        }
    }
}

/// Every word, started with a capital.
///
/// The rest of the word is left as it was: an operator who capitalises
/// "well-known" expects "Well-Known", not "WELL-KNOWN".
fn capitalize_words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut start_of_word = true;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if start_of_word {
                out.extend(ch.to_uppercase());
            } else {
                out.push(ch);
            }
            start_of_word = false;
        } else {
            out.push(ch);
            start_of_word = true;
        }
    }
    out
}

/// Where the text sits on its line.
///
/// The text is raised or lowered by a fraction of the font size, so a
/// superscript reads as a note above the line and a subscript as a note
/// below it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Baseline {
    #[default]
    Default,
    Raise,
    Lower,
    Superscript,
    Subscript,
}

impl Baseline {
    /// The name the Text menu shows for this baseline.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Raise => "Raise",
            Self::Lower => "Lower",
            Self::Superscript => "Superscript",
            Self::Subscript => "Subscript",
        }
    }

    /// Every baseline the operator can choose, in the order the menu shows them.
    pub const ALL: [Baseline; 5] = [
        Baseline::Default,
        Baseline::Raise,
        Baseline::Lower,
        Baseline::Superscript,
        Baseline::Subscript,
    ];

    /// How far the text is shifted, as a fraction of the font size.
    ///
    /// A superscript is raised and made smaller; a subscript is lowered and
    /// made smaller.
    pub fn offset(&self) -> f32 {
        match self {
            Self::Default => 0.0,
            Self::Raise => 0.15,
            Self::Lower => -0.15,
            Self::Superscript => 0.33,
            Self::Subscript => -0.2,
        }
    }

    /// The size the text is drawn at, as a fraction of the font size.
    ///
    /// A superscript and a subscript are notes: they are drawn smaller than
    /// the text they sit in.
    pub fn size_scale(&self) -> f32 {
        match self {
            Self::Default | Self::Raise | Self::Lower => 1.0,
            Self::Superscript | Self::Subscript => 0.66,
        }
    }
}

/// Whether the font joins its letters together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ligatures {
    /// The font's own ligatures are used.
    #[default]
    Default,
    /// The letters are drawn on their own.
    None,
}

impl Ligatures {
    /// The name the Text menu shows for this setting.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Use Default",
            Self::None => "Use None",
        }
    }

    /// Every setting the operator can choose, in the order the menu shows them.
    pub const ALL: [Ligatures; 2] = [Ligatures::Default, Ligatures::None];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub baseline: Baseline,
    #[serde(default)]
    pub ligatures: Ligatures,
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

impl TextStyle {
    /// The smallest font size the editor accepts.
    ///
    /// Below this the text is a hairline on the projector.
    pub const MIN_FONT_SIZE: f32 = 4.0;

    /// The largest font size the editor accepts.
    pub const MAX_FONT_SIZE: f32 = 400.0;

    /// Clamp a font size to the range the editor accepts.
    ///
    /// The size is clamped the same way wherever it is typed: a size that
    /// one panel accepts and another silently changes is a value the
    /// operator cannot reason about.
    pub fn clamp_font_size(size: f32) -> f32 {
        size.clamp(Self::MIN_FONT_SIZE, Self::MAX_FONT_SIZE)
    }
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

impl Transition {
    pub fn duration_ms(&self) -> u64 {
        match self {
            Transition::Cut => 0,
            Transition::Fade { duration_ms }
            | Transition::Slide { duration_ms }
            | Transition::Dissolve { duration_ms }
            | Transition::Push { duration_ms, .. }
            | Transition::Zoom { duration_ms }
            | Transition::Flip { duration_ms }
            | Transition::Clock { duration_ms }
            | Transition::Wipe { duration_ms, .. } => *duration_ms,
        }
    }

    pub fn with_duration(&self, dur: u64) -> Self {
        match self {
            Transition::Cut => Transition::Cut,
            Transition::Fade { .. } => Transition::Fade { duration_ms: dur },
            Transition::Slide { .. } => Transition::Slide { duration_ms: dur },
            Transition::Dissolve { .. } => Transition::Dissolve { duration_ms: dur },
            Transition::Push { direction, .. } => Transition::Push {
                duration_ms: dur,
                direction: *direction,
            },
            Transition::Zoom { .. } => Transition::Zoom { duration_ms: dur },
            Transition::Flip { .. } => Transition::Flip { duration_ms: dur },
            Transition::Clock { .. } => Transition::Clock { duration_ms: dur },
            Transition::Wipe { angle_deg, .. } => Transition::Wipe {
                duration_ms: dur,
                angle_deg: *angle_deg,
            },
        }
    }

    /// Rebuild a transition from the string persisted in settings
    /// (`TransitionConfig::default_type`, written by the output settings tab),
    /// applying `duration_ms`. Unknown strings fall back to a fade so a
    /// hand-edited config can never produce a transition the renderer rejects.
    pub fn from_config_str(s: &str, duration_ms: u64) -> Self {
        match s {
            "cut" => Transition::Cut,
            "slide" => Transition::Slide { duration_ms },
            "dissolve" => Transition::Dissolve { duration_ms },
            "zoom" => Transition::Zoom { duration_ms },
            "flip" => Transition::Flip { duration_ms },
            "clock" => Transition::Clock { duration_ms },
            "push_0" => Transition::Push {
                duration_ms,
                direction: 0,
            },
            "push_1" => Transition::Push {
                duration_ms,
                direction: 1,
            },
            "wipe_0" => Transition::Wipe {
                duration_ms,
                angle_deg: 0,
            },
            "wipe_180" => Transition::Wipe {
                duration_ms,
                angle_deg: 180,
            },
            _ => Transition::Fade { duration_ms },
        }
    }
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
            underline: false,
            strikethrough: false,
            baseline: Baseline::Default,
            ligatures: Ligatures::Default,
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
    /// The text object the operator is editing.
    ///
    /// The selected object wins, then the first text object on the slide.
    /// The slide's own content is only used when it has no objects at all:
    /// the renderer draws the objects, so the editors have to change the
    /// objects.
    pub fn editable_text_mut(
        &mut self,
        selected: Option<usize>,
    ) -> Option<(&mut String, &mut TextStyle)> {
        // Which object is the target? The selected one, else the first text
        // object on the slide.
        let target = selected
            .filter(|idx| {
                self.layers
                    .get(*idx)
                    .is_some_and(|layer| matches!(layer.content, ObjectContent::Text { .. }))
            })
            .or_else(|| {
                self.layers
                    .iter()
                    .position(|layer| matches!(layer.content, ObjectContent::Text { .. }))
            });

        if let Some(idx) = target {
            let layer = self.layers.get_mut(idx)?;
            if let ObjectContent::Text { text, style, .. } = &mut layer.content {
                return Some((text, style));
            }
            return None;
        }

        // A slide with no objects at all: its content is the object.
        if self.layers.is_empty()
            && let SlideContent::Text { text, style } = &mut self.content
        {
            return Some((text, style));
        }
        None
    }

    /// The text the operator is editing, rewritten.
    ///
    /// The change goes through the same rule every other text control uses,
    /// so the slide's text and the editor's mirror cannot disagree about
    /// which text is being edited.
    pub fn rewrite_editable_text(&mut self, selected: Option<usize>, rewrite: TextRewrite) -> bool {
        let Some((text, _)) = self.editable_text_mut(selected) else {
            return false;
        };
        let rewritten = rewrite.apply(text);
        *text = rewritten;
        true
    }

    /// The read-only counterpart of [`Self::editable_text_mut`].
    pub fn editable_text(&self, selected: Option<usize>) -> Option<(&String, &TextStyle)> {
        let target = selected
            .filter(|idx| {
                self.layers
                    .get(*idx)
                    .is_some_and(|layer| matches!(layer.content, ObjectContent::Text { .. }))
            })
            .or_else(|| {
                self.layers
                    .iter()
                    .position(|layer| matches!(layer.content, ObjectContent::Text { .. }))
            });

        if let Some(idx) = target {
            let layer = self.layers.get(idx)?;
            if let ObjectContent::Text { text, style, .. } = &layer.content {
                return Some((text, style));
            }
            return None;
        }

        if self.layers.is_empty()
            && let SlideContent::Text { text, style } = &self.content
        {
            return Some((text, style));
        }
        None
    }

    /// The video the slide plays, if it has one.
    ///
    /// A video can be a media object, the slide's own content (older
    /// slides) or the slide's background. Every reader has to agree on where
    /// the video lives, or the player starts while the output shows nothing.
    pub fn video_path(&self) -> Option<String> {
        for layer in &self.layers {
            if let ObjectContent::Video { path, .. } = &layer.content
                && !path.is_empty()
            {
                return Some(path.clone());
            }
        }
        if let SlideContent::Video { path, .. } = &self.content
            && !path.is_empty()
        {
            return Some(path.clone());
        }
        if let Background::Video(path) = &self.background
            && !path.is_empty()
        {
            return Some(path.clone());
        }
        None
    }

    /// The image the slide shows, if it has one.
    pub fn image(&self) -> Option<(String, ImageFit)> {
        for layer in &self.layers {
            if let ObjectContent::Image { path, fit } = &layer.content
                && !path.is_empty()
            {
                return Some((path.clone(), *fit));
            }
        }
        match &self.content {
            SlideContent::Image { path, fit } if !path.is_empty() => Some((path.clone(), *fit)),
            _ => None,
        }
    }

    /// The kind of content the slide carries.
    ///
    /// The objects decide, because they are the picture: a slide whose
    /// objects hold an image is an image slide, whatever its legacy content
    /// says. A slide without objects is the kind its legacy content is.
    pub fn content_kind(&self) -> SlideKind {
        if self.layers.is_empty() {
            return match self.content {
                SlideContent::Text { .. } => SlideKind::Text,
                SlideContent::Image { .. } => SlideKind::Image,
                SlideContent::Video { .. } => SlideKind::Video,
            };
        }
        if self
            .layers
            .iter()
            .any(|layer| matches!(layer.content, ObjectContent::Image { .. }))
        {
            return SlideKind::Image;
        }
        if self
            .layers
            .iter()
            .any(|layer| matches!(layer.content, ObjectContent::Video { .. }))
        {
            return SlideKind::Video;
        }
        SlideKind::Text
    }

    /// The media on the slide, with the file it may not have yet.
    ///
    /// A media slide is one media object, and the file may not be chosen for
    /// it yet: the panel has to offer the picker, or the slide is a blank
    /// the operator cannot fill.
    pub fn media(&self) -> Option<SlideMedia<'_>> {
        for layer in &self.layers {
            match &layer.content {
                ObjectContent::Image { path, fit } => {
                    return Some(SlideMedia::Image { path, fit: *fit });
                }
                ObjectContent::Video { path, thumbnail } => {
                    return Some(SlideMedia::Video {
                        path,
                        thumbnail: thumbnail.as_deref(),
                    });
                }
                _ => {}
            }
        }
        match &self.content {
            SlideContent::Image { path, fit } => Some(SlideMedia::Image { path, fit: *fit }),
            SlideContent::Video { path, thumbnail } => Some(SlideMedia::Video {
                path,
                thumbnail: thumbnail.as_deref(),
            }),
            SlideContent::Text { .. } => None,
        }
    }

    /// The thumbnail of the slide's video, if it has one.
    pub fn video_thumbnail(&self) -> Option<String> {
        for layer in &self.layers {
            if let ObjectContent::Video { thumbnail, .. } = &layer.content {
                return thumbnail.clone();
            }
        }
        match &self.content {
            SlideContent::Video { thumbnail, .. } => thumbnail.clone(),
            _ => None,
        }
    }

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
            flip_horizontal: false,
            flip_vertical: false,
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

    /// The highest depth an object on the slide is drawn at.
    ///
    /// The picture is drawn back to front, so a new object has to go above
    /// the others to be seen.
    pub fn top_z_order(&self) -> i32 {
        self.layers
            .iter()
            .map(|layer| layer.z_order)
            .max()
            .unwrap_or(0)
    }

    /// Add an object on top of the slide's others and return its index.
    ///
    /// The slide is drawn back to front, so an object with the default depth
    /// would be hidden behind the ones that are already there.
    pub fn add_object_on_top(&mut self, mut object: Object) -> usize {
        object.z_order = self.top_z_order() + 1;
        self.layers.push(object);
        self.layers.len() - 1
    }

    /// Make the draw order follow the list order.
    ///
    /// The picture is drawn back to front and the list is what the operator
    /// reorders, so the two have to agree. The depths are renumbered after
    /// every move: nudging two depths by one breaks as soon as the depths
    /// are not consecutive, and an object that keeps its place in the
    /// picture is a move that did nothing.
    pub fn renumber_z_order(&mut self) {
        for (index, layer) in self.layers.iter_mut().enumerate() {
            layer.z_order = index as i32;
        }
    }

    // -- Arrangement --------------------------------------------------------
    //
    // The objects on a slide can be lined up, stacked and mirrored. The
    // commands live here so the menu, the canvas and the inspector all
    // arrange a slide the same way: an object that moves in the menu and
    // stays put in the editor is an arrangement the operator cannot trust.

    /// Line the selected objects up.
    ///
    /// One object is lined up with the slide, because that is what the
    /// operator means by "align centre" for a single title. Several objects
    /// are lined up with each other, because the slide is not what the
    /// operator is looking at — the selection is.
    pub fn align_objects(&mut self, indices: &[usize], mode: AlignMode) -> bool {
        let indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|i| *i < self.layers.len())
            .collect();
        if indices.is_empty() {
            return false;
        }

        // The frame the objects are aligned inside.
        let (frame_x, frame_y, frame_w, frame_h) = if indices.len() == 1 {
            (0.0, 0.0, 1.0, 1.0)
        } else {
            let mut left = f32::MAX;
            let mut top = f32::MAX;
            let mut right = f32::MIN;
            let mut bottom = f32::MIN;
            for index in &indices {
                let object = &self.layers[*index];
                left = left.min(object.position_x);
                top = top.min(object.position_y);
                right = right.max(object.position_x + object.width);
                bottom = bottom.max(object.position_y + object.height);
            }
            (left, top, right - left, bottom - top)
        };

        let mut moved = false;
        for index in indices {
            let object = &mut self.layers[index];
            if object.locked {
                continue;
            }
            let before = (object.position_x, object.position_y);
            match mode {
                AlignMode::Left => object.position_x = frame_x,
                AlignMode::Center => object.position_x = frame_x + (frame_w - object.width) / 2.0,
                AlignMode::Right => object.position_x = frame_x + frame_w - object.width,
                AlignMode::Top => object.position_y = frame_y,
                AlignMode::Middle => object.position_y = frame_y + (frame_h - object.height) / 2.0,
                AlignMode::Bottom => object.position_y = frame_y + frame_h - object.height,
            }
            if before != (object.position_x, object.position_y) {
                moved = true;
            }
        }
        moved
    }

    /// Move the selected objects one step through the draw order.
    ///
    /// The objects keep their order among themselves: a step moves the
    /// block, so a title stays above the picture it belongs to.
    pub fn reorder_objects(&mut self, indices: &[usize], direction: Reorder) -> bool {
        let mut selected: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|i| *i < self.layers.len())
            .collect();
        if selected.is_empty() {
            return false;
        }
        selected.sort_unstable();
        selected.dedup();

        // The list is the draw order: the picture is drawn back to front and
        // the depths are renumbered after every move, so the list order and
        // the depth order are the same thing. `order` holds the layer that
        // sits at each depth.
        let mut order: Vec<usize> = (0..self.layers.len()).collect();

        let mut moved = false;
        match direction {
            Reorder::Forward | Reorder::Backward => {
                let step = matches!(direction, Reorder::Forward) as i64 * 2 - 1;
                // Walk the selected items from the front of the draw order
                // backwards for a step forward, and forwards for a step back,
                // so a block moves as one piece instead of collapsing.
                let mut positions: Vec<usize> = selected
                    .iter()
                    .filter_map(|s| order.iter().position(|o| o == s))
                    .collect();
                positions.sort_unstable();
                if step > 0 {
                    positions.reverse();
                }
                for position in positions {
                    let neighbour = position as i64 + step;
                    if neighbour < 0 || neighbour >= order.len() as i64 {
                        continue;
                    }
                    let neighbour = neighbour as usize;
                    // The block keeps its order: an item only moves past an
                    // item that is not part of the selection.
                    if selected.contains(&order[neighbour]) {
                        continue;
                    }
                    order.swap(position, neighbour);
                    moved = true;
                }
            }
            Reorder::ToFront => {
                let block: Vec<usize> = selected.iter().map(|s| order[*s]).collect();
                let rest: Vec<usize> = order
                    .iter()
                    .copied()
                    .filter(|i| !selected.contains(i))
                    .collect();
                order = rest.into_iter().chain(block).collect();
                moved = true;
            }
            Reorder::ToBack => {
                let block: Vec<usize> = selected.iter().map(|s| order[*s]).collect();
                let rest: Vec<usize> = order
                    .iter()
                    .copied()
                    .filter(|i| !selected.contains(i))
                    .collect();
                order = block.into_iter().chain(rest).collect();
                moved = true;
            }
        }

        if !moved {
            return false;
        }
        let mut layers: Vec<Option<Object>> = self.layers.drain(..).map(Some).collect();
        for (depth, source) in order.into_iter().enumerate() {
            if let Some(object) = layers[source].take() {
                let mut object = object;
                object.z_order = depth as i32;
                self.layers.push(object);
            }
        }
        self.renumber_z_order();
        true
    }

    /// Mirror the selected objects.
    ///
    /// A text object is not mirrored: a title that reads backwards is a
    /// mistake the operator cannot undo by turning the mirror off, so the
    /// text is left alone and the command reports that it did nothing.
    pub fn flip_objects(&mut self, indices: &[usize], horizontal: bool) -> bool {
        let mut flipped = false;
        for index in indices {
            let Some(object) = self.layers.get_mut(*index) else {
                continue;
            };
            if object.locked || matches!(object.content, ObjectContent::Text { .. }) {
                continue;
            }
            if horizontal {
                object.flip_horizontal = !object.flip_horizontal;
            } else {
                object.flip_vertical = !object.flip_vertical;
            }
            flipped = true;
        }
        flipped
    }

    /// Lock or unlock the selected objects.
    pub fn set_objects_locked(&mut self, indices: &[usize], locked: bool) -> bool {
        let mut changed = false;
        for index in indices {
            if let Some(object) = self.layers.get_mut(*index)
                && object.locked != locked
            {
                object.locked = locked;
                changed = true;
            }
        }
        changed
    }

    /// Unlock every object on the slide.
    pub fn unlock_all(&mut self) -> bool {
        let mut changed = false;
        for object in self.layers.iter_mut() {
            if object.locked {
                object.locked = false;
                changed = true;
            }
        }
        changed
    }
}

/// How a set of objects is lined up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMode {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

/// Which way through the draw order the objects move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reorder {
    Forward,
    Backward,
    ToFront,
    ToBack,
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
    /// How many items the plan has.
    ///
    /// The list of plans carries the count rather than the items, so the
    /// sidebar can say how long a plan is without loading every item of
    /// every plan.
    #[serde(default)]
    pub item_count: usize,
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
            item_count: 0,
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

    /// A slide with three objects at known places.
    fn slide_with_three_objects() -> Slide {
        let mut slide = Slide::new_text(String::from("Slide"));
        let mut first = Object::new_text(String::from("First"));
        first.position_x = 0.2;
        first.position_y = 0.2;
        first.width = 0.2;
        first.height = 0.2;
        let mut second = Object::new_text(String::from("Second"));
        second.position_x = 0.5;
        second.position_y = 0.5;
        second.width = 0.2;
        second.height = 0.2;
        let mut third = Object::new_text(String::from("Third"));
        third.position_x = 0.7;
        third.position_y = 0.7;
        third.width = 0.2;
        third.height = 0.2;
        slide.layers.push(first);
        slide.layers.push(second);
        slide.layers.push(third);
        slide
    }

    /// One object is lined up with the slide.
    ///
    /// The operator means "centre it" when there is only the one object, and
    /// the slide is what the object is centred on.
    #[test]
    fn one_object_is_aligned_with_the_slide() {
        let mut slide = slide_with_three_objects();
        assert!(slide.align_objects(&[1], AlignMode::Center));
        // "Center" is across the slide and "Middle" is down it, so centring
        // leaves the object where it was vertically.
        assert_eq!(slide.layers[1].position_x, 0.4);
        assert_eq!(slide.layers[1].position_y, 0.5);
        assert!(slide.align_objects(&[1], AlignMode::Middle));
        assert_eq!(slide.layers[1].position_y, 0.4);
    }

    /// Several objects are lined up with each other.
    ///
    /// The slide is not what the operator is looking at when a group is
    /// picked: the group is.
    #[test]
    fn several_objects_are_aligned_with_each_other() {
        let mut slide = slide_with_three_objects();
        assert!(slide.align_objects(&[0, 2], AlignMode::Left));
        // The frame of the group runs from 0.2 to 0.9, so both objects are
        // lined up with its left edge.
        assert_eq!(slide.layers[0].position_x, 0.2);
        assert_eq!(slide.layers[2].position_x, 0.2);
        // The one that did not move is left alone.
        assert_eq!(slide.layers[1].position_x, 0.5);
    }

    /// A locked object is not moved.
    ///
    /// A lock the operator set is a promise, so the arrangement commands
    /// keep it.
    #[test]
    fn a_locked_object_is_not_moved() {
        let mut slide = slide_with_three_objects();
        slide.layers[0].locked = true;
        slide.align_objects(&[0], AlignMode::Right);
        assert_eq!(slide.layers[0].position_x, 0.2);
    }

    /// Bringing an object forward keeps the objects below it below it.
    #[test]
    fn bringing_forward_keeps_the_order_of_the_rest() {
        let mut slide = slide_with_three_objects();
        assert!(slide.reorder_objects(&[0], Reorder::Forward));
        assert_eq!(slide.layers[0].content_text(), Some("Second"));
        assert_eq!(slide.layers[1].content_text(), Some("First"));
        assert_eq!(slide.layers[2].content_text(), Some("Third"));
    }

    /// A group moves as one piece.
    ///
    /// A title above its picture stays above it: a step that moved the
    /// title past the picture would change what the slide says.
    #[test]
    fn a_group_moves_as_one_piece() {
        let mut slide = slide_with_three_objects();
        assert!(slide.reorder_objects(&[0, 1], Reorder::Forward));
        assert_eq!(slide.layers[0].content_text(), Some("Third"));
        assert_eq!(slide.layers[1].content_text(), Some("First"));
        assert_eq!(slide.layers[2].content_text(), Some("Second"));
    }

    /// The draw order and the list order are the same thing.
    ///
    /// The list is drawn back to front, so a depth that says otherwise would
    /// draw the slide in an order nobody can see.
    #[test]
    fn the_draw_order_follows_the_list() {
        let mut slide = slide_with_three_objects();
        slide.reorder_objects(&[2], Reorder::ToFront);
        for (index, object) in slide.layers.iter().enumerate() {
            assert_eq!(object.z_order, index as i32);
        }
        assert_eq!(slide.layers[2].content_text(), Some("Third"));
    }

    /// A text object is not mirrored.
    ///
    /// A title that reads backwards is a mistake the operator cannot undo by
    /// turning the mirror off.
    #[test]
    fn a_text_object_is_not_mirrored() {
        let mut slide = slide_with_three_objects();
        assert!(!slide.flip_objects(&[1], true));
        assert!(!slide.layers[1].flip_horizontal);
    }

    /// A picture is mirrored.
    #[test]
    fn a_picture_is_mirrored() {
        let mut slide = slide_with_three_objects();
        slide.layers[1].content = ObjectContent::Image {
            path: String::from("/tmp/a.png"),
            fit: ImageFit::Fill,
        };
        assert!(slide.flip_objects(&[1], true));
        assert!(slide.layers[1].flip_horizontal);
        assert!(slide.flip_objects(&[1], true));
        assert!(!slide.layers[1].flip_horizontal);
    }

    /// Unlocking lets every object be moved again.
    #[test]
    fn unlocking_lets_every_object_be_moved() {
        let mut slide = slide_with_three_objects();
        slide.set_objects_locked(&[0, 1], true);
        assert!(slide.layers[0].locked && slide.layers[1].locked);
        assert!(slide.unlock_all());
        assert!(slide.layers.iter().all(|object| !object.locked));
    }

    /// The font size is clamped the same way everywhere.
    ///
    /// The Text tab and the Layers tab both edit the size, so a size that
    /// one panel accepts and the other silently changes is a value the
    /// operator cannot reason about.
    #[test]
    fn the_font_size_is_clamped_the_same_way_everywhere() {
        assert_eq!(TextStyle::clamp_font_size(0.0), TextStyle::MIN_FONT_SIZE);
        assert_eq!(TextStyle::clamp_font_size(72.0), 72.0);
        assert_eq!(
            TextStyle::clamp_font_size(10_000.0),
            TextStyle::MAX_FONT_SIZE
        );
        assert_eq!(TextStyle::MIN_FONT_SIZE, 4.0);
        assert_eq!(TextStyle::MAX_FONT_SIZE, 400.0);
    }

    /// A nudge moves the object by a step on the slide, not on the screen.
    ///
    /// One pixel on a 4K projector has to be one pixel on a 720p screen too,
    /// so the step is measured against the resolution the slide was authored
    /// at.
    #[test]
    fn a_nudge_is_a_step_on_the_slide() {
        let mut object = Object::new_text("Hello".to_string());
        object.position_x = 0.5;
        object.position_y = 0.5;

        assert!(object.nudge(10.0, 0.0, 1920.0, 1080.0));
        assert!(
            (object.position_x - 0.5 - 10.0 / 1920.0).abs() < 1e-6,
            "the object moved by ten slide pixels (got {})",
            object.position_x
        );
        assert!(
            (object.position_y - 0.5).abs() < 1e-6,
            "it moved sideways only"
        );

        // The vertical step is measured against the height, so the up and
        // down keys move as far as the left and right ones.
        let mut tall = Object::new_text("Hello".to_string());
        tall.position_y = 0.5;
        assert!(tall.nudge(0.0, 10.0, 1920.0, 1080.0));
        assert!(
            (tall.position_y - 0.5 - 10.0 / 1080.0).abs() < 1e-6,
            "the vertical step is a step on the height (got {})",
            tall.position_y
        );

        // The same step on a wider slide is a smaller share of it.
        let mut wide = Object::new_text("Hello".to_string());
        wide.position_x = 0.5;
        assert!(wide.nudge(10.0, 0.0, 3840.0, 2160.0));
        assert!(
            (wide.position_x - 0.5 - 10.0 / 3840.0).abs() < 1e-6,
            "a 4K slide moves half as far"
        );
    }

    /// A locked object does not move, and an object never leaves the slide.
    #[test]
    fn a_nudge_respects_the_lock_and_the_slide() {
        let mut locked = Object::new_text("Hello".to_string());
        locked.locked = true;
        let before = locked.position_x;
        assert!(!locked.nudge(10.0, 0.0, 1920.0, 1080.0));
        assert_eq!(locked.position_x, before, "a locked object stays put");

        let mut edge = Object::new_text("Hello".to_string());
        edge.position_x = 0.999;
        assert!(edge.nudge(100.0, 0.0, 1920.0, 1080.0));
        assert_eq!(edge.position_x, 1.0, "the object stops at the edge");

        let mut corner = Object::new_text("Hello".to_string());
        corner.position_y = 0.0;
        assert!(corner.nudge(0.0, -100.0, 1920.0, 1080.0));
        assert_eq!(corner.position_y, 0.0, "the object stops at the edge");
    }
    /// Deleting the last object of a slide leaves the slide empty.
    ///
    /// The slide's legacy content would otherwise synthesise the object
    /// again, and the object the operator deleted would come straight back.
    #[test]
    fn deleting_the_last_object_leaves_the_slide_empty() {
        use crate::domain::ObjectContent;

        let mut slide = Slide::new_text("Hello".to_string());
        slide.migrate_to_objects();
        assert_eq!(slide.layers.len(), 1, "the legacy text became an object");

        // The operator deletes it, and the legacy content is cleared with it.
        slide.layers.clear();
        slide.content = SlideContent::Text {
            text: String::new(),
            style: TextStyle::default(),
        };

        // Nothing carries the deleted text any more, so the slide shows
        // nothing.
        let layers = slide.effective_layers();
        assert!(
            layers.iter().all(|object| !matches!(
                &object.content,
                ObjectContent::Text { text, .. } if !text.is_empty()
            )),
            "the deleted object must not come back"
        );
    }

    /// A new object goes on top of the slide's others.
    ///
    /// The slide is drawn back to front, so an object with the default depth
    /// would be hidden behind the ones that are already there — the
    /// operator would add a picture and see nothing.
    #[test]
    fn a_new_object_goes_on_top() {
        use crate::domain::Object;

        let mut slide = Slide::new_text(String::new());
        slide.layers.push(Object::new_shape(ShapeType::Rectangle));

        let index = slide.add_object_on_top(Object::new_shape(ShapeType::Ellipse));
        assert_eq!(index, 1);
        assert!(
            slide.layers[1].z_order > slide.layers[0].z_order,
            "the new object is drawn in front of the old one"
        );
        assert_eq!(slide.top_z_order(), slide.layers[1].z_order);
    }

    /// The picture is drawn back to front and the list is what the operator
    /// reorders, so the two have to agree. A slide whose depths are not
    /// consecutive still draws in the right order, and renumbering makes
    /// them consecutive again.
    #[test]
    fn the_draw_order_follows_the_list_order() {
        use crate::domain::Object;

        let mut slide = Slide::new_text("Slide".to_string());
        let mut back = Object::new_shape(ShapeType::Rectangle);
        back.z_order = 0;
        let mut middle = Object::new_text("middle".to_string());
        middle.z_order = 7;
        let mut front = Object::new_shape(ShapeType::Ellipse);
        front.z_order = 3;
        slide.layers = vec![back, middle, front];

        // The depths are not consecutive, but the order is still back to
        // front: the compositor sorts by the depth.
        let mut sorted = slide.layers.clone();
        sorted.sort_by_key(|layer| layer.z_order);
        assert_eq!(sorted[0].z_order, 0);
        assert_eq!(sorted[2].z_order, 7);

        // Renumbering makes the depths follow the list.
        slide.renumber_z_order();
        let depths: Vec<i32> = slide.layers.iter().map(|layer| layer.z_order).collect();
        assert_eq!(depths, vec![0, 1, 2]);

        // A new object goes on top, so the operator sees it.
        assert_eq!(slide.top_z_order(), 2);
    }

    #[test]
    fn transition_from_config_str_roundtrips_settings_names() {
        // The strings are the ones written by the output settings tab.
        assert_eq!(
            Transition::from_config_str("fade", 400),
            Transition::Fade { duration_ms: 400 }
        );
        assert_eq!(Transition::from_config_str("cut", 400), Transition::Cut);
        assert_eq!(
            Transition::from_config_str("push_1", 400),
            Transition::Push {
                duration_ms: 400,
                direction: 1
            }
        );
        assert_eq!(
            Transition::from_config_str("wipe_180", 400),
            Transition::Wipe {
                duration_ms: 400,
                angle_deg: 180
            }
        );
        assert_eq!(
            Transition::from_config_str("zoom", 250),
            Transition::Zoom { duration_ms: 250 }
        );
    }

    #[test]
    fn transition_from_config_str_unknown_falls_back_to_fade() {
        assert_eq!(
            Transition::from_config_str("sparkle", 500),
            Transition::Fade { duration_ms: 500 }
        );
    }

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
    fn text_transform_start_case_starts_every_word() {
        assert_eq!(
            TextTransform::StartCase.apply("amazing grace how sweet"),
            "Amazing Grace How Sweet"
        );
    }

    #[test]
    fn text_transform_start_case_empty_string() {
        assert_eq!(TextTransform::StartCase.apply(""), "");
    }

    #[test]
    fn text_transform_start_case_single_word() {
        assert_eq!(TextTransform::StartCase.apply("grace"), "Grace");
    }

    /// The title case keeps the small words small: a title is read as a
    /// phrase, not as a row of capital letters.
    #[test]
    fn text_transform_title_case_leaves_the_small_words_alone() {
        assert_eq!(
            TextTransform::TitleCase.apply("the lord is my shepherd"),
            "The Lord Is My Shepherd"
        );
        assert_eq!(
            TextTransform::TitleCase.apply("a song of the sea"),
            "A Song of the Sea"
        );
    }

    #[test]
    fn text_transform_small_caps_is_upper_case() {
        assert_eq!(TextTransform::SmallCaps.apply("grace"), "GRACE");
        assert!(TextTransform::SmallCaps.is_small_caps());
        assert!(!TextTransform::Uppercase.is_small_caps());
    }

    /// The transformations rewrite the words themselves, and the words are
    /// what the audience reads.
    #[test]
    fn a_text_rewrite_changes_the_words() {
        assert_eq!(TextRewrite::UpperCase.apply("grace"), "GRACE");
        assert_eq!(TextRewrite::LowerCase.apply("GRACE"), "grace");
        assert_eq!(TextRewrite::Capitalize.apply("the lord"), "The Lord");
        // The rest of the word is left as it was: an operator who
        // capitalises "well-known" expects "Well-Known".
        assert_eq!(TextRewrite::Capitalize.apply("well-known"), "Well-Known");
    }

    /// The rewrite reaches the text the operator is editing, through the
    /// same rule every other text control uses.
    #[test]
    fn a_text_rewrite_reaches_the_text_being_edited() {
        let mut slide = Slide::new_text(String::from("grace"));
        assert!(slide.rewrite_editable_text(None, TextRewrite::UpperCase));
        assert_eq!(
            slide.editable_text(None).map(|(text, _)| text.as_str()),
            Some("GRACE")
        );

        // A slide with an object on it: the selected object wins.
        let mut slide = Slide::new_text(String::from("the slide"));
        slide.layers.push(Object::new_text(String::from("grace")));
        assert!(slide.rewrite_editable_text(Some(0), TextRewrite::UpperCase));
        assert_eq!(
            slide.editable_text(Some(0)).map(|(text, _)| text.as_str()),
            Some("GRACE")
        );
        // The slide's own content was not the text being edited.
        assert_eq!(
            slide.editable_text(None).map(|(text, _)| text.as_str()),
            Some("GRACE")
        );
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

    #[test]
    fn slide_cues_add_and_retain_action() {
        let mut s = Slide::new_text("Amazing Grace".to_string());
        assert!(s.cues.is_empty());
        s.cues.push(Cue::new("Clear Slide", Action::ClearSlide));
        s.cues.push(Cue::new("Start Timer", Action::StartTimer));
        assert_eq!(s.cues.len(), 2);
        assert!(matches!(s.cues[0].action, Action::ClearSlide));
        assert!(matches!(s.cues[1].action, Action::StartTimer));
        assert!(s.cues[0].enabled);
    }

    #[test]
    fn song_verses_convert_to_presentation_slides() {
        let mut song = Song::new("How Great Thou Art".to_string());
        song.verses.push(Verse::new(
            "Verse 1".to_string(),
            "O Lord my God...".to_string(),
            0,
        ));
        song.verses.push(Verse::new(
            "Chorus".to_string(),
            "Then sings my soul...".to_string(),
            1,
        ));

        let slides: Vec<Slide> = song
            .verses
            .iter()
            .map(|v| Slide::new_text_in_group(v.content.clone(), v.label.clone()))
            .collect();

        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].group.as_deref(), Some("Verse 1"));
        assert_eq!(slides[1].group.as_deref(), Some("Chorus"));
    }

    #[test]
    fn slide_effective_layers_preserves_text_and_position() {
        let slide = Slide::new_text("Hello OpenPresenter".to_string());
        let layers = slide.effective_layers();
        assert_eq!(layers.len(), 1);
        if let ObjectContent::Text { text, .. } = &layers[0].content {
            assert_eq!(text, "Hello OpenPresenter");
        } else {
            panic!("Expected text layer content");
        }
    }

    #[test]
    fn a_slide_reports_its_media_wherever_it_lives() {
        // A video object.
        let mut slide = Slide::new_text(String::new());
        slide.layers = vec![Object::new_video("clip.mp4", None)];
        assert_eq!(slide.video_path().as_deref(), Some("clip.mp4"));

        // A video in the slide's own content (older slides).
        let mut slide = Slide::new_text(String::new());
        slide.content = SlideContent::Video {
            path: String::from("legacy.mp4"),
            thumbnail: None,
        };
        assert_eq!(slide.video_path().as_deref(), Some("legacy.mp4"));

        // A video as the slide's background.
        let mut slide = Slide::new_text(String::new());
        slide.background = Background::Video(String::from("background.mp4"));
        assert_eq!(slide.video_path().as_deref(), Some("background.mp4"));

        // A slide without a video reports none.
        let slide = Slide::new_text(String::from("Just text"));
        assert!(slide.video_path().is_none());
    }

    #[test]
    fn a_slide_reports_its_image_and_fit() {
        let mut slide = Slide::new_text(String::new());
        slide.layers = vec![Object::new_image("photo.png", ImageFit::Stretch)];
        assert_eq!(
            slide.image(),
            Some((String::from("photo.png"), ImageFit::Stretch))
        );

        // An image in the slide's own content (older slides).
        let mut slide = Slide::new_text(String::new());
        slide.content = SlideContent::Image {
            path: String::from("legacy.png"),
            fit: ImageFit::Center,
        };
        assert_eq!(
            slide.image(),
            Some((String::from("legacy.png"), ImageFit::Center))
        );

        let slide = Slide::new_text(String::from("Just text"));
        assert!(slide.image().is_none());
    }
}
