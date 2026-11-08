//! Per-feature state held by `MainWindow`.
//!
//! `MainWindow` owns one struct per feature rather than a flat list of
//! fields, so a feature's state stays together and a new feature adds a
//! struct instead of expanding the window.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use gpui_kit::component::input::InputState;
use gpui_kit::{AnyWindowHandle, Entity, Point, RenderImage, component::slider::SliderState};

use crate::db::{BibleRepository, LibraryRepository, SongRepository, ThemeRepository};
use crate::domain::{
    BibleTranslation, BibleVerse, LibraryAsset, Playlist, Presentation, PropManager, SlideTheme,
    Song,
};
use crate::media::AudioPlayer;
use crate::media::MediaPlayer;
use crate::output::OutputManager;
use crate::recording::RecordingManager;
use crate::triggers::TriggerManager;
use crate::ui::canvas::DragState;
use crate::ui::messages::{
    InspectorTab, RailContextTarget, RightDockTab, SettingsTab, SidebarTab, ViewMode,
};
use crate::ui::stage_window::StageSlot;

/// Bible panel state.
pub(crate) struct BibleState {
    pub(crate) repo: BibleRepository,
    pub(crate) service: crate::services::BibleService,
    pub(crate) translations: Vec<BibleTranslation>,
    pub(crate) selected_translation: Option<String>,
    pub(crate) books: Vec<String>,
    pub(crate) selected_book: Option<String>,
    pub(crate) chapters: Vec<i32>,
    pub(crate) selected_chapter: Option<i32>,
    pub(crate) verses: Vec<BibleVerse>,
    pub(crate) selected_verse_indices: Vec<usize>,
    pub(crate) search: String,
    pub(crate) search_truncated: bool,
    pub(crate) verses_per_slide: usize,
    pub(crate) translation_to_delete: Option<String>,
}

impl BibleState {
    /// Create the state around a repository.
    pub(crate) fn new(repo: BibleRepository, service: crate::services::BibleService) -> Self {
        Self {
            repo,
            service,
            translations: Vec::new(),
            selected_translation: None,
            books: Vec::new(),
            selected_book: None,
            chapters: Vec::new(),
            selected_chapter: None,
            verses: Vec::new(),
            selected_verse_indices: Vec::new(),
            search: String::new(),
            search_truncated: false,
            verses_per_slide: 2,
            translation_to_delete: None,
        }
    }
}

/// One verse's label/content inputs.
pub(crate) struct VerseInputs {
    /// Label field.
    pub(crate) label: Entity<InputState>,
    /// Content field.
    pub(crate) content: Entity<InputState>,
}

/// Song panel state.
pub(crate) struct SongState {
    pub(crate) repo: SongRepository,
    pub(crate) service: crate::services::SongService,
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
    pub(crate) verse_inputs: Vec<VerseInputs>,
    pub(crate) library_songs: Vec<Song>,
}

impl SongState {
    /// Create the state around a repository.
    pub(crate) fn new(repo: SongRepository, service: crate::services::SongService) -> Self {
        Self {
            repo,
            service,
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
            verse_inputs: Vec::new(),
            library_songs: Vec::new(),
        }
    }
}

/// Layer inspector state: the strings the panel fields mirror.
pub(crate) struct LayerState {
    pub(crate) selected_index: Option<usize>,
    pub(crate) selected_indices: Vec<usize>,
    pub(crate) text: String,
    pub(crate) font_size: String,
    pub(crate) pos_x: String,
    pub(crate) pos_y: String,
    pub(crate) width: String,
    pub(crate) height: String,
    pub(crate) stroke_width: String,
    pub(crate) corner_radius: String,
    pub(crate) font_family: String,
    pub(crate) line_height: String,
    pub(crate) letter_spacing: String,
    pub(crate) glow_radius: String,
    pub(crate) text_stroke_width: String,
    pub(crate) text_hex_sync: bool,
}

