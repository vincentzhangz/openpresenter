use crate::db::{BibleRepository, LibraryRepository, SongRepository, ThemeRepository};
use crate::domain::{
    BibleTranslation, BibleVerse, LibraryAsset, Playlist, Presentation, PropManager, SlideTheme,
    Song,
};
use crate::media::AudioPlayer;
use crate::media::MediaPlayer;
use crate::ndi::NdiOutputLoop;
use crate::output::OutputManager;
use crate::recording::RecordingManager;
use crate::triggers::TriggerManager;
use crate::ui::messages::{
    InspectorTab, RailContextTarget, RightDockTab, SettingsTab, SidebarTab, ViewMode,
};
use crate::ui::presenter::TransitionState;

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
    pub(crate) plans: Vec<Playlist>,
    pub(crate) editing: Option<Playlist>,
    pub(crate) to_delete: Option<String>,
    pub(crate) name_edit: String,
    pub(crate) active: Option<Playlist>,
    pub(crate) item_index: usize,
}

impl ServiceState {
    pub(crate) fn new() -> Self {
        Self {
            plans: Vec::new(),
            editing: None,
            to_delete: None,
            name_edit: String::new(),
            active: None,
            item_index: 0,
        }
    }
}

/// Props, live alerts, and presentation overlay state.
#[derive(Default)]
pub(crate) struct PropsState {
    pub(crate) manager: PropManager,
    pub(crate) panel_open: bool,
    pub(crate) new_prop_name: String,
    pub(crate) new_look_name: String,
    pub(crate) lower_third_title: String,
    pub(crate) lower_third_subtitle: String,
}

/// Theme library and slide-theme styling state.
pub(crate) struct ThemesState {
    pub(crate) list: Vec<SlideTheme>,
    pub(crate) selected_theme_id: Option<String>,
    pub(crate) new_theme_name: String,
    pub(crate) repo: ThemeRepository,
}

impl ThemesState {
    pub(crate) fn new(repo: ThemeRepository) -> Self {
        Self {
            list: Vec::new(),
            selected_theme_id: None,
            new_theme_name: String::new(),
            repo,
        }
    }
}

/// Trigger and automation macro state.
pub(crate) struct TriggersState {
    pub(crate) manager: TriggerManager,
    pub(crate) rx:
        std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::triggers::Action>>>,
    pub(crate) panel_open: bool,
    pub(crate) http_port_str: String,
    pub(crate) osc_port_str: String,
    pub(crate) new_macro_name: String,
    pub(crate) macro_running_handles:
        std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
    pub(crate) macro_running_ids: std::collections::HashSet<String>,
}

/// H.264 video recording manager state.
pub(crate) struct RecordingFeatureState {
    pub(crate) manager: RecordingManager,
    pub(crate) panel_open: bool,
}

impl Default for RecordingFeatureState {
    fn default() -> Self {
        Self {
            manager: RecordingManager::new(),
            panel_open: false,
        }
    }
}

/// Library media asset state.
pub(crate) struct LibraryState {
    pub(crate) assets: Vec<LibraryAsset>,
    pub(crate) selected_id: Option<String>,
    pub(crate) recently_used_ids: Vec<String>,
    pub(crate) repo: LibraryRepository,
}

impl LibraryState {
    pub(crate) fn new(repo: LibraryRepository) -> Self {
        Self {
            assets: Vec::new(),
            selected_id: None,
            recently_used_ids: Vec::new(),
            repo,
        }
    }
}

/// Video and media-player playback state.
pub(crate) struct VideoState {
    pub(crate) player: Option<MediaPlayer>,
    pub(crate) frame: Option<iced::widget::image::Handle>,
    pub(crate) position: f64,
    pub(crate) looping: bool,
    pub(crate) volume: f32,
    pub(crate) muted: bool,
    pub(crate) speed: f64,
}

impl Default for VideoState {
    fn default() -> Self {
        Self {
            player: None,
            frame: None,
            position: 0.0,
            looping: true,
            volume: 1.0,
            muted: false,
            speed: 1.0,
        }
    }
}

/// Multi-screen output, matrix looks, and NDI broadcast state.
pub(crate) struct OutputState {
    pub(crate) manager: OutputManager,
    pub(crate) settings_open: bool,
    pub(crate) new_label: String,
    pub(crate) new_ndi_name: String,
    pub(crate) window_id: Option<iced::window::Id>,
    pub(crate) screen_x: String,
    pub(crate) screen_y: String,
    pub(crate) show_settings: bool,
    pub(crate) black_screen: bool,
    pub(crate) is_fullscreen: bool,
    pub(crate) matrix_open: bool,
    pub(crate) selected_look_id: Option<String>,
    pub(crate) editing_look_name: String,
    pub(crate) message_input: String,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) settings_status_message: Option<String>,
}

impl OutputState {
    pub(crate) fn new(app_config: &crate::config::Config) -> Self {
        Self {
            manager: OutputManager::with_defaults(),
            settings_open: false,
            new_label: String::new(),
            new_ndi_name: String::new(),
            window_id: None,
            screen_x: app_config.output.screen_x.to_string(),
            screen_y: app_config.output.screen_y.to_string(),
            show_settings: false,
            black_screen: false,
            is_fullscreen: false,
            matrix_open: false,
            selected_look_id: None,
            editing_look_name: String::new(),
            message_input: String::new(),
            settings_tab: SettingsTab::General,
            settings_status_message: None,
        }
    }
}

