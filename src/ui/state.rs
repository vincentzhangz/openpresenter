use crate::db::{BibleRepository, ServiceRepository, SongRepository};
use crate::media::AudioPlayer;
use crate::slides::{BibleTranslation, BibleVerse, ServicePlan, Song};

pub(crate) struct BibleState {
    pub(crate) repo: BibleRepository,
    pub(crate) translations: Vec<BibleTranslation>,
    pub(crate) selected_translation: Option<String>,
    pub(crate) books: Vec<String>,
    pub(crate) selected_book: Option<String>,
    pub(crate) chapters: Vec<i32>,
    pub(crate) selected_chapter: Option<i32>,
    pub(crate) verses: Vec<BibleVerse>,
    pub(crate) selected_verse_indices: Vec<usize>,
    pub(crate) search: String,
    pub(crate) verses_per_slide: usize,
    pub(crate) translation_to_delete: Option<String>,
}

impl BibleState {
    pub(crate) fn new(repo: BibleRepository) -> Self {
        Self {
            repo,
            translations: Vec::new(),
            selected_translation: None,
            books: Vec::new(),
            selected_book: None,
            chapters: Vec::new(),
            selected_chapter: None,
            verses: Vec::new(),
            selected_verse_indices: Vec::new(),
            search: String::new(),
            verses_per_slide: 2,
            translation_to_delete: None,
        }
    }
}

pub(crate) struct SongState {
    pub(crate) repo: SongRepository,
    pub(crate) songs: Vec<Song>,
    pub(crate) search: String,
    pub(crate) editing: Option<Song>,
    pub(crate) to_delete: Option<String>,
    pub(crate) edit_title: String,
    pub(crate) edit_artist: String,
    pub(crate) edit_copyright: String,
    pub(crate) edit_ccli: String,
    pub(crate) edit_key: String,
    pub(crate) edit_bpm: String,
}

impl SongState {
    pub(crate) fn new(repo: SongRepository) -> Self {
        Self {
            repo,
            songs: Vec::new(),
            search: String::new(),
            editing: None,
            to_delete: None,
            edit_title: String::new(),
            edit_artist: String::new(),
            edit_copyright: String::new(),
            edit_ccli: String::new(),
            edit_key: String::new(),
            edit_bpm: String::new(),
        }
    }
}

pub(crate) struct LayerState {
    pub(crate) selected_index: Option<usize>,
    pub(crate) text: String,
    pub(crate) font_size: String,
    pub(crate) pos_x: String,
    pub(crate) pos_y: String,
    pub(crate) width: String,
    pub(crate) height: String,
    pub(crate) stroke_width: String,
    pub(crate) font_family: String,
    pub(crate) line_height: String,
    pub(crate) letter_spacing: String,
    pub(crate) glow_radius: String,
    pub(crate) text_stroke_width: String,
}

impl Default for LayerState {
    fn default() -> Self {
        Self {
            selected_index: None,
            text: String::new(),
            font_size: String::from("72"),
            pos_x: String::from("0.5"),
            pos_y: String::from("0.5"),
            width: String::from("0.9"),
            height: String::from("0.6"),
            stroke_width: String::from("0"),
            font_family: String::from("Arial"),
            line_height: String::from("1.2"),
            letter_spacing: String::from("0"),
            glow_radius: String::from("8"),
            text_stroke_width: String::from("0"),
        }
    }
}

pub(crate) struct AudioState {
    pub(crate) player: Option<AudioPlayer>,
    pub(crate) path: String,
    pub(crate) track: Option<String>,
    pub(crate) playing: bool,
    pub(crate) volume: f32,
    pub(crate) looping: bool,
    pub(crate) panel_visible: bool,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            player: None,
            path: String::new(),
            track: None,
            playing: false,
            volume: 0.8,
            looping: false,
            panel_visible: false,
        }
    }
}

pub(crate) struct ServiceState {
    pub(crate) repo: ServiceRepository,
    pub(crate) plans: Vec<ServicePlan>,
    pub(crate) editing: Option<ServicePlan>,
    pub(crate) to_delete: Option<String>,
    pub(crate) name_edit: String,
    pub(crate) active: Option<ServicePlan>,
    pub(crate) item_index: usize,
}

impl ServiceState {
    pub(crate) fn new(repo: ServiceRepository) -> Self {
        Self {
            repo,
            plans: Vec::new(),
            editing: None,
            to_delete: None,
            name_edit: String::new(),
            active: None,
            item_index: 0,
        }
    }
}