impl Default for LayerState {
    fn default() -> Self {
        Self {
            selected_index: None,
            selected_indices: Vec::new(),
            text: String::new(),
            font_size: String::from("72"),
            pos_x: String::from("0.5"),
            pos_y: String::from("0.5"),
            width: String::from("0.9"),
            height: String::from("0.6"),
            stroke_width: String::from("0"),
            corner_radius: String::from("0"),
            font_family: String::from("Arial"),
            line_height: String::from("1.2"),
            letter_spacing: String::from("0"),
            glow_radius: String::from("8"),
            text_stroke_width: String::from("0"),
            text_hex_sync: false,
        }
    }
}

/// One track waiting in the audio queue.
#[derive(Debug, Clone)]
pub(crate) struct QueueTrack {
    /// File path of the track.
    pub(crate) path: String,
    /// Display name of the track.
    pub(crate) name: String,
}

/// Audio transport state.
pub(crate) struct AudioState {
    pub(crate) player: Option<AudioPlayer>,
    pub(crate) path: String,
    pub(crate) track: Option<String>,
    pub(crate) playing: bool,
    pub(crate) volume: f32,
    pub(crate) looping: bool,
    pub(crate) panel_visible: bool,
    /// Tracks waiting to play after the current one ends.
    pub(crate) queue: Vec<QueueTrack>,
    /// Index into `queue` of the `track` currently loaded.
    pub(crate) queue_index: Option<usize>,
    /// Playback-position snapshots for the transport readout.
    pub(crate) position_secs: u64,
    pub(crate) duration_secs: u64,
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
            queue: Vec::new(),
            queue_index: None,
            position_secs: 0,
            duration_secs: 0,
        }
    }
}

/// Playlist (service plan) panel state.
pub(crate) struct ServiceState {
    pub(crate) plans: Vec<Playlist>,
    pub(crate) editing: Option<Playlist>,
    pub(crate) to_delete: Option<String>,
    pub(crate) name_edit: String,
    pub(crate) active: Option<Playlist>,
    pub(crate) item_index: usize,
    /// Continuous mode: the service wraps to the first item at the end,
    /// the way a background loop does.
    pub(crate) continuous: bool,
    pub(crate) header_editing: bool,
    pub(crate) item_names: Vec<String>,
}

impl ServiceState {
    /// Create the state.
    pub(crate) fn new() -> Self {
        Self {
            plans: Vec::new(),
            editing: None,
            to_delete: None,
            name_edit: String::new(),
            active: None,
            item_index: 0,
            continuous: false,
            header_editing: false,
            item_names: Vec::new(),
        }
    }
}

/// Props, live alerts, and overlay state.
#[derive(Default)]
pub(crate) struct PropsState {
    pub(crate) manager: PropManager,
    pub(crate) panel_open: bool,
    pub(crate) new_prop_name: String,
    pub(crate) new_look_name: String,
    pub(crate) lower_third_title: String,
    pub(crate) lower_third_subtitle: String,
    pub(crate) editing_prop_id: Option<String>,
}

/// Theme library state.
pub(crate) struct ThemesState {
    pub(crate) list: Vec<SlideTheme>,
    pub(crate) selected_theme_id: Option<String>,
    pub(crate) new_theme_name: String,
    pub(crate) repo: ThemeRepository,
}

impl ThemesState {
    /// Create the state around a repository.
    pub(crate) fn new(repo: ThemeRepository) -> Self {
        Self {
            list: Vec::new(),
            selected_theme_id: None,
            new_theme_name: String::new(),
            repo,
        }
    }
}

/// Trigger and macro state.
pub(crate) struct TriggersState {
    pub(crate) manager: TriggerManager,
    pub(crate) rx: Arc<std::sync::Mutex<tokio::sync::mpsc::Receiver<crate::triggers::Action>>>,
    pub(crate) panel_open: bool,
    pub(crate) http_port_str: String,
    pub(crate) osc_port_str: String,
    pub(crate) midi_ports: Vec<crate::triggers::midi::MidiPortInfo>,
    pub(crate) selected_midi_port: Option<usize>,
    pub(crate) new_macro_name: String,
    pub(crate) macro_running_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    pub(crate) macro_running_ids: HashSet<String>,
}

/// Recording panel state.
pub(crate) struct RecordingFeatureState {
    pub(crate) manager: RecordingManager,
    pub(crate) panel_open: bool,
    pub(crate) confirm_stop_pending: bool,
}