/// Live presentation, stage display, and transition state.
pub(crate) struct PresentingState {
    pub(crate) presentation: Option<Presentation>,
    pub(crate) slide_index: usize,
    pub(crate) transition: Option<TransitionState>,
    pub(crate) stage_display_active: bool,
    pub(crate) slide_context_index: Option<usize>,
    pub(crate) slide_context_pos: Option<iced::Point>,
    pub(crate) group_submenu: bool,
    pub(crate) cue_submenu: bool,
    pub(crate) transition_submenu: bool,
    pub(crate) slide_grid_cols: usize,
    pub(crate) slide_view_mode: crate::ui::messages::SlideViewMode,
    pub(crate) global_transition: crate::domain::Transition,
    pub(crate) clock_secs: u64,
    pub(crate) timer_secs: u64,
    pub(crate) timer_running: bool,
    pub(crate) timer_start_epoch: u64,
    pub(crate) ndi_output: Option<NdiOutputLoop>,

    // Decoupled multi-layer engine state:
    pub(crate) slide_layer_active: bool,
    pub(crate) media_layer_active: bool,
    pub(crate) preview_screen_target: String,
}

impl Default for PresentingState {
    fn default() -> Self {
        Self {
            presentation: None,
            slide_index: 0,
            transition: None,
            stage_display_active: false,
            slide_context_index: None,
            slide_context_pos: None,
            group_submenu: false,
            cue_submenu: false,
            transition_submenu: false,
            slide_grid_cols: 4,
            slide_view_mode: crate::ui::messages::SlideViewMode::default(),
            global_transition: crate::domain::Transition::Fade { duration_ms: 500 },
            clock_secs: 0,
            timer_secs: 0,
            timer_running: false,
            timer_start_epoch: 0,
            ndi_output: None,
            slide_layer_active: true,
            media_layer_active: true,
            preview_screen_target: String::from("main"),
        }
    }
}

/// Presentation editing, canvas, and undo-redo state.
pub(crate) struct EditorState {
    pub(crate) presentations: Vec<Presentation>,
    pub(crate) editing: Option<Presentation>,
    pub(crate) editing_name: String,
    pub(crate) selected_slide_index: Option<usize>,
    pub(crate) editing_slide_text: String,
    pub(crate) editing_slide_font_size: String,
    pub(crate) editing_transition_duration: String,
    pub(crate) editing_group_label: String,
    pub(crate) editing_slide_notes: String,
    pub(crate) new_presentation_window_id: Option<iced::window::Id>,
    pub(crate) new_presentation_name: String,
    pub(crate) show_delete_confirmation: bool,
    pub(crate) delete_target_id: Option<String>,
    pub(crate) delete_confirm_window_id: Option<iced::window::Id>,
    pub(crate) undo_stack: Vec<Presentation>,
    pub(crate) redo_stack: Vec<Presentation>,
    pub(crate) copied_style: Option<crate::domain::TextStyle>,
    pub(crate) inline_editing: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            presentations: Vec::new(),
            editing: None,
            editing_name: String::new(),
            selected_slide_index: None,
            editing_slide_text: String::new(),
            editing_slide_font_size: String::from("72"),
            editing_transition_duration: String::from("500"),
            editing_group_label: String::new(),
            editing_slide_notes: String::new(),
            new_presentation_window_id: None,
            new_presentation_name: String::new(),
            show_delete_confirmation: false,
            delete_target_id: None,
            delete_confirm_window_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            copied_style: None,
            inline_editing: false,
        }
    }
}

/// Shell layout, navigation rails, and modal view state.
pub(crate) struct ShellState {
    pub(crate) current_mode: ViewMode,
    pub(crate) inspector_tab: InspectorTab,
    pub(crate) search_query: String,
    pub(crate) sidebar_tab: SidebarTab,
    pub(crate) right_dock_tab: RightDockTab,
    pub(crate) media_bin_open: bool,
    pub(crate) rail_context_target: Option<RailContextTarget>,
    pub(crate) rail_cursor_pos: Option<iced::Point>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            current_mode: ViewMode::Show,
            inspector_tab: InspectorTab::Text,
            search_query: String::new(),
            sidebar_tab: SidebarTab::default(),
            right_dock_tab: RightDockTab::default(),
            media_bin_open: true,
            rail_context_target: None,
            rail_cursor_pos: None,
        }
    }
}

/// App-wide UI state (shortcuts window, reduced motion, error toast).
#[derive(Default)]
pub(crate) struct UiState {
    pub(crate) shortcuts_window_id: Option<iced::window::Id>,
    pub(crate) reduce_motion: bool,
    pub(crate) error_message: Option<String>,
}

/// Package (.opp) and lyrics import/export panel state.
#[derive(Default)]
pub(crate) struct ImportExportState {
    pub(crate) panel_open: bool,
}