impl Default for RecordingFeatureState {
    fn default() -> Self {
        Self {
            manager: RecordingManager::new(),
            panel_open: false,
            confirm_stop_pending: false,
        }
    }
}

/// Library asset state.
pub(crate) struct LibraryState {
    pub(crate) assets: Vec<LibraryAsset>,
    pub(crate) selected_id: Option<String>,
    pub(crate) recently_used_ids: Vec<String>,
    pub(crate) repo: LibraryRepository,
    pub(crate) to_delete: Option<String>,
    pub(crate) missing: Vec<String>,
    pub(crate) recent_ids: Vec<String>,
}

impl LibraryState {
    /// Create the state around a repository.
    pub(crate) fn new(repo: LibraryRepository) -> Self {
        Self {
            assets: Vec::new(),
            selected_id: None,
            recently_used_ids: Vec::new(),
            repo,
            to_delete: None,
            missing: Vec::new(),
            recent_ids: Vec::new(),
        }
    }
}

/// Video playback and decoded-frame state.
pub(crate) struct VideoState {
    pub(crate) player: Option<MediaPlayer>,
    pub(crate) frame: Option<Arc<RenderImage>>,
    pub(crate) frame_fingerprint: u64,
    pub(crate) frame_identity: u64,
    pub(crate) position: f64,
    pub(crate) looping: bool,
    pub(crate) volume: f32,
    pub(crate) muted: bool,
    pub(crate) speed: f64,
    pub(crate) pending_path: Option<String>,
    pub(crate) pending_play: bool,
    pub(crate) opening_generation: u64,
    pub(crate) opened: Option<Arc<RenderImage>>,
    pub(crate) seek_target: Option<f64>,
}

impl Default for VideoState {
    fn default() -> Self {
        Self {
            player: None,
            frame: None,
            frame_fingerprint: 0,
            frame_identity: 0,
            position: 0.0,
            looping: true,
            volume: 1.0,
            muted: false,
            speed: 1.0,
            pending_path: None,
            pending_play: false,
            opening_generation: 0,
            opened: None,
            seek_target: None,
        }
    }
}

/// Routes the output windows read.
#[derive(Default)]
pub(crate) struct RouteSlot {
    routes: std::sync::Mutex<HashMap<String, crate::output::OutputContentRoute>>,
}

impl RouteSlot {
    /// Publish the routes.
    pub(crate) fn publish(&self, routes: HashMap<String, crate::output::OutputContentRoute>) {
        if let Ok(mut guard) = self.routes.lock() {
            *guard = routes;
        }
    }

    /// Read the routes.
    pub(crate) fn read(
        &self,
    ) -> std::collections::HashMap<String, crate::output::OutputContentRoute> {
        self.routes.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// Output, matrix looks, and window state.
pub(crate) struct OutputState {
    pub(crate) manager: OutputManager,
    pub(crate) settings_open: bool,
    pub(crate) settings_window: Option<AnyWindowHandle>,
    pub(crate) new_label: String,
    pub(crate) new_ndi_name: String,
    pub(crate) window_id: Option<gpui_kit::WindowId>,
    pub(crate) windows: HashMap<String, AnyWindowHandle>,
    pub(crate) route_slot: Arc<RouteSlot>,
    pub(crate) screen_x: String,
    pub(crate) screen_y: String,
    pub(crate) placement_target: String,
    pub(crate) show_settings: bool,
    pub(crate) black_screen: bool,
    pub(crate) is_fullscreen: bool,
    pub(crate) matrix_open: bool,
    pub(crate) selected_look_id: Option<String>,
    pub(crate) editing_look_name: String,
    pub(crate) message_input: String,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) settings_status_message: Option<String>,
    pub(crate) detected_displays: Vec<crate::output::DetectedDisplay>,
}

impl OutputState {
    /// Create the state, seeding placement strings from config.
    pub(crate) fn new(app_config: &crate::config::Config) -> Self {
        Self {
            manager: OutputManager::with_defaults(),
            settings_open: false,
            settings_window: None,
            new_label: String::new(),
            new_ndi_name: String::new(),
            window_id: None,
            windows: HashMap::new(),
            route_slot: Arc::new(RouteSlot::default()),
            screen_x: app_config.output.screen_x.to_string(),
            screen_y: app_config.output.screen_y.to_string(),
            placement_target: String::new(),
            show_settings: false,
            black_screen: false,
            is_fullscreen: false,
            matrix_open: false,
            selected_look_id: None,
            editing_look_name: String::new(),
            message_input: String::new(),
            settings_tab: SettingsTab::General,
            settings_status_message: None,
            detected_displays: crate::output::detect_displays(),
        }
    }
}

/// Pending cue queue.
pub(crate) struct PendingCues {
    entries: Vec<PendingCue>,
}

/// One queued cue.
pub(crate) struct PendingCue {
    /// When the cue fires.
    pub(crate) at: std::time::Instant,
    /// What the cue does.
    pub(crate) action: crate::domain::Action,
}

impl PendingCues {
    /// Create an empty queue.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Clear the queue.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Replace the queue with fresh cues.
    pub fn replace(
        &mut self,
        cues: &[crate::domain::Cue],
        tx: &tokio::sync::mpsc::Sender<crate::domain::Action>,
    ) {
        let now = std::time::Instant::now();
        self.entries = cues
            .iter()
            .filter(|cue| cue.enabled)
            .map(|cue| {
                let _ = tx;
                PendingCue {
                    at: now + std::time::Duration::from_millis(cue.delay_ms),
                    action: cue.action.clone(),
                }
            })
            .collect();
    }
}

/// Live presentation state.
pub(crate) struct PresentingState {
    pub(crate) presentation: Option<Presentation>,
    pub(crate) slide_index: usize,
    pub(crate) transition: Option<crate::domain::Transition>,
    pub(crate) stage_display_active: bool,
    pub(crate) slide_context_index: Option<usize>,
    pub(crate) slide_context_pos: Option<Point<f32>>,
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
    pub(crate) countdown_end_epoch: Option<u64>,
    pub(crate) stage_window_id: Option<gpui_kit::WindowId>,
    pub(crate) ndi_outputs: HashMap<String, crate::ndi::NdiOutputLoop>,
    pub(crate) ndi_blacked: bool,
    pub(crate) stage_slot: Arc<StageSlot>,
    pub(crate) slide_layer_active: bool,
    pub(crate) media_layer_active: bool,
    pub(crate) preview_screen_target: String,
    pub(crate) pending_cues: PendingCues,
    pub(crate) timeline_playing: bool,
    pub(crate) timeline_elapsed_secs: u64,
    pub(crate) timeline_start_epoch: Option<u64>,
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
            countdown_end_epoch: None,
            stage_window_id: None,
            ndi_outputs: HashMap::new(),
            ndi_blacked: false,
            stage_slot: Arc::new(StageSlot::default()),
            slide_layer_active: true,
            media_layer_active: true,
            preview_screen_target: String::from("main"),
            pending_cues: PendingCues::new(),
            timeline_playing: false,
            timeline_elapsed_secs: 0,
            timeline_start_epoch: None,
        }
    }
}

/// Presentation editing and undo state.
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
    pub(crate) new_presentation_open: bool,
    pub(crate) show_delete_confirmation: bool,
    pub(crate) delete_target_id: Option<String>,
    pub(crate) undo_stack: Vec<Presentation>,
    pub(crate) redo_stack: Vec<Presentation>,
    pub(crate) undo_coalesce: Vec<Presentation>,
    pub(crate) copied_style: Option<crate::domain::TextStyle>,
    pub(crate) inline_editing: bool,
    pub(crate) pending_cue_delay_ms: u64,
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
            new_presentation_open: false,
            show_delete_confirmation: false,
            delete_target_id: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            undo_coalesce: Vec::new(),
            copied_style: None,
            inline_editing: false,
            pending_cue_delay_ms: 0,
        }
    }
}

/// Which workspace pane a pointer drag is resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaneResizeTarget {
    Left,
    Right,
    Bottom,
}

/// The pane dimensions and the drag in progress, if any.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PaneDragState {
    pub(crate) target: PaneResizeTarget,
    pub(crate) start_pointer: Point<f32>,
    pub(crate) start_value: f32,
}

/// Shell layout and modal state.
pub(crate) struct ShellState {
    pub(crate) current_mode: ViewMode,
    pub(crate) inspector_tab: InspectorTab,
    pub(crate) search_query: String,
    pub(crate) sidebar_tab: SidebarTab,
    pub(crate) right_dock_tab: RightDockTab,
    pub(crate) left_panel_width: f32,
    pub(crate) right_panel_width: f32,
    pub(crate) bottom_panel_height: f32,
    pub(crate) pane_drag: Option<PaneDragState>,
    pub(crate) media_bin_open: bool,
    pub(crate) rail_context_target: Option<RailContextTarget>,
    pub(crate) rail_cursor_pos: Option<Point<f32>>,
    pub(crate) capture_settings_open: bool,
    pub(crate) clear_groups_open: bool,
    pub(crate) cue_list_slide: Option<usize>,
    pub(crate) groups_open: bool,
    pub(crate) test_pattern_open: bool,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            current_mode: ViewMode::Show,
            inspector_tab: InspectorTab::Text,
            search_query: String::new(),
            sidebar_tab: SidebarTab::default(),
            right_dock_tab: RightDockTab::default(),
            left_panel_width: 260.0,
            right_panel_width: 400.0,
            bottom_panel_height: 230.0,
            pane_drag: None,
            media_bin_open: true,
            rail_context_target: None,
            rail_cursor_pos: None,
            capture_settings_open: false,
            clear_groups_open: false,
            cue_list_slide: None,
            groups_open: false,
            test_pattern_open: false,
        }
    }
}

/// App-wide UI state.
#[derive(Default)]
pub(crate) struct UiState {
    pub(crate) shortcuts_open: bool,
    pub(crate) about_open: bool,
    pub(crate) reduce_motion: bool,
    pub(crate) error_message: Option<String>,
    /// Whether an update check is in flight.
    pub(crate) update_checking: bool,
    /// The last update-check result: the latest release, an error, or empty.
    pub(crate) update_check_status: String,
}

/// Canvas interaction state.
pub(crate) struct CanvasState {
    pub(crate) editing: Option<usize>,
    pub(crate) copied: Option<crate::domain::Object>,
    pub(crate) drag: Option<DragState>,
    pub(crate) zoom: f32,
    pub(crate) actual_size: bool,
    pub(crate) show_rulers: bool,
    pub(crate) slide_guides: bool,
    pub(crate) theme_guides: bool,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            editing: None,
            copied: None,
            drag: None,
            zoom: 1.0,
            actual_size: false,
            show_rulers: false,
            slide_guides: false,
            theme_guides: false,
        }
    }
}

/// Per-screen compositor tracking.
#[derive(Debug, Default)]
pub(crate) struct ScreenState {
    /// Content fingerprint last requested.
    pub(crate) fingerprint: u64,
    /// Generation of the in-flight request.
    pub(crate) generation: u64,
    /// Whether a request is in flight.
    pub(crate) in_flight: bool,
    /// When the request was issued.
    pub(crate) requested_at: Option<std::time::Instant>,
    /// Transition frames remaining.
    pub(crate) transition_remaining: usize,
    /// Whether the screen needs a fresh frame.
    pub(crate) dirty: bool,
}

/// Compositor handle and frame slots.
pub(crate) struct CompositorState {
    pub(crate) handle: Option<crate::ui::compositor::CompositorHandle>,
    pub(crate) slot: Arc<crate::render::frame_slot::FrameSlot>,
    pub(crate) screens: HashMap<String, ScreenState>,
    pub(crate) broadcast_generation: u64,
}

impl CompositorState {
    /// Create the state around a spawned worker.
    pub(crate) fn new(
        handle: crate::ui::compositor::CompositorHandle,
        slot: Arc<crate::render::frame_slot::FrameSlot>,
    ) -> Self {
        Self {
            handle: Some(handle),
            slot,
            screens: HashMap::new(),
            broadcast_generation: 0,
        }
    }
}

/// Stubs so slider entities type-check before the window wires them.
pub(crate) type SliderEntity = Entity<SliderState>;
