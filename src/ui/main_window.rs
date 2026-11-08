//! The root window: owns every feature state and renders the shell.

use std::sync::Arc;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::slider::SliderState;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, Context, Entity, FocusHandle, Focusable, IntoElement, MouseButton, MouseDownEvent,
    Point, Render, SharedString, Styled, TestSupportExt, Window, div, px,
};

use crate::config::Config;
use crate::db::{Database, LibraryRepository, ThemeRepository};
use crate::ui::canvas::{CanvasGuides, CanvasMode};
use crate::ui::messages::{Message, RightDockTab, SidebarTab, ViewMode};
use crate::ui::state::{
    AudioState, BibleState, CanvasState, CompositorState, EditorState, LayerState, LibraryState,
    OutputState, PresentingState, PropsState, RecordingFeatureState, ServiceState, ShellState,
    SongState, ThemesState, TriggersState, UiState, VideoState,
};
use crate::ui::theme;
use gpui_kit::ObjectFit;

/// The main window entity.
pub struct MainWindow {
    pub(crate) shell: ShellState,
    pub(crate) editor: EditorState,
    pub(crate) presenting: PresentingState,
    pub(crate) library: LibraryState,
    pub(crate) theme_state: ThemesState,
    pub(crate) ui: UiState,
    pub(crate) services: crate::services::Services,
    pub(crate) song: SongState,
    pub(crate) video: VideoState,
    pub(crate) service: ServiceState,
    pub(crate) app_config: Config,
    pub(crate) layer: LayerState,
    pub(crate) bible: BibleState,
    pub(crate) audio: AudioState,
    pub(crate) output: OutputState,
    pub(crate) props: PropsState,
    pub(crate) triggers: TriggersState,
    pub(crate) recording: RecordingFeatureState,
    pub(crate) calendar: crate::ui::calendar::CalendarState,
    pub(crate) cue_inspector: crate::ui::cue_inspector::CueSelection,
    pub(crate) clear_groups: crate::ui::clear_groups::ClearGroupsState,
    pub(crate) groups: crate::ui::groups::GroupsState,
    pub(crate) test_pattern: crate::ui::test_pattern::TestPatternState,
    pub(crate) canvas: CanvasState,
    pub(crate) compositor: CompositorState,
    pub(crate) capture_preset_name: String,
    pub(crate) import_export_tx: Option<tokio::sync::mpsc::Sender<crate::ui::import_export::Done>>,
    /// The update-check result channel: an in-flight "Check for Updates"
    /// posts its outcome here (the check itself runs on a worker thread).
    pub(crate) update_check_tx:
        Option<tokio::sync::mpsc::Sender<crate::ui::settings::UpdateCheckOutcome>>,
    // Field entities.
    pub(crate) focus_handle: FocusHandle,
    pub(crate) search_input: Entity<InputState>,
    pub(crate) new_presentation_input: Entity<InputState>,
    pub(crate) sync_rename_input: Entity<InputState>,
    pub(crate) inspector_text_input: Entity<InputState>,
    pub(crate) inspector_font_size_input: Entity<InputState>,
    pub(crate) inspector_transition_dur_input: Entity<InputState>,
    pub(crate) inspector_group_label_input: Entity<InputState>,
    pub(crate) inspector_notes_input: Entity<InputState>,
    pub(crate) themes_name_input: Entity<InputState>,
    pub(crate) layer_text_input: Entity<InputState>,
    pub(crate) layer_font_size_input: Entity<InputState>,
    pub(crate) layer_pos_x_input: Entity<InputState>,
    pub(crate) layer_pos_y_input: Entity<InputState>,
    pub(crate) layer_width_input: Entity<InputState>,
    pub(crate) layer_height_input: Entity<InputState>,
    pub(crate) layer_stroke_width_input: Entity<InputState>,
    pub(crate) layer_corner_radius_input: Entity<InputState>,
    pub(crate) layer_font_family_input: Entity<InputState>,
    pub(crate) layer_line_height_input: Entity<InputState>,
    pub(crate) layer_letter_spacing_input: Entity<InputState>,
    pub(crate) layer_glow_radius_input: Entity<InputState>,
    pub(crate) layer_text_stroke_width_input: Entity<InputState>,
    pub(crate) layer_text_hex_input: Entity<InputState>,
    pub(crate) props_new_name_input: Entity<InputState>,
    pub(crate) props_lower_third_title_input: Entity<InputState>,
    pub(crate) props_lower_third_subtitle_input: Entity<InputState>,
    pub(crate) props_look_name_input: Entity<InputState>,
    pub(crate) look_name_input: Entity<InputState>,
    pub(crate) alert_input: Entity<InputState>,
    pub(crate) announcement_input: Entity<InputState>,
    pub(crate) announcement_input_clear_pending: bool,
    pub(crate) bible_search_input: Entity<InputState>,
    pub(crate) calendar_name_input: Entity<InputState>,
    pub(crate) canvas_text_input: Entity<InputState>,
    pub(crate) capture_destination_input: Entity<InputState>,
    pub(crate) capture_file_name_input: Entity<InputState>,
    pub(crate) capture_preset_name_input: Entity<InputState>,
    pub(crate) clear_groups_name_input: Entity<InputState>,
    pub(crate) cue_name_input: Entity<InputState>,
    pub(crate) groups_name_input: Entity<InputState>,
    pub(crate) ndi_source_input: Entity<InputState>,
    pub(crate) output_label_input: Entity<InputState>,
    pub(crate) output_ndi_name_input: Entity<InputState>,
    pub(crate) output_screen_x_input: Entity<InputState>,
    pub(crate) output_screen_y_input: Entity<InputState>,
    pub(crate) playlist_header_input: Entity<InputState>,
    pub(crate) playlist_name_input: Entity<InputState>,
    pub(crate) recording_fps_input: Entity<InputState>,
    pub(crate) recording_path_input: Entity<InputState>,
    pub(crate) song_artist_input: Entity<InputState>,
    pub(crate) song_bpm_input: Entity<InputState>,
    pub(crate) song_ccli_input: Entity<InputState>,
    pub(crate) song_copyright_input: Entity<InputState>,
    pub(crate) song_key_input: Entity<InputState>,
    pub(crate) song_search_input: Entity<InputState>,
    pub(crate) song_title_input: Entity<InputState>,
    pub(crate) triggers_http_port_input: Entity<InputState>,
    pub(crate) triggers_macro_name_input: Entity<InputState>,
    pub(crate) triggers_osc_port_input: Entity<InputState>,
    pub(crate) layer_opacity_slider: Entity<SliderState>,
    pub(crate) layer_text_r_slider: Entity<SliderState>,
    pub(crate) layer_text_g_slider: Entity<SliderState>,
    pub(crate) layer_text_b_slider: Entity<SliderState>,
    pub(crate) layer_fill_r_slider: Entity<SliderState>,
    pub(crate) layer_fill_g_slider: Entity<SliderState>,
    pub(crate) layer_fill_b_slider: Entity<SliderState>,
    pub(crate) layer_fill_a_slider: Entity<SliderState>,
    pub(crate) layer_shape_stroke_r_slider: Entity<SliderState>,
    pub(crate) layer_shape_stroke_g_slider: Entity<SliderState>,
    pub(crate) layer_shape_stroke_b_slider: Entity<SliderState>,
    pub(crate) layer_shape_stroke_a_slider: Entity<SliderState>,
    pub(crate) layer_glow_r_slider: Entity<SliderState>,
    pub(crate) layer_glow_g_slider: Entity<SliderState>,
    pub(crate) layer_glow_b_slider: Entity<SliderState>,
    pub(crate) layer_stroke_r_slider: Entity<SliderState>,
    pub(crate) layer_stroke_g_slider: Entity<SliderState>,
    pub(crate) layer_stroke_b_slider: Entity<SliderState>,
    pub(crate) video_volume_slider: Entity<SliderState>,
    pub(crate) video_seek_slider: Entity<SliderState>,
    pub(crate) audio_volume_slider: Entity<SliderState>,
    pub(crate) transition_duration_slider: Entity<SliderState>,
    pub(crate) default_grid_cols_slider: Entity<SliderState>,
    /// Handle to the main window for output windows.
    pub(crate) window_handle: Option<gpui_kit::AnyWindowHandle>,
}

impl MainWindow {
    /// Create the window state.
    pub fn new(
        db: Arc<Database>,
        app_config: Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let services = crate::services::Services::new(db.clone());
        let mut trigger_manager = crate::triggers::TriggerManager::default();
        trigger_manager.http_port = app_config.triggers.http_port;
        trigger_manager.osc_port = app_config.triggers.osc_port;
        let rx = Arc::new(std::sync::Mutex::new(trigger_manager.subscribe()));
        let http_port_str = app_config.triggers.http_port.to_string();
        let osc_port_str = app_config.triggers.osc_port.to_string();

        let (handle, response_rx) = crate::ui::compositor::spawn_for_window();
        let slot = Arc::new(crate::render::frame_slot::FrameSlot::default());
        let compositor = CompositorState::new(handle, slot);

        let shell = ShellState {
            current_mode: match app_config.general.startup_mode.as_str() {
                "edit" => ViewMode::Edit,
                _ => ViewMode::Show,
            },
            ..Default::default()
        };
        let ui = UiState {
            reduce_motion: app_config.ui.reduce_motion,
            ..Default::default()
        };
        let output = OutputState::new(&app_config);

        let make_input =
            |cx: &mut Context<Self>, window: &mut Window| cx.new(|cx| InputState::new(window, cx));
        let make_slider = |cx: &mut Context<Self>| cx.new(|_cx| SliderState::new());

        let mut window_state = Self {
            shell,
            editor: EditorState::default(),
            presenting: PresentingState {
                global_transition: crate::domain::Transition::from_config_str(
                    &app_config.transitions.default_type,
                    app_config.transitions.default_duration_ms,
                ),
                slide_grid_cols: app_config.ui.default_grid_cols.clamp(2, 6),
                ..Default::default()
            },
            library: LibraryState::new(LibraryRepository::new(db.clone())),
            theme_state: ThemesState::new(ThemeRepository::new(db.clone())),
            ui,
            song: SongState::new(
                crate::db::SongRepository::new(db.clone()),
                services.songs.clone(),
            ),
            video: VideoState::default(),
            service: ServiceState::new(),
            app_config,
            layer: LayerState::default(),
            bible: BibleState::new(
                crate::db::BibleRepository::new(db.clone()),
                services.bible.clone(),
            ),
            audio: AudioState::default(),
            output,
            props: PropsState::default(),
            triggers: TriggersState {
                manager: trigger_manager,
                rx,
                panel_open: false,
                http_port_str,
                osc_port_str,
                midi_ports: crate::triggers::midi::list_midi_inputs(),
                selected_midi_port: None,
                new_macro_name: String::new(),
                macro_running_handles: std::collections::HashMap::new(),
                macro_running_ids: std::collections::HashSet::new(),
            },
            recording: RecordingFeatureState::default(),
            calendar: crate::ui::calendar::CalendarState::new(),
            cue_inspector: crate::ui::cue_inspector::CueSelection::default(),
            clear_groups: crate::ui::clear_groups::ClearGroupsState::new(),
            groups: crate::ui::groups::GroupsState::new(),
            test_pattern: crate::ui::test_pattern::TestPatternState::default(),
            canvas: CanvasState::default(),
            compositor,
            capture_preset_name: String::new(),
            import_export_tx: None,
            update_check_tx: None,
            focus_handle: cx.focus_handle(),
            search_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder(gpui_kit::SharedString::from(
                    "Search presentations, slides, songs…",
                ))
            }),
            new_presentation_input: make_input(cx, window),
            sync_rename_input: make_input(cx, window),
            inspector_text_input: make_input(cx, window),
            inspector_font_size_input: make_input(cx, window),
            inspector_transition_dur_input: make_input(cx, window),
            inspector_group_label_input: make_input(cx, window),
            inspector_notes_input: make_input(cx, window),
            themes_name_input: make_input(cx, window),
            layer_text_input: make_input(cx, window),
            layer_font_size_input: make_input(cx, window),
            layer_pos_x_input: make_input(cx, window),
            layer_pos_y_input: make_input(cx, window),
            layer_width_input: make_input(cx, window),
            layer_height_input: make_input(cx, window),
            layer_stroke_width_input: make_input(cx, window),
            layer_corner_radius_input: make_input(cx, window),
            layer_font_family_input: make_input(cx, window),
            layer_line_height_input: make_input(cx, window),
            layer_letter_spacing_input: make_input(cx, window),
            layer_glow_radius_input: make_input(cx, window),
            layer_text_stroke_width_input: make_input(cx, window),
            layer_text_hex_input: make_input(cx, window),
            props_new_name_input: make_input(cx, window),
            props_lower_third_title_input: make_input(cx, window),
            props_lower_third_subtitle_input: make_input(cx, window),
            props_look_name_input: make_input(cx, window),
            look_name_input: make_input(cx, window),
            alert_input: make_input(cx, window),
            announcement_input: make_input(cx, window),
            announcement_input_clear_pending: false,
            bible_search_input: make_input(cx, window),
            calendar_name_input: make_input(cx, window),
            canvas_text_input: make_input(cx, window),
            capture_destination_input: make_input(cx, window),
            capture_file_name_input: make_input(cx, window),
            capture_preset_name_input: make_input(cx, window),
            clear_groups_name_input: make_input(cx, window),
            cue_name_input: make_input(cx, window),
            groups_name_input: make_input(cx, window),
            ndi_source_input: make_input(cx, window),
            output_label_input: make_input(cx, window),
            output_ndi_name_input: make_input(cx, window),
            output_screen_x_input: make_input(cx, window),
            output_screen_y_input: make_input(cx, window),
            playlist_header_input: make_input(cx, window),
            playlist_name_input: make_input(cx, window),
            recording_fps_input: make_input(cx, window),
            recording_path_input: make_input(cx, window),
            song_artist_input: make_input(cx, window),
            song_bpm_input: make_input(cx, window),
            song_ccli_input: make_input(cx, window),
            song_copyright_input: make_input(cx, window),
            song_key_input: make_input(cx, window),
            song_search_input: make_input(cx, window),
            song_title_input: make_input(cx, window),
            triggers_http_port_input: make_input(cx, window),
            triggers_macro_name_input: make_input(cx, window),
            triggers_osc_port_input: make_input(cx, window),
            layer_opacity_slider: make_slider(cx),
            layer_text_r_slider: make_slider(cx),
            layer_text_g_slider: make_slider(cx),
            layer_text_b_slider: make_slider(cx),
            layer_fill_r_slider: make_slider(cx),
            layer_fill_g_slider: make_slider(cx),
            layer_fill_b_slider: make_slider(cx),
            layer_fill_a_slider: make_slider(cx),
            layer_shape_stroke_r_slider: make_slider(cx),
            layer_shape_stroke_g_slider: make_slider(cx),
            layer_shape_stroke_b_slider: make_slider(cx),
            layer_shape_stroke_a_slider: make_slider(cx),
            layer_glow_r_slider: make_slider(cx),
            layer_glow_g_slider: make_slider(cx),
            layer_glow_b_slider: make_slider(cx),
            layer_stroke_r_slider: make_slider(cx),
            layer_stroke_g_slider: make_slider(cx),
            layer_stroke_b_slider: make_slider(cx),
            video_volume_slider: make_slider(cx),
            video_seek_slider: make_slider(cx),
            audio_volume_slider: make_slider(cx),
            transition_duration_slider: make_slider(cx),
            default_grid_cols_slider: make_slider(cx),
            window_handle: None,
            services,
        };
        wire_inputs(&window_state, cx);
        window_state.load_presentations();
        window_state.load_songs();
        window_state.load_service_plans();
        window_state.load_bible_translations();
        window_state.load_themes();
        window_state.load_lib_assets();
        window_state.load_props();
        window_state.start_bridges(cx, response_rx);
        window_state
    }

    /// Start the helper tasks that drive clocks, compositing, and the
    /// import channel.
    fn start_bridges(
        &mut self,
        cx: &mut Context<Self>,
        mut response_rx: tokio::sync::mpsc::UnboundedReceiver<
            crate::ui::compositor::CompositorResponse,
        >,
    ) {
        let _import_tx = tokio::sync::mpsc::channel::<crate::ui::import_export::Done>(16).0;
        // The import channel is created here so background threads can post
        // completions back to the UI thread.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::ui::import_export::Done>(16);
        self.import_export_tx = Some(tx);
        // The update check answers on its own channel, so a slow network
        // never holds up the presenter.
        let (utx, mut urx) =
            tokio::sync::mpsc::channel::<crate::ui::settings::UpdateCheckOutcome>(4);
        self.update_check_tx = Some(utx);
        // Clock tick.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(500))
                    .await;
                let Some(entity) = this.upgrade() else {
                    break;
                };
                entity.update(cx, |w, cx| {
                    crate::ui::stage::clock_tick(w);
                    cx.notify();
                });
            }
        })
        .detach();
        // Compositor tick and frame responses.
        cx.spawn(async move |this, cx| {
            loop {
                tokio::select! {
                    _ = cx.background_executor().timer(std::time::Duration::from_millis(66)) => {
                        let Some(entity) = this.upgrade() else { break; };
                        entity.update(cx, |w, cx| {
                            crate::ui::compositor::request_if_changed(w, cx);
                            crate::ui::video::video_frame_tick(w, cx);
                            crate::ui::audio::audio_tick(w, cx);
                            cx.notify();
                        });
                    }
                    response = response_rx.recv() => {
                        let Some(response) = response else { break; };
                        let Some(entity) = this.upgrade() else { break; };
                        entity.update(cx, |w, cx| {
                            crate::ui::compositor::apply_frame(w, response, cx);
                        });
                    }
                    done = rx.recv() => {
                        let Some(done) = done else { break; };
                        let Some(entity) = this.upgrade() else { break; };
                        entity.update(cx, |w, cx| {
                            // The Done arrives without a Window; route through
                            // the handler with a no-op spinner handled inline.
                            crate::ui::import_export::done_now(w, done, cx);
                        });
                    }
                    update_done = urx.recv() => {
                        let Some(outcome) = update_done else { break; };
                        let Some(entity) = this.upgrade() else { break; };
                        entity.update(cx, |w, cx| {
                            w.ui.update_checking = false;
                            w.ui.update_check_status = outcome.message;
                            cx.notify();
                        });
                    }
                }
            }
        })
        .detach();
    }

    /// Show an error in the toast.
    pub(crate) fn set_error(&mut self, msg: impl Into<String>) {
        self.ui.error_message = Some(msg.into());
    }

    /// Snapshot the document for undo.
    pub(crate) fn push_undo(&mut self) {
        if let Some(ref pres) = self.editor.editing {
            self.editor.undo_stack.push(pres.clone());
            if self.editor.undo_stack.len() > 50 {
                self.editor.undo_stack.remove(0);
            }
            self.editor.redo_stack.clear();
        }
    }

    /// Snapshot, coalescing rapid edits.
    pub(crate) fn push_undo_coalesced(&mut self) {
        self.push_undo();
    }

    /// The slide being edited.
    pub(crate) fn get_current_slide(&self) -> Option<&crate::domain::Slide> {
        let index = self.editor.selected_slide_index?;
        self.editor.editing.as_ref()?.slides.get(index)
    }

    /// The slide being edited, mutably.
    pub(crate) fn get_current_slide_mut(&mut self) -> Option<&mut crate::domain::Slide> {
        let index = self.editor.selected_slide_index?;
        self.editor.editing.as_mut()?.slides.get_mut(index)
    }

    /// The indices the selection points at.
    pub(crate) fn selected_object_indices(&self) -> Vec<usize> {
        if !self.layer.selected_indices.is_empty() {
            self.layer.selected_indices.clone()
        } else if let Some(i) = self.layer.selected_index {
            vec![i]
        } else {
            Vec::new()
        }
    }

    /// Persist the edited slide.
    pub(crate) fn persist_slide(&mut self, slide: crate::domain::Slide, cx: &mut Context<Self>) {
        if let Some(ref pres) = self.editor.editing {
            let id = pres.id.clone();
            if let Err(e) = self.services.presentations.update_slide(&id, &slide) {
                self.set_error(format!("Save failed: {e}"));
            }
        }
        let _ = cx;
    }

    /// Reload the presentation list.
    pub(crate) fn load_presentations(&mut self) {
        match self.services.presentations.list() {
            Ok(list) => self.editor.presentations = list,
            Err(e) => self.set_error(format!("Failed to load presentations: {e}")),
        }
    }

    /// Reload the song list.
    pub(crate) fn load_songs(&mut self) {
        match self.song.service.list() {
            Ok(list) => self.song.songs = list,
            Err(e) => self.set_error(format!("Failed to load songs: {e}")),
        }
    }

    /// Reload the playlist list.
    pub(crate) fn load_service_plans(&mut self) {
        match self.services.playlists.list() {
            Ok(list) => self.service.plans = list,
            Err(e) => self.set_error(format!("Failed to load service plans: {e}")),
        }
    }

    /// Reload the Bible translations.
    pub(crate) fn load_bible_translations(&mut self) {
        match self.bible.service.list_translations() {
            Ok(list) => self.bible.translations = list,
            Err(e) => self.set_error(format!("Failed to load Bible translations: {e}")),
        }
    }

    /// Reload the themes.
    pub(crate) fn load_themes(&mut self) {
        match self.theme_state.repo.list_themes() {
            Ok(list) => self.theme_state.list = list,
            Err(e) => self.set_error(format!("Failed to load themes: {e}")),
        }
    }

    /// Reload the library assets.
    pub(crate) fn load_lib_assets(&mut self) {
        match self.library.repo.list_assets() {
            Ok(list) => {
                self.library.assets = list.clone();
                self.library.recent_ids = list.iter().take(12).map(|a| a.id.clone()).collect();
            }
            Err(e) => self.set_error(format!("Failed to load library assets: {e}")),
        }
    }

    /// Where the props file lives.
    pub(crate) fn props_path(&self) -> std::path::PathBuf {
        self.app_config
            .db_path
            .parent()
            .map(|p| p.join("props.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("props.json"))
    }

    /// Load the props library from disk.
    pub(crate) fn load_props(&mut self) {
        let path = self.props_path();
        if path.exists() {
            self.props.manager = crate::domain::PropManager::load_from_file(&path);
        }
        if self.props.manager.looks.is_empty() {
            let default_look = crate::domain::Look::with_default_screens("Default");
            let active_id = default_look.id.clone();
            self.props.manager.looks = vec![
                default_look,
                crate::domain::Look::with_default_screens("Broadcast / Lower Third"),
            ];
            self.props.manager.active_look_id = Some(active_id);
        }
    }

    /// Write the props library to disk.
    pub(crate) fn save_props(&mut self) {
        let path = self.props_path();
        if let Err(e) = self.props.manager.save_to_file(&path) {
            self.set_error(format!("Failed to save props: {e}"));
        }
    }

    /// Rebuild the menu bar.
    pub(crate) fn refresh_menu_if_changed(&mut self, cx: &mut gpui_kit::App) {
        let capitalization = self
            .get_current_slide()
            .and_then(|slide| slide.editable_text(self.layer.selected_index))
            .map(|(_, style)| style.text_transform)
            .unwrap_or_default();
        let baseline = self
            .get_current_slide()
            .and_then(|slide| slide.editable_text(self.layer.selected_index))
            .map(|(_, style)| style.baseline)
            .unwrap_or_default();
        let ligatures = self
            .get_current_slide()
            .and_then(|slide| slide.editable_text(self.layer.selected_index))
            .map(|(_, style)| style.ligatures)
            .unwrap_or_default();
        let font_family = self
            .get_current_slide()
            .and_then(|slide| slide.editable_text(self.layer.selected_index))
            .map(|(_, style)| style.font_family.clone())
            .unwrap_or_default();
        let state = crate::ui::menus::MenuState {
            capitalization,
            baseline,
            ligatures,
            font_family,
            slide_view: match self.presenting.slide_view_mode {
                crate::ui::messages::SlideViewMode::Easy => crate::ui::menus::SlideViewChoice::Easy,
                crate::ui::messages::SlideViewMode::Grid => crate::ui::menus::SlideViewChoice::Grid,
                crate::ui::messages::SlideViewMode::Table => {
                    crate::ui::menus::SlideViewChoice::Table
                }
            },
            editor_visible: self.shell.current_mode == ViewMode::Edit,
            continuous_playlist: self.service.continuous,
            media_bin_visible: self.shell.media_bin_open,
        };
        let menus = crate::ui::menus::build(&state);
        cx.set_menus(menus);
    }

    /// The window title.
    pub(crate) fn title(&self) -> String {
        match (&self.shell.current_mode, &self.editor.editing) {
            (ViewMode::Edit, Some(pres)) => format!("{} — OpenPresenter", pres.name),
            _ => String::from("OpenPresenter"),
        }
    }

    /// Dispatch a message to the feature that owns it.
    pub fn handle_message(&mut self, msg: Message, window: &mut Window, cx: &mut Context<Self>) {
        match msg {
            Message::Noop => {}
            Message::EscapePressed => crate::ui::navigation::escape_pressed(self, window, cx),
            Message::StartupConfig => {
                if self.app_config.triggers.http_enabled {
                    self.triggers.manager.start_http();
                }
                if self.app_config.triggers.osc_enabled {
                    self.triggers.manager.start_osc();
                }
            }
            Message::DismissError => {
                self.ui.error_message = None;
            }
            Message::SwitchMode(mode) => crate::ui::navigation::switch_mode(self, mode, cx),
            Message::SwitchInspectorTab(tab) => {
                self.shell.inspector_tab = tab;
                cx.notify();
            }
            Message::OpenTheme => crate::ui::navigation::open_theme(self, cx),
            Message::OpenTextEditor => crate::ui::navigation::open_text_editor(self, cx),
            Message::FocusSearch => crate::ui::navigation::focus_search(self, window, cx),
            Message::ToggleEditMode => {
                let next = match self.shell.current_mode {
                    ViewMode::Edit => ViewMode::Show,
                    ViewMode::Show => ViewMode::Edit,
                };
                crate::ui::navigation::switch_mode(self, next, cx);
            }
            Message::SelectLeftSection(tab) => {
                crate::ui::navigation::switch_sidebar_tab(self, tab, window, cx)
            }
            Message::SelectRightDockTab(tab) => {
                self.shell.right_dock_tab = tab;
                cx.notify();
            }
            Message::ToggleMediaBin => {
                self.shell.media_bin_open = !self.shell.media_bin_open;
                cx.notify();
            }
            Message::BackToList => crate::ui::navigation::back_to_list(self, cx),
            Message::Quit => {
                cx.quit();
            }
            Message::SearchQueryChanged(q) => {
                crate::ui::navigation::search_query_changed(self, q, cx)
            }
            Message::SearchResultSlide(pres_id, idx) => {
                crate::ui::navigation::open_search_result_slide(self, pres_id, idx, window, cx)
            }
            Message::NewPresentationClicked => {
                crate::ui::navigation::new_presentation_clicked(self, window, cx)
            }
            Message::CreatePresentation => {
                crate::ui::navigation::create_presentation(self, window, cx)
            }
            Message::CancelNewPresentation => {
                crate::ui::navigation::cancel_new_presentation(self, cx)
            }
            Message::NewPresentationNameChanged(name) => {
                self.editor.editing_name = name;
                cx.notify();
            }
            Message::SelectPresentation(id) => {
                if let Ok(pres) = self.services.presentations.get(&id) {
                    crate::ui::navigation::open_for_editing(self, pres, None);
                }
            }
            Message::OpenPresentation(id) => {
                crate::ui::navigation::open_presentation(self, id, window, cx)
            }
            Message::RenamePresentation => crate::ui::navigation::rename_presentation(self, cx),
            Message::RenamePresentationChanged(name) => {
                crate::ui::navigation::rename_changed(self, name, cx)
            }
            Message::DeletePresentationClicked(id) => {
                crate::ui::navigation::delete_presentation_clicked(self, id, cx)
            }
            Message::ConfirmDeletePresentation => {
                crate::ui::navigation::confirm_delete_presentation(self, cx)
            }
            Message::CancelDelete => crate::ui::navigation::cancel_delete(self, cx),
            Message::PresentingSelectSlide(i) => crate::ui::navigation::select_slide(self, i, cx),
            Message::PresentingSelectLastSlide => {
                crate::ui::navigation::select_last_slide(self, cx)
            }
            Message::PresentingNextSlide => crate::ui::navigation::next_slide(self, cx),
            Message::PresentingPrevSlide => crate::ui::navigation::prev_slide(self, cx),
            Message::ShowSlidesCursorMoved(point) => {
                self.presenting.slide_context_pos = Some(point);
            }
            Message::ShowSlideContextMenu(i) => {
                self.presenting.slide_context_index = Some(i);
                if self.presenting.slide_context_pos.is_none() {
                    self.presenting.slide_context_pos = Some(Point::new(0.0, 0.0));
                }
                cx.notify();
            }
            Message::ShowSlideGroupSubmenu => {
                self.presenting.group_submenu = !self.presenting.group_submenu;
                cx.notify();
            }
            Message::ShowSlideCueSubmenu => {
                self.presenting.cue_submenu = !self.presenting.cue_submenu;
                cx.notify();
            }
            Message::ShowSlideTransitionSubmenu => {
                self.presenting.transition_submenu = !self.presenting.transition_submenu;
                cx.notify();
            }
            Message::HideSlideContextMenu => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                cx.notify();
            }
            Message::ShowRailContextMenu(target) => {
                self.shell.rail_context_target = Some(target);
                cx.notify();
            }
            Message::HideRailContextMenu => {
                self.shell.rail_context_target = None;
                cx.notify();
            }
            Message::RailCursorMoved(point) => {
                self.shell.rail_cursor_pos = Some(point);
            }
            Message::DuplicatePresentation(id) => {
                crate::ui::navigation::duplicate_presentation(self, id, cx)
            }
            Message::EditPresentation(id) => {
                crate::ui::navigation::edit_presentation(self, id, window, cx)
            }
            Message::ExportPresentation(id) => {
                crate::ui::import_export::export_presentation(self, &id);
            }
            Message::SongToPresentation(id) => {
                crate::ui::playlist::song_to_presentation(self, id, window, cx)
            }
            Message::AddSlideCue(i, cue) => crate::ui::navigation::add_slide_cue(self, i, cue, cx),
            Message::ClearSlideCues(i) => crate::ui::navigation::clear_slide_cues(self, i, cx),
            Message::SetSlideTransition(i, t) => {
                crate::ui::navigation::set_slide_transition(self, i, t, cx)
            }
            Message::EditSlide(i) => crate::ui::navigation::edit_slide(self, i, cx),
            Message::SetSlideGridCols(cols) => {
                crate::ui::navigation::set_slide_grid_cols(self, cols, cx)
            }
            Message::SetSlideViewMode(mode) => {
                crate::ui::navigation::set_slide_view_mode(self, mode, cx)
            }
            Message::JumpToGroup(prefix) => crate::ui::navigation::jump_to_group(self, &prefix, cx),
            Message::SetGlobalTransition(t) => {
                crate::ui::navigation::set_global_transition(self, t, cx)
            }
            Message::AnimationTick => {
                crate::ui::compositor::request_if_changed(self, cx);
            }
            Message::StartTimer => crate::ui::navigation::start_timer(self, cx),
            Message::StopTimer => crate::ui::navigation::stop_timer(self, cx),
            Message::ResetTimer => crate::ui::navigation::reset_timer(self, cx),
            Message::CountdownAdd(secs) => crate::ui::navigation::countdown_add(self, secs, cx),
            Message::CountdownClear => crate::ui::navigation::countdown_clear(self, cx),
            Message::ToggleNdi => crate::ui::ndi::toggle_ndi(self, cx),
            Message::NdiSendCurrent => crate::ui::ndi::send_current(self, cx),
            Message::NdiBlackScreen => crate::ui::ndi::black_screen(self, cx),
            Message::ClearOutput => crate::ui::navigation::clear_output(self, cx),
            Message::ClearAll => crate::ui::navigation::clear_all(self, cx),
            Message::ClearSlide => crate::ui::navigation::clear_slide(self, cx),
            Message::ClearMedia => crate::ui::navigation::clear_media(self, cx),
            Message::ClearProps => crate::ui::navigation::clear_props(self, cx),
            Message::ClearMessages => crate::ui::navigation::clear_messages(self, cx),
            Message::ClearAudio => crate::ui::navigation::clear_audio(self, cx),
            Message::SelectPreviewScreen(id) => {
                crate::ui::navigation::select_preview_screen(self, id, cx)
            }
            Message::ToggleLooksMatrixModal => {
                self.output.matrix_open = !self.output.matrix_open;
                cx.notify();
            }
            Message::SelectLook(id) => crate::ui::output::select_look(self, id, cx),
            Message::CreateLook(name) => crate::ui::output::create_look(self, name, window, cx),
            Message::DeleteLook(id) => crate::ui::output::delete_look(self, id, cx),
            Message::LookNameInputChanged(name) => {
                crate::ui::output::look_name_input_changed(self, name, cx)
            }
            Message::ToggleLookScreenLayer(look, screen, layer) => {
                crate::ui::output::toggle_look_screen_layer(self, look, screen, layer, cx)
            }
            Message::SetLookScreenTheme(look, screen, theme) => {
                crate::ui::output::set_look_screen_theme(self, look, screen, theme, cx)
            }
            Message::SendLiveMessage(_) => {
                crate::ui::navigation::send_live_message(self, window, cx)
            }
            Message::DismissLiveMessage => crate::ui::navigation::dismiss_live_message(self, cx),
            Message::LiveMessageInputChanged(text) => {
                crate::ui::navigation::live_message_input_changed(self, text, cx)
            }
            Message::AnnouncementInputChanged(text) => {
                crate::ui::navigation::announcement_input_changed(self, text, cx)
            }
            Message::Ndi(msg) => crate::ui::ndi::update(self, msg, cx),
            Message::ImageFilePicked(path) => crate::ui::slides::image_file_picked(self, path, cx),
            Message::VideoFilePicked(path) => crate::ui::slides::video_file_picked(self, path, cx),
            Message::ToggleStageDisplay => crate::ui::navigation::toggle_stage_display(self, cx),
            Message::ToggleStageWindow => crate::ui::stage::toggle_stage_window(self, cx),
            Message::ClockTick => crate::ui::stage::clock_tick(self),
            Message::Stage(msg) => crate::ui::stage::update(self, msg, cx),
            Message::SwitchSidebarTab(tab) => {
                crate::ui::navigation::switch_sidebar_tab(self, tab, window, cx)
            }
            Message::Library(msg) => crate::ui::library::update(self, msg, cx),
            Message::Themes(msg) => crate::ui::themes::update(self, msg, cx),
            Message::Slides(msg) => crate::ui::slides::update(self, msg, cx),
            Message::Layers(msg) => crate::ui::layers::update(self, msg, cx),
            Message::Typography(msg) => crate::ui::typography::update(self, msg, cx),
            Message::ToggleShortcutsOverlay => {
                self.ui.shortcuts_open = !self.ui.shortcuts_open;
                cx.notify();
            }
            Message::ToggleAbout => {
                self.ui.about_open = !self.ui.about_open;
                cx.notify();
            }
            Message::Undo => crate::ui::navigation::undo(self, cx),
            Message::Redo => crate::ui::navigation::redo(self, cx),
            Message::ToggleReduceMotion => crate::ui::navigation::toggle_reduce_motion(self, cx),
            Message::Playlist(msg) => crate::ui::playlist::update(self, msg, window, cx),
            Message::Songs(msg) => crate::ui::songs::update(self, msg, window, cx),
            Message::Bible(msg) => crate::ui::bible::update(self, msg, window, cx),
            Message::Calendar(msg) => crate::ui::calendar::update(self, msg, cx),
            Message::CaptureSettings(msg) => crate::ui::capture_settings::update(self, msg, cx),
            Message::ClearGroups(msg) => crate::ui::clear_groups::update(self, msg, cx),
            Message::CueInspector(msg) => crate::ui::cue_inspector::update(self, msg, cx),
            Message::Groups(msg) => crate::ui::groups::update(self, msg, cx),
            Message::TestPattern(msg) => crate::ui::test_pattern::update(self, msg, cx),
            Message::Timeline(msg) => crate::ui::timeline::update(self, msg, cx),
            Message::VideoFrameTick => crate::ui::video::video_frame_tick(self, cx),
            Message::AudioTick => crate::ui::audio::audio_tick(self, cx),
            Message::Video(msg) => crate::ui::video::update(self, msg, cx),
            Message::OpenOutputWindow => crate::ui::output::open_output_window(self, cx),
            Message::CloseOutputWindow => crate::ui::output::close_output_window(self, cx),
            Message::ToggleOutputWindow => crate::ui::output::toggle_output_window(self, cx),
            Message::OutputWindowOpened => crate::ui::output::sync_output_windows(self, cx),
            Message::ToggleOutputBlackScreen => {
                crate::ui::navigation::toggle_black_screen(self, cx)
            }
            Message::ToggleOutputSettings => {
                if self.output.settings_open {
                    crate::ui::output::close_settings(self, cx);
                } else {
                    crate::ui::output::open_settings(self, window, cx);
                }
            }
            Message::OutputScreenXChanged(v) => {
                crate::ui::output::output_screen_x_changed(self, v, cx)
            }
            Message::OutputScreenYChanged(v) => {
                crate::ui::output::output_screen_y_changed(self, v, cx)
            }
            Message::OutputAutoFullscreenToggled(v) => {
                crate::ui::output::output_auto_fullscreen_toggled(self, v, cx)
            }
            Message::OutputFullscreenToggled => {
                crate::ui::output::output_fullscreen_toggled(self, cx)
            }
            Message::Audio(msg) => crate::ui::audio::update(self, msg, cx),
            Message::Output(msg) => crate::ui::output::update(self, msg, window, cx),
            Message::ImportExport(msg) => crate::ui::import_export::update(self, msg, cx),
            Message::Props(msg) => crate::ui::props::update(self, msg, cx),
            Message::SetMask(mask) => crate::ui::props::set_mask(self, mask, cx),
            Message::TogglePropsPanel => {
                self.props.panel_open = !self.props.panel_open;
                cx.notify();
            }
            Message::ToggleTriggersPanel => {
                self.triggers.panel_open = !self.triggers.panel_open;
                cx.notify();
            }
            Message::Triggers(msg) => crate::ui::triggers::update(self, msg, window, cx),
            Message::TriggerFired(action) => crate::ui::triggers::apply_action(self, action, cx),
            Message::Recording(msg) => crate::ui::recording::update(self, msg, cx),
            Message::ToggleRecordingPanel => {
                self.recording.panel_open = !self.recording.panel_open;
                cx.notify();
            }
            Message::Zoom(cmd) => crate::ui::shell::menu::zoom(self, cmd, cx),
            Message::Arrange(cmd) => {
                crate::ui::arrange::apply(self, cmd, cx);
            }
            Message::Insert(cmd) => {
                crate::ui::insert::apply(self, cmd, cx);
            }
            Message::TextStyle(cmd) => {
                crate::ui::typography::apply_text_style_command(self, cmd, cx);
            }
            Message::ToggleRulers => {
                self.canvas.show_rulers = !self.canvas.show_rulers;
                cx.notify();
            }
            Message::ToggleSlideGuides => {
                self.canvas.slide_guides = !self.canvas.slide_guides;
                cx.notify();
            }
            Message::ToggleThemeGuides => {
                self.canvas.theme_guides = !self.canvas.theme_guides;
                cx.notify();
            }
            Message::CanvasBeginTextEdit(index) => {
                crate::ui::canvas::begin_text_edit(self, index, window, cx)
            }
            Message::CopyObject => crate::ui::canvas::copy_object(self, cx),
            Message::PasteObject => crate::ui::canvas::paste_object(self, cx),
            Message::DeleteObject => crate::ui::layers::delete_selected_layer(self, cx),
            Message::DuplicateObject => crate::ui::layers::duplicate_selected_layer(self, cx),
            Message::NudgeLeft => crate::ui::layers::nudge_selected_layer(self, -1.0, 0.0, cx),
            Message::NudgeRight => crate::ui::layers::nudge_selected_layer(self, 1.0, 0.0, cx),
            Message::NudgeUp => crate::ui::layers::nudge_selected_layer(self, 0.0, -1.0, cx),
            Message::NudgeDown => crate::ui::layers::nudge_selected_layer(self, 0.0, 1.0, cx),
            Message::NudgeLeftFast => crate::ui::layers::nudge_selected_layer(self, -10.0, 0.0, cx),
            Message::NudgeRightFast => crate::ui::layers::nudge_selected_layer(self, 10.0, 0.0, cx),
            Message::NudgeUpFast => crate::ui::layers::nudge_selected_layer(self, 0.0, -10.0, cx),
            Message::NudgeDownFast => crate::ui::layers::nudge_selected_layer(self, 0.0, 10.0, cx),
            Message::OpenSettings => crate::ui::output::open_settings(self, window, cx),
            Message::OpenSettingsTab(tab) => {
                crate::ui::output::open_settings(self, window, cx);
                crate::ui::output::select_tab(self, tab, cx);
            }
        }
    }

    /// Handle a key action fired by the shell.
    ///
    /// A command only applies in the mode it belongs to: presenter keys
    /// drive the show, editor keys edit the slide, and while a text field
    /// owns the focus the key belongs to the field. Returning `false`
    /// releases the event, so the field still receives the character.
    pub(crate) fn handle_action(
        &mut self,
        action: &dyn gpui_kit::Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // The actions are namespaced ("openpresenter::NextSlide"), so the
        // match works on the bare name.
        let name = action.name().rsplit("::").next().unwrap_or(action.name());
        let typing = input_is_focused(self, window, cx);
        let show = self.shell.current_mode == ViewMode::Show;
        let edit = !show;
        let live = self
            .presenting
            .presentation
            .as_ref()
            .is_some_and(|pres| !pres.slides.is_empty());
        let show_key = show && live && !typing;
        let edit_key = edit && !typing;
        match name {
            "NextSlide" | "PrevSlide" | "FirstSlide" | "LastSlide" | "JumpToVerse"
            | "JumpToChorus" => {
                if !show_key {
                    return false;
                }
                match name {
                    "NextSlide" => crate::ui::navigation::next_slide(self, cx),
                    "PrevSlide" => crate::ui::navigation::prev_slide(self, cx),
                    "FirstSlide" => crate::ui::navigation::select_slide(self, 0, cx),
                    "LastSlide" => crate::ui::navigation::select_last_slide(self, cx),
                    "JumpToVerse" => crate::ui::navigation::jump_to_group(self, "verse", cx),
                    _ => crate::ui::navigation::jump_to_group(self, "chorus", cx),
                }
                true
            }
            "ClearAll" | "ClearSlide" | "ClearMedia" | "ClearProps" | "ClearAudio"
            | "ClearMessages" | "ToggleBlackScreen" => {
                if !show || typing {
                    return false;
                }
                match name {
                    "ClearAll" => crate::ui::navigation::clear_all(self, cx),
                    "ClearSlide" => crate::ui::navigation::clear_slide(self, cx),
                    "ClearMedia" => crate::ui::navigation::clear_media(self, cx),
                    "ClearProps" => crate::ui::navigation::clear_props(self, cx),
                    "ClearAudio" => crate::ui::navigation::clear_audio(self, cx),
                    "ClearMessages" => crate::ui::navigation::clear_messages(self, cx),
                    _ => crate::ui::navigation::toggle_black_screen(self, cx),
                }
                true
            }
            "GotoSlide1" | "GotoSlide2" | "GotoSlide3" | "GotoSlide4" | "GotoSlide5"
            | "GotoSlide6" | "GotoSlide7" | "GotoSlide8" | "GotoSlide9" => {
                if !show_key {
                    return false;
                }
                let index = name
                    .strip_prefix("GotoSlide")
                    .and_then(|n| n.parse::<usize>().ok())
                    .map(|n| n.saturating_sub(1))
                    .unwrap_or(0);
                crate::ui::navigation::select_slide(self, index, cx);
                true
            }
            "Undo" => {
                if !edit_key {
                    return false;
                }
                crate::ui::navigation::undo(self, cx);
                true
            }
            "Redo" => {
                if !edit_key {
                    return false;
                }
                crate::ui::navigation::redo(self, cx);
                true
            }
            "DeleteObject" => {
                if !edit_key {
                    return false;
                }
                let has_selection = self.layer.selected_index.is_some();
                if !has_selection {
                    return false;
                }
                crate::ui::layers::delete_selected_layer(self, cx);
                true
            }
            "DuplicateObject" => {
                if !edit_key || self.layer.selected_index.is_none() {
                    return false;
                }
                crate::ui::layers::duplicate_selected_layer(self, cx);
                true
            }
            "CopyObject" => {
                if !edit_key || self.layer.selected_index.is_none() {
                    return false;
                }
                crate::ui::canvas::copy_object(self, cx);
                true
            }
            "PasteObject" => {
                if !edit_key {
                    return false;
                }
                crate::ui::canvas::paste_object(self, cx);
                true
            }
            "NudgeLeft" | "NudgeRight" | "NudgeUp" | "NudgeDown" | "NudgeLeftFast"
            | "NudgeRightFast" | "NudgeUpFast" | "NudgeDownFast" => {
                if typing {
                    return false;
                }
                if edit {
                    let (dx, dy) = match name {
                        "NudgeLeft" => (-1.0, 0.0),
                        "NudgeRight" => (1.0, 0.0),
                        "NudgeUp" => (0.0, -1.0),
                        "NudgeDown" => (0.0, 1.0),
                        "NudgeLeftFast" => (-10.0, 0.0),
                        "NudgeRightFast" => (10.0, 0.0),
                        "NudgeUpFast" => (0.0, -10.0),
                        _ => (0.0, 10.0),
                    };
                    crate::ui::layers::nudge_selected_layer(self, dx, dy, cx);
                    return true;
                }
                // The arrows belong to the show as well: left/right step and
                // up/down step the other way.
                if !show_key {
                    return false;
                }
                match name {
                    "NudgeRight" | "NudgeRightFast" | "NudgeDown" | "NudgeDownFast" => {
                        crate::ui::navigation::next_slide(self, cx);
                    }
                    _ => crate::ui::navigation::prev_slide(self, cx),
                }
                true
            }
            "Escape" => {
                crate::ui::navigation::escape_pressed(self, window, cx);
                true
            }
            // The menu bar. Every menu action is registered with
            // `on_boxed_action` in the root view and lands here, so a menu
            // item can never be a command that does nothing. The menu-only
            // actions have no key binding, so they are gated by the mode
            // alone, not by whether a text field is focused: a click on a
            // menu item is explicit intent, and a focused input must not
            // swallow it.
            "NewPresentation" => {
                self.handle_message(Message::NewPresentationClicked, window, cx);
                true
            }
            "NewPlaylist" => {
                self.handle_message(
                    Message::SelectLeftSection(SidebarTab::Playlists),
                    window,
                    cx,
                );
                self.handle_message(
                    Message::Playlist(crate::ui::playlist::Message::New),
                    window,
                    cx,
                );
                true
            }
            "NewLibrary" => {
                // There is one library; a "new library" lands the operator
                // on it, where they can start importing into it.
                self.handle_message(Message::SelectLeftSection(SidebarTab::Library), window, cx);
                true
            }
            "ImportFile" => {
                self.handle_message(
                    Message::ImportExport(crate::ui::import_export::Message::ImportOpp),
                    window,
                    cx,
                );
                true
            }
            "ImportMedia" => {
                crate::ui::shell::menu::import_media(self, window, cx);
                true
            }
            "ImportTextFromClipboard" => {
                crate::ui::shell::menu::import_text_from_clipboard(self, window, cx);
                true
            }
            "ImportLibrary" => {
                self.handle_message(
                    Message::Library(crate::ui::library::Message::ImportAsset),
                    window,
                    cx,
                );
                true
            }
            "ImportPowerPoint" => {
                self.handle_message(
                    Message::ImportExport(crate::ui::import_export::Message::ImportPowerPoint),
                    window,
                    cx,
                );
                true
            }
            "ImportPowerPointAsImages" => {
                crate::ui::import_export::import_powerpoint(
                    self,
                    crate::ui::import_export::PowerPointImport::AsImages,
                    cx,
                );
                true
            }
            "ImportPowerPointAsPresentation" => {
                crate::ui::import_export::import_powerpoint(
                    self,
                    crate::ui::import_export::PowerPointImport::AsPresentation,
                    cx,
                );
                true
            }
            "ExportPresentation" => {
                let Some(editing) = self.editor.editing.clone() else {
                    return false;
                };
                self.handle_message(Message::ExportPresentation(editing.id), window, cx);
                true
            }
            "ExportSlideImages" => {
                crate::ui::shell::menu::export_slide_images(self, window, cx);
                true
            }
            "ExportPlaylist" => {
                crate::ui::shell::menu::export_playlist(self, window, cx);
                true
            }
            "ExportText" => {
                crate::ui::shell::menu::export_text(self, window, cx);
                true
            }
            "ExportThemes" => {
                crate::ui::shell::menu::export_themes(self, window, cx);
                true
            }
            "ExportPdf" | "PrintPresentation" => {
                crate::ui::import_export::print_presentation(self, cx);
                true
            }
            "Cut" => {
                if !edit {
                    return false;
                }
                crate::ui::shell::menu::cut_object(self, window, cx);
                true
            }
            "CopyTextStyle" | "PasteTextStyle" => {
                if !edit {
                    return false;
                }
                let command = if name == "CopyTextStyle" {
                    crate::ui::messages::TextStyleCommand::CopyStyle
                } else {
                    crate::ui::messages::TextStyleCommand::PasteStyle
                };
                self.handle_message(Message::TextStyle(command), window, cx);
                true
            }
            "SelectAll" => {
                if !edit {
                    return false;
                }
                crate::ui::shell::menu::select_all(self, window, cx);
                true
            }
            "DeselectAll" => {
                if !edit {
                    return false;
                }
                crate::ui::shell::menu::deselect_all(self, window, cx);
                true
            }
            "SearchLibrary" => {
                if !edit {
                    return false;
                }
                self.handle_message(Message::FocusSearch, window, cx);
                true
            }
            "TransformUpperCase" | "TransformLowerCase" | "TransformCapitalize" => {
                if !edit {
                    return false;
                }
                let rewrite = match name {
                    "TransformUpperCase" => crate::domain::TextRewrite::UpperCase,
                    "TransformLowerCase" => crate::domain::TextRewrite::LowerCase,
                    _ => crate::domain::TextRewrite::Capitalize,
                };
                crate::ui::typography::apply_text_rewrite(self, rewrite, cx);
                true
            }
            "NewSlide" => {
                if !edit {
                    return false;
                }
                self.handle_message(
                    Message::Slides(crate::ui::slides::Message::AddSlide),
                    window,
                    cx,
                );
                true
            }
            "InsertText"
            | "InsertShape"
            | "InsertMedia"
            | "InsertWeb"
            | "InsertTimer"
            | "InsertPlaybackMarker"
            | "InsertScreenPreview"
            | "InsertStageMessage"
            | "InsertTimecode"
            | "InsertChordChart" => {
                if !edit {
                    return false;
                }
                use crate::ui::messages::InsertCommand as Ins;
                let command = match name {
                    "InsertText" => Ins::Text,
                    "InsertShape" => Ins::Shape,
                    "InsertMedia" => Ins::Media,
                    "InsertWeb" => Ins::Web,
                    "InsertTimer" => Ins::Timer,
                    "InsertPlaybackMarker" => Ins::PlaybackMarker,
                    "InsertScreenPreview" => Ins::ScreenPreview,
                    "InsertStageMessage" => Ins::StageMessage,
                    "InsertTimecode" => Ins::Timecode,
                    _ => Ins::ChordChart,
                };
                self.handle_message(Message::Insert(command), window, cx);
                true
            }
            "AlignLeft" | "AlignCenter" | "AlignRight" | "AlignTop" | "AlignMiddle"
            | "AlignBottom" => {
                if !edit {
                    return false;
                }
                use crate::domain::AlignMode as Align;
                let mode = match name {
                    "AlignLeft" => Align::Left,
                    "AlignCenter" => Align::Center,
                    "AlignRight" => Align::Right,
                    "AlignTop" => Align::Top,
                    "AlignMiddle" => Align::Middle,
                    _ => Align::Bottom,
                };
                self.handle_message(
                    Message::Arrange(crate::ui::messages::ArrangeCommand::Align(mode)),
                    window,
                    cx,
                );
                true
            }
            "BringForward" | "BringToFront" | "SendBackward" | "SendToBack" => {
                if !edit {
                    return false;
                }
                use crate::domain::Reorder as Order;
                let order = match name {
                    "BringForward" => Order::Forward,
                    "BringToFront" => Order::ToFront,
                    "SendBackward" => Order::Backward,
                    _ => Order::ToBack,
                };
                self.handle_message(
                    Message::Arrange(crate::ui::messages::ArrangeCommand::Reorder(order)),
                    window,
                    cx,
                );
                true
            }
            "FlipHorizontally" | "FlipVertically" => {
                if !edit {
                    return false;
                }
                let command = if name == "FlipHorizontally" {
                    crate::ui::messages::ArrangeCommand::FlipHorizontal
                } else {
                    crate::ui::messages::ArrangeCommand::FlipVertical
                };
                self.handle_message(Message::Arrange(command), window, cx);
                true
            }
            "LockObject" | "UnlockAllObjects" => {
                if !edit {
                    return false;
                }
                let command = if name == "LockObject" {
                    crate::ui::messages::ArrangeCommand::Lock
                } else {
                    crate::ui::messages::ArrangeCommand::UnlockAll
                };
                self.handle_message(Message::Arrange(command), window, cx);
                true
            }
            "TextBold" | "TextItalic" | "TextUnderline" | "TextStrikethrough" => {
                if !edit {
                    return false;
                }
                use crate::ui::messages::TextStyleCommand as Style;
                let command = match name {
                    "TextBold" => Style::Bold,
                    "TextItalic" => Style::Italic,
                    "TextUnderline" => Style::Underline,
                    _ => Style::Strikethrough,
                };
                self.handle_message(Message::TextStyle(command), window, cx);
                true
            }
            "TextBigger"
            | "TextSmaller"
            | "TextLoosen"
            | "TextTighten"
            | "TextSpacingDefault"
            | "TextApplyStyleToAllSlides" => {
                if !edit {
                    return false;
                }
                use crate::ui::messages::TextStyleCommand as Style;
                let command = match name {
                    "TextBigger" => Style::Bigger,
                    "TextSmaller" => Style::Smaller,
                    "TextLoosen" => Style::Loosen,
                    "TextTighten" => Style::Tighten,
                    "TextSpacingDefault" => Style::SpacingDefault,
                    _ => Style::ApplyToAllSlides,
                };
                self.handle_message(Message::TextStyle(command), window, cx);
                true
            }
            "TextFontArial" | "TextFontGeorgia" | "TextFontImpact" | "TextFontCourierNew" => {
                if !edit {
                    return false;
                }
                let family = match name {
                    "TextFontArial" => "Arial",
                    "TextFontGeorgia" => "Georgia",
                    "TextFontImpact" => "Impact",
                    _ => "Courier New",
                };
                self.handle_message(
                    Message::Typography(
                        crate::ui::typography::Message::SelectedLayerFontFamilyChanged(
                            family.to_string(),
                        ),
                    ),
                    window,
                    cx,
                );
                true
            }
            "TextCapitalizationNone"
            | "TextCapitalizationAllCaps"
            | "TextCapitalizationSmallCaps"
            | "TextCapitalizationTitleCase"
            | "TextCapitalizationStartCase"
            | "TextCapitalizationLowerCase" => {
                if !edit {
                    return false;
                }
                use crate::domain::TextTransform as Casing;
                let casing = match name {
                    "TextCapitalizationNone" => Casing::None,
                    "TextCapitalizationAllCaps" => Casing::Uppercase,
                    "TextCapitalizationSmallCaps" => Casing::SmallCaps,
                    "TextCapitalizationTitleCase" => Casing::TitleCase,
                    "TextCapitalizationStartCase" => Casing::StartCase,
                    _ => Casing::Lowercase,
                };
                self.handle_message(
                    Message::TextStyle(crate::ui::messages::TextStyleCommand::Capitalization(
                        casing,
                    )),
                    window,
                    cx,
                );
                true
            }
            "TextBaselineDefault"
            | "TextBaselineRaise"
            | "TextBaselineLower"
            | "TextBaselineSuperscript"
            | "TextBaselineSubscript" => {
                if !edit {
                    return false;
                }
                use crate::domain::Baseline as Base;
                let baseline = match name {
                    "TextBaselineDefault" => Base::Default,
                    "TextBaselineRaise" => Base::Raise,
                    "TextBaselineLower" => Base::Lower,
                    "TextBaselineSuperscript" => Base::Superscript,
                    _ => Base::Subscript,
                };
                self.handle_message(
                    Message::TextStyle(crate::ui::messages::TextStyleCommand::Baseline(baseline)),
                    window,
                    cx,
                );
                true
            }
            "TextLigaturesDefault" | "TextLigaturesNone" => {
                if !edit {
                    return false;
                }
                use crate::domain::Ligatures as Lig;
                let ligatures = if name == "TextLigaturesDefault" {
                    Lig::Default
                } else {
                    Lig::None
                };
                self.handle_message(
                    Message::TextStyle(crate::ui::messages::TextStyleCommand::Ligatures(ligatures)),
                    window,
                    cx,
                );
                true
            }
            "ZoomActualSize" | "ZoomFitCanvas" | "ZoomFitSlide" | "ZoomIn" | "ZoomOut" => {
                if !edit {
                    return false;
                }
                use crate::ui::messages::ZoomCommand as Zoom;
                let command = match name {
                    "ZoomActualSize" => Zoom::ActualSize,
                    "ZoomFitCanvas" => Zoom::FitCanvas,
                    "ZoomFitSlide" => Zoom::FitSlide,
                    "ZoomIn" => Zoom::In,
                    _ => Zoom::Out,
                };
                self.handle_message(Message::Zoom(command), window, cx);
                true
            }
            "ShowRulers" | "ShowSlideGuides" | "ShowThemeGuides" => {
                if !edit {
                    return false;
                }
                let msg = match name {
                    "ShowRulers" => Message::ToggleRulers,
                    "ShowSlideGuides" => Message::ToggleSlideGuides,
                    _ => Message::ToggleThemeGuides,
                };
                self.handle_message(msg, window, cx);
                true
            }
            "ClearGroups" => {
                if !show {
                    return false;
                }
                let index = self.clear_groups.selected;
                self.handle_message(
                    Message::ClearGroups(crate::ui::clear_groups::Message::Clear(index)),
                    window,
                    cx,
                );
                true
            }
            "ClearAnnouncements" => {
                if !show {
                    return false;
                }
                crate::ui::navigation::clear_announcements(self, cx);
                true
            }
            "ClearToLogo" => {
                if !show {
                    return false;
                }
                crate::ui::navigation::clear_to_logo(self, cx);
                true
            }
            "ConfigureClearGroups" => {
                self.handle_message(
                    Message::ClearGroups(crate::ui::clear_groups::Message::Open),
                    window,
                    cx,
                );
                true
            }
            "RevealInLibrary" => {
                if self.editor.editing.is_none() {
                    return false;
                }
                crate::ui::shell::menu::reveal_in_library(self, window, cx);
                true
            }
            "NextBackground" | "PreviousBackground" => {
                if !edit {
                    return false;
                }
                if name == "NextBackground" {
                    crate::ui::shell::menu::next_background(self, window, cx);
                } else {
                    crate::ui::shell::menu::previous_background(self, window, cx);
                }
                true
            }
            "ViewEditor" => {
                self.handle_message(Message::SwitchMode(ViewMode::Edit), window, cx);
                true
            }
            "ViewShow" => {
                self.handle_message(Message::SwitchMode(ViewMode::Show), window, cx);
                true
            }
            "ViewStage" | "ViewTimecode" => {
                self.handle_message(Message::ToggleStageDisplay, window, cx);
                true
            }
            "ViewMediaBin" => {
                self.handle_message(Message::ToggleMediaBin, window, cx);
                true
            }
            "ViewProps" | "ViewMessages" | "ViewTimers" => {
                if !show {
                    self.handle_message(Message::SwitchMode(ViewMode::Show), window, cx);
                }
                let tab = match name {
                    "ViewProps" => RightDockTab::Props,
                    "ViewMessages" => RightDockTab::Messages,
                    _ => RightDockTab::Timers,
                };
                self.handle_message(Message::SelectRightDockTab(tab), window, cx);
                true
            }
            "ViewBibles" => {
                self.handle_message(Message::SelectLeftSection(SidebarTab::Bible), window, cx);
                true
            }
            "ViewActionPalette" | "OpenKeyMappings" | "OpenHelp" => {
                self.handle_message(Message::ToggleShortcutsOverlay, window, cx);
                true
            }
            "ViewMacros" => {
                self.handle_message(Message::ToggleTriggersPanel, window, cx);
                true
            }
            "ViewContinuousPlaylist" => {
                // The menu runs the plan continuously: it starts the plan
                // when nothing is running, otherwise it toggles whether the
                // service wraps at the end.
                if self.service.active.is_some() {
                    let on = !self.service.continuous;
                    self.handle_message(
                        Message::Playlist(crate::ui::playlist::Message::SetContinuous(on)),
                        window,
                        cx,
                    );
                } else if self.service.editing.is_some() {
                    self.handle_message(
                        Message::Playlist(crate::ui::playlist::Message::SetContinuous(true)),
                        window,
                        cx,
                    );
                    self.handle_message(
                        Message::Playlist(crate::ui::playlist::Message::Start),
                        window,
                        cx,
                    );
                } else {
                    self.handle_message(
                        Message::SelectLeftSection(SidebarTab::Playlists),
                        window,
                        cx,
                    );
                }
                true
            }
            "ViewTimeline" => {
                self.handle_message(Message::SwitchMode(ViewMode::Show), window, cx);
                true
            }
            "ViewCalendar" => {
                self.handle_message(
                    Message::Calendar(crate::ui::calendar::Message::Open),
                    window,
                    cx,
                );
                true
            }
            "ViewSlideEasy" | "ViewSlideGrid" | "ViewSlideTable" => {
                let mode = match name {
                    "ViewSlideEasy" => crate::ui::messages::SlideViewMode::Easy,
                    "ViewSlideGrid" => crate::ui::messages::SlideViewMode::Grid,
                    _ => crate::ui::messages::SlideViewMode::Table,
                };
                self.handle_message(Message::SetSlideViewMode(mode), window, cx);
                true
            }
            "AboutApp" => {
                self.handle_message(Message::ToggleAbout, window, cx);
                true
            }
            "QuitApp" => {
                cx.quit();
                true
            }
            "HideApp" => {
                cx.hide();
                true
            }
            "HideOtherApps" => {
                cx.hide_other_apps();
                true
            }
            "ShowAllApps" => {
                cx.unhide_other_apps();
                true
            }
            "OpenSettingsGeneral"
            | "OpenSettingsAudio"
            | "OpenSettingsDevices"
            | "OpenSettingsGroups"
            | "OpenSettingsImport"
            | "OpenSettingsInputs"
            | "OpenSettingsNetwork"
            | "OpenSettingsScreens"
            | "OpenSettingsUpdates"
            | "OpenSettingsMedia"
            | "OpenSettingsSlideLabels"
            | "OpenSettingsTransitions"
            | "OpenSettingsAdvanced" => {
                use crate::ui::messages::SettingsTab as Tab;
                let tab = match name {
                    "OpenSettingsGeneral" => Tab::General,
                    "OpenSettingsAudio" => Tab::Audio,
                    "OpenSettingsDevices" => Tab::Devices,
                    "OpenSettingsGroups" => Tab::GroupsAndLabels,
                    "OpenSettingsImport" => Tab::Import,
                    "OpenSettingsInputs" => Tab::Inputs,
                    "OpenSettingsNetwork" => Tab::Network,
                    "OpenSettingsScreens" => Tab::Screens,
                    "OpenSettingsUpdates" => Tab::Updates,
                    "OpenSettingsMedia" => Tab::Media,
                    "OpenSettingsSlideLabels" => Tab::SlideLabels,
                    "OpenSettingsTransitions" => Tab::Transitions,
                    _ => Tab::Advanced,
                };
                self.handle_message(Message::OpenSettingsTab(tab), window, cx);
                true
            }
            "OpenGroupsSettings" => {
                self.handle_message(
                    Message::Groups(crate::ui::groups::Message::Open),
                    window,
                    cx,
                );
                true
            }
            "OpenTestPatterns" => {
                self.handle_message(
                    Message::TestPattern(crate::ui::test_pattern::Message::Open),
                    window,
                    cx,
                );
                true
            }
            "OpenCueInspector" => {
                let index = if show {
                    self.presenting.slide_index
                } else {
                    self.editor.selected_slide_index.unwrap_or(0)
                };
                let has_cues = self
                    .editor
                    .editing
                    .as_ref()
                    .and_then(|pres| pres.slides.get(index))
                    .map(|slide| !slide.cues.is_empty())
                    .unwrap_or(false);
                let msg = if has_cues {
                    crate::ui::cue_inspector::Message::Open(0)
                } else {
                    crate::ui::cue_inspector::Message::OpenList(index)
                };
                self.handle_message(Message::CueInspector(msg), window, cx);
                true
            }
            "ResetAllSettings" => {
                self.handle_message(
                    Message::Output(crate::ui::output::Message::ResetToDefaults),
                    window,
                    cx,
                );
                true
            }
            "MinimizeWindow" => {
                window.minimize_window();
                true
            }
            "ZoomWindow" => {
                window.zoom_window();
                true
            }
            "BringAllToFront" => {
                window.activate_window();
                true
            }
            _ => false,
        }
    }
}

/// A listener that routes a global key action to [`MainWindow::handle_action`].
///
/// The actions are registered one by one with `on_boxed_action`, because
/// GPUI dispatches by concrete action type: a single `dyn Action` listener
/// cannot be registered. A command that does not apply releases the event,
/// so a character key that belongs to a text field still reaches it.
fn action_route(
    cx: &Context<MainWindow>,
) -> impl Fn(&dyn gpui_kit::Action, &mut Window, &mut gpui_kit::App) + 'static {
    cx.listener(|w, action: &dyn gpui_kit::Action, window, cx| {
        if !w.handle_action(action, window, cx) {
            cx.propagate();
        }
    })
}

/// Whether one of the window's text fields owns the keyboard focus.
///
/// While a field has the focus, every character belongs to the field: a
/// presenter shortcut that was also a letter would swallow the letter and
/// the operator could not type a name at all.
fn input_is_focused(w: &MainWindow, window: &Window, cx: &mut Context<MainWindow>) -> bool {
    let Some(focused) = window.focused(cx) else {
        return false;
    };
    if focused == w.focus_handle {
        return false;
    }
    let handles = [
        w.search_input.read(cx).focus_handle(cx),
        w.new_presentation_input.read(cx).focus_handle(cx),
        w.sync_rename_input.read(cx).focus_handle(cx),
        w.inspector_text_input.read(cx).focus_handle(cx),
        w.inspector_font_size_input.read(cx).focus_handle(cx),
        w.inspector_transition_dur_input.read(cx).focus_handle(cx),
        w.inspector_group_label_input.read(cx).focus_handle(cx),
        w.inspector_notes_input.read(cx).focus_handle(cx),
        w.themes_name_input.read(cx).focus_handle(cx),
        w.layer_text_input.read(cx).focus_handle(cx),
        w.layer_font_size_input.read(cx).focus_handle(cx),
        w.layer_pos_x_input.read(cx).focus_handle(cx),
        w.layer_pos_y_input.read(cx).focus_handle(cx),
        w.layer_width_input.read(cx).focus_handle(cx),
        w.layer_height_input.read(cx).focus_handle(cx),
        w.layer_stroke_width_input.read(cx).focus_handle(cx),
        w.layer_corner_radius_input.read(cx).focus_handle(cx),
        w.layer_font_family_input.read(cx).focus_handle(cx),
        w.layer_line_height_input.read(cx).focus_handle(cx),
        w.layer_letter_spacing_input.read(cx).focus_handle(cx),
        w.layer_glow_radius_input.read(cx).focus_handle(cx),
        w.layer_text_stroke_width_input.read(cx).focus_handle(cx),
        w.layer_text_hex_input.read(cx).focus_handle(cx),
        w.props_new_name_input.read(cx).focus_handle(cx),
        w.props_lower_third_title_input.read(cx).focus_handle(cx),
        w.props_lower_third_subtitle_input.read(cx).focus_handle(cx),
        w.props_look_name_input.read(cx).focus_handle(cx),
        w.look_name_input.read(cx).focus_handle(cx),
        w.alert_input.read(cx).focus_handle(cx),
        w.announcement_input.read(cx).focus_handle(cx),
        w.bible_search_input.read(cx).focus_handle(cx),
        w.calendar_name_input.read(cx).focus_handle(cx),
        w.canvas_text_input.read(cx).focus_handle(cx),
        w.capture_destination_input.read(cx).focus_handle(cx),
        w.capture_file_name_input.read(cx).focus_handle(cx),
        w.capture_preset_name_input.read(cx).focus_handle(cx),
        w.clear_groups_name_input.read(cx).focus_handle(cx),
        w.cue_name_input.read(cx).focus_handle(cx),
        w.groups_name_input.read(cx).focus_handle(cx),
        w.ndi_source_input.read(cx).focus_handle(cx),
        w.output_label_input.read(cx).focus_handle(cx),
        w.output_ndi_name_input.read(cx).focus_handle(cx),
        w.output_screen_x_input.read(cx).focus_handle(cx),
        w.output_screen_y_input.read(cx).focus_handle(cx),
        w.playlist_header_input.read(cx).focus_handle(cx),
        w.playlist_name_input.read(cx).focus_handle(cx),
        w.recording_fps_input.read(cx).focus_handle(cx),
        w.recording_path_input.read(cx).focus_handle(cx),
        w.song_artist_input.read(cx).focus_handle(cx),
        w.song_bpm_input.read(cx).focus_handle(cx),
        w.song_ccli_input.read(cx).focus_handle(cx),
        w.song_copyright_input.read(cx).focus_handle(cx),
        w.song_key_input.read(cx).focus_handle(cx),
        w.song_search_input.read(cx).focus_handle(cx),
        w.song_title_input.read(cx).focus_handle(cx),
        w.triggers_http_port_input.read(cx).focus_handle(cx),
        w.triggers_macro_name_input.read(cx).focus_handle(cx),
        w.triggers_osc_port_input.read(cx).focus_handle(cx),
    ];
    handles.contains(&focused)
}

/// Mirror a field's typing into the feature state that owns it.
///
/// The fields are entity-stateful, so nothing writes them back into the
/// model on its own: the subscription is the copy. `set_value` emits no
/// change, so syncing the fields from the model never loops.
fn subscribe_input(
    cx: &mut Context<MainWindow>,
    entity: &Entity<InputState>,
    on_change: impl Fn(&mut MainWindow, String, &mut Context<MainWindow>) + 'static,
) {
    let input = entity.clone();
    let input_sub = input.clone();
    cx.subscribe(&input_sub, move |this, _, event: &InputEvent, cx| {
        if let InputEvent::Change = event {
            let value = input.read(cx).value().to_string();
            on_change(this, value, cx);
        }
    })
    .detach();
}

/// Wire every text field entity in the window to the state it mirrors.
fn wire_inputs(w: &MainWindow, cx: &mut Context<MainWindow>) {
    subscribe_input(cx, &w.search_input, |this, value, cx| {
        crate::ui::navigation::search_query_changed(this, value, cx);
    });
    subscribe_input(cx, &w.new_presentation_input, |this, value, cx| {
        this.editor.editing_name = value;
        cx.notify();
    });
    subscribe_input(cx, &w.sync_rename_input, |this, value, cx| {
        crate::ui::navigation::rename_changed(this, value, cx);
    });
    subscribe_input(cx, &w.inspector_text_input, |this, value, cx| {
        crate::ui::slides::slide_text_changed(this, value, cx);
    });
    subscribe_input(cx, &w.inspector_font_size_input, |this, value, cx| {
        crate::ui::slides::slide_font_size_changed(this, value, cx);
    });
    subscribe_input(cx, &w.inspector_transition_dur_input, |this, value, cx| {
        crate::ui::slides::transition_duration_changed(this, value, cx);
    });
    subscribe_input(cx, &w.inspector_group_label_input, |this, value, cx| {
        crate::ui::slides::group_label_changed(this, value, cx);
    });
    subscribe_input(cx, &w.inspector_notes_input, |this, value, cx| {
        crate::ui::slides::slide_notes_changed(this, value, cx);
    });
    subscribe_input(cx, &w.themes_name_input, |this, value, cx| {
        crate::ui::themes::theme_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_text_input, |this, value, cx| {
        crate::ui::layers::selected_layer_text_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_font_size_input, |this, value, cx| {
        crate::ui::layers::selected_layer_font_size_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_pos_x_input, |this, value, cx| {
        crate::ui::layers::selected_layer_position_x_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_pos_y_input, |this, value, cx| {
        crate::ui::layers::selected_layer_position_y_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_width_input, |this, value, cx| {
        crate::ui::layers::selected_layer_width_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_height_input, |this, value, cx| {
        crate::ui::layers::selected_layer_height_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_stroke_width_input, |this, value, cx| {
        crate::ui::layers::selected_layer_shape_stroke_width_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_corner_radius_input, |this, value, cx| {
        crate::ui::layers::selected_layer_shape_corner_radius_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_font_family_input, |this, value, cx| {
        crate::ui::typography::font_family_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_line_height_input, |this, value, cx| {
        crate::ui::typography::line_height_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_letter_spacing_input, |this, value, cx| {
        crate::ui::typography::letter_spacing_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_glow_radius_input, |this, value, cx| {
        crate::ui::typography::glow_radius_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_text_stroke_width_input, |this, value, cx| {
        crate::ui::typography::text_stroke_width_changed(this, value, cx);
    });
    subscribe_input(cx, &w.layer_text_hex_input, |this, value, cx| {
        text_hex_changed(this, value, cx);
    });
    subscribe_input(cx, &w.props_new_name_input, |this, value, cx| {
        crate::ui::props::new_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.props_lower_third_title_input, |this, value, cx| {
        crate::ui::props::lower_third_title_changed(this, value, cx);
    });
    subscribe_input(
        cx,
        &w.props_lower_third_subtitle_input,
        |this, value, cx| {
            crate::ui::props::lower_third_subtitle_changed(this, value, cx);
        },
    );
    subscribe_input(cx, &w.props_look_name_input, |this, value, cx| {
        crate::ui::props::look_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.look_name_input, |this, value, cx| {
        crate::ui::output::look_name_input_changed(this, value, cx);
    });
    subscribe_input(cx, &w.alert_input, |this, value, cx| {
        crate::ui::navigation::live_message_input_changed(this, value, cx);
    });
    subscribe_input(cx, &w.announcement_input, |this, value, cx| {
        crate::ui::navigation::announcement_input_changed(this, value, cx);
    });
    subscribe_input(cx, &w.bible_search_input, |this, value, cx| {
        crate::ui::bible::search_changed(this, value, cx);
    });
    subscribe_input(cx, &w.calendar_name_input, |this, value, cx| {
        crate::ui::calendar::update(this, crate::ui::calendar::Message::NameChanged(value), cx);
    });
    subscribe_input(cx, &w.canvas_text_input, |this, value, cx| {
        crate::ui::canvas::text_edited(this, value, cx);
    });
    subscribe_input(cx, &w.capture_destination_input, |this, value, cx| {
        crate::ui::capture_settings::update(
            this,
            crate::ui::capture_settings::Message::DestinationChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.capture_file_name_input, |this, value, cx| {
        crate::ui::capture_settings::update(
            this,
            crate::ui::capture_settings::Message::FileNameChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.capture_preset_name_input, |this, value, cx| {
        crate::ui::capture_settings::update(
            this,
            crate::ui::capture_settings::Message::NewNameChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.clear_groups_name_input, |this, value, cx| {
        crate::ui::clear_groups::update(
            this,
            crate::ui::clear_groups::Message::NewNameChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.cue_name_input, |this, value, cx| {
        crate::ui::cue_inspector::update(
            this,
            crate::ui::cue_inspector::Message::NameChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.groups_name_input, |this, value, cx| {
        crate::ui::groups::update(
            this,
            crate::ui::groups::Message::NewGroupNameChanged(value),
            cx,
        );
    });
    subscribe_input(cx, &w.ndi_source_input, |this, value, cx| {
        crate::ui::output::ndi_source_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.output_label_input, |this, value, cx| {
        crate::ui::output::new_label_changed(this, value, cx);
    });
    subscribe_input(cx, &w.output_ndi_name_input, |this, value, cx| {
        crate::ui::output::new_ndi_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.output_screen_x_input, |this, value, cx| {
        crate::ui::output::output_screen_x_changed(this, value, cx);
    });
    subscribe_input(cx, &w.output_screen_y_input, |this, value, cx| {
        crate::ui::output::output_screen_y_changed(this, value, cx);
    });
    subscribe_input(cx, &w.playlist_header_input, |this, value, cx| {
        let index = this.service.item_index;
        crate::ui::playlist::item_header_changed(this, index, value, cx);
    });
    subscribe_input(cx, &w.playlist_name_input, |this, value, cx| {
        crate::ui::playlist::plan_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.recording_fps_input, |this, value, _cx| {
        crate::ui::recording::fps_changed(this, value);
    });
    subscribe_input(cx, &w.recording_path_input, |this, value, _cx| {
        crate::ui::recording::path_changed(this, value);
    });
    subscribe_input(cx, &w.song_artist_input, |this, value, cx| {
        crate::ui::songs::artist_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_bpm_input, |this, value, cx| {
        crate::ui::songs::bpm_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_ccli_input, |this, value, cx| {
        crate::ui::songs::ccli_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_copyright_input, |this, value, cx| {
        crate::ui::songs::copyright_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_key_input, |this, value, cx| {
        crate::ui::songs::key_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_search_input, |this, value, cx| {
        crate::ui::songs::search_changed(this, value, cx);
    });
    subscribe_input(cx, &w.song_title_input, |this, value, cx| {
        crate::ui::songs::title_changed(this, value, cx);
    });
    subscribe_input(cx, &w.triggers_http_port_input, |this, value, cx| {
        crate::ui::triggers::http_port_changed(this, value, cx);
    });
    subscribe_input(cx, &w.triggers_macro_name_input, |this, value, cx| {
        crate::ui::triggers::macro_name_changed(this, value, cx);
    });
    subscribe_input(cx, &w.triggers_osc_port_input, |this, value, cx| {
        crate::ui::triggers::osc_port_changed(this, value, cx);
    });
}

/// Parse a `#RRGGBB` hex colour into the selected text object's style.
fn text_hex_changed(w: &mut MainWindow, value: String, cx: &mut Context<MainWindow>) {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return;
    }
    let rgb = u32::from_str_radix(hex, 16).unwrap_or(0);
    let colour = crate::domain::Color {
        r: ((rgb >> 16) & 0xff) as u8,
        g: ((rgb >> 8) & 0xff) as u8,
        b: (rgb & 0xff) as u8,
        a: 255,
    };
    w.push_undo_coalesced();
    if let Some(layer) = crate::ui::layers::selected_layer_mut(w)
        && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
    {
        style.color = colour;
    }
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Keep the panel fields in step with the model before drawing.
        crate::ui::inspector::sync_inputs(self, window, cx);
        crate::ui::output::sync_inputs(self, window, cx);
        crate::ui::slides::sync_inputs(self, window, cx);
        crate::ui::songs::sync_inputs(self, window, cx);
        crate::ui::playlist::sync_inputs(self, window, cx);
        crate::ui::triggers::sync_inputs(self, window, cx);
        crate::ui::recording::sync_inputs(self, window, cx);
        crate::ui::audio::sync_inputs(self, window, cx);
        crate::ui::bible::sync_inputs(self, window, cx);
        crate::ui::library::sync_inputs(self, window, cx);
        crate::ui::props::sync_inputs(self, window, cx);

        // The new-presentation field takes the focus when its modal opens,
        // so the operator can type the name straight away.
        if self.editor.new_presentation_open {
            let input = self.new_presentation_input.clone();
            window.focus(&input.read(cx).focus_handle(cx), cx);
        }

        let navbar = crate::ui::shell::navbar::view(self, cx).into_any_element();
        let left_rail = left_rail(self, cx);
        let center = center(self, cx);
        let right_dock = right_dock(self, cx);
        let modals = crate::ui::shell::modals::view(self, cx).into_any_element();
        let slide_menu = slide_context_menu(self, cx);
        let rail_menu = rail_context_menu(self, cx);
        let toast = self.ui.error_message.clone().map(|msg| {
            crate::ui::components::error_toast(
                msg,
                cx.listener(|w, _, window, cx| w.handle_message(Message::DismissError, window, cx)),
            )
            .into_any_element()
        });

        let mut root = div()
            .id("main-root")
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::c(theme::BG_DARKEST))
            .track_focus(&self.focus_handle)
            // A click anywhere in the shell returns the keyboard to the
            // shell: the canvas objects take the click without taking the
            // focus, and a key pressed then would go nowhere. The capture
            // phase runs before the canvas handler stops the click, and a
            // field the operator clicks into still takes the focus itself
            // afterwards, because its own mouse-down runs later.
            .capture_any_mouse_down(cx.listener(|w, _, window, cx| {
                let handle = w.focus_handle.clone();
                handle.focus(window, cx);
            }))
            .on_boxed_action(&crate::ui::actions::FirstSlide, action_route(cx))
            .on_boxed_action(&crate::ui::actions::LastSlide, action_route(cx))
            .on_boxed_action(&crate::ui::actions::JumpToVerse, action_route(cx))
            .on_boxed_action(&crate::ui::actions::JumpToChorus, action_route(cx))
            .on_boxed_action(&crate::ui::actions::ClearAll, action_route(cx))
            .on_boxed_action(&crate::ui::actions::ToggleBlackScreen, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide1, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide2, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide3, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide4, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide5, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide6, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide7, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide8, action_route(cx))
            .on_boxed_action(&crate::ui::actions::GotoSlide9, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeLeft, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeRight, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeUp, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeDown, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeLeftFast, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeRightFast, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeUpFast, action_route(cx))
            .on_boxed_action(&crate::ui::actions::NudgeDownFast, action_route(cx))
            .on_boxed_action(&crate::ui::actions::Escape, action_route(cx));
        // Every action the menu bar names gets a route, so every menu item
        // dispatches to `handle_action`: a menu item with no route would be
        // a command the operator can click that does nothing. The keys'
        // own actions (above) are not in this list, so nothing fires twice.
        for action in crate::ui::actions::menu_actions() {
            root = root.on_boxed_action(&*action, action_route(cx));
        }
        root.child(navbar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .child(left_rail)
                    .child(center)
                    .child(div().w(px(400.)).flex_shrink_0().h_full().child(right_dock)),
            )
            .child(modals)
            .child(slide_menu)
            .child(rail_menu)
            .child(
                div()
                    .absolute()
                    .top(px(54.))
                    .right(px(12.))
                    .child(toast.unwrap_or_else(|| div().into_any_element())),
            )
    }
}

/// The left rail: search, vertical nav, section list, footer.
///
/// Matches the reference: the five libraries are a vertical list with an
/// orange outline on the active one, not a horizontal tab bar. The section
/// header names the list below it, and the footer holds the new/import/
/// export/media-bin controls.
fn left_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let tab = w.shell.sidebar_tab;
    let body: AnyElement = match tab {
        SidebarTab::Presentations => presentations_rail(w, cx),
        SidebarTab::Playlists => playlist_rail(w, cx),
        SidebarTab::Library => library_rail(w, cx),
        SidebarTab::Songs => songs_rail(w, cx),
        SidebarTab::Bible => bible_rail(w, cx),
    };
    div()
        .w(px(240.))
        .h_full()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARK))
        .border_r_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div().p(px(8.)).child(
                Input::new(&w.search_input.clone())
                    .small()
                    .aria_label("Search presentations, slides, songs"),
            ),
        )
        .child(rail_nav(w, cx))
        .child(body)
        .child(rail_footer(w, cx))
        .into_any_element()
}

/// The vertical library nav above the section list.
fn rail_nav(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let tab = w.shell.sidebar_tab;
    let items = [
        (
            SidebarTab::Presentations,
            "rail-presentations",
            "Presentations",
        ),
        (SidebarTab::Playlists, "rail-playlists", "Playlists"),
        (SidebarTab::Library, "rail-library", "Library"),
        (SidebarTab::Songs, "rail-songs", "Songs"),
        (SidebarTab::Bible, "rail-bible", "Bible"),
    ];
    let mut col = div().flex().flex_col().gap(px(1.)).px(px(8.)).pb(px(6.));
    for (value, id, label) in items {
        let active = tab == value;
        col = col.child(
            div()
                .id(id.to_string())
                .test_support()
                .w_full()
                .px(px(10.))
                .py(px(7.))
                .rounded(px(4.))
                .text_size(px(12.))
                .text_color(if active {
                    theme::c(theme::TEXT_PRIMARY)
                } else {
                    theme::c(theme::TEXT_SECONDARY)
                })
                .bg(if active {
                    theme::c(theme::ACCENT_ORANGE).opacity(0.14)
                } else {
                    theme::c(theme::TRANSPARENT)
                })
                .border_1()
                .border_color(if active {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::TRANSPARENT)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::SwitchSidebarTab(value), window, cx);
                }))
                .child(label),
        );
    }
    col.into_any_element()
}

/// The rail footer: new presentation, import/export, and the media bin.
fn rail_footer(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div().flex().flex_row().justify_center().child(
                Button::new("rail-new-presentation")
                    .label("+ New Presentation")
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(Message::NewPresentationClicked, window, cx);
                    })),
            ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(4.))
                .px(px(8.))
                .pb(px(6.))
                .child(
                    Button::new("rail-import")
                        .label("Import .opp")
                        .xsmall()
                        .flex_1()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::ImportExport(crate::ui::import_export::Message::ImportOpp),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("rail-export")
                        .label("Export .opp")
                        .xsmall()
                        .flex_1()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::ImportExport(crate::ui::import_export::Message::ExportOpp),
                                window,
                                cx,
                            );
                        })),
                ),
        )
        .child(
            div()
                .id("rail-media-bin")
                .test_support()
                .px(px(8.))
                .py(px(6.))
                .border_t_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .flex()
                .flex_col()
                .gap(px(2.))
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(Message::ToggleMediaBin, window, cx);
                }))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("MEDIA BIN"),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(if w.shell.media_bin_open {
                            format!("{} media items", w.library.assets.len())
                        } else {
                            "No media items".to_string()
                        }),
                ),
        )
        .into_any_element()
}

/// Presentations list with right-click menu.
fn presentations_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let query = w.shell.search_query.trim().to_lowercase();
    let filtered: Vec<_> = w
        .editor
        .presentations
        .iter()
        .filter(|pres| query.is_empty() || pres.name.to_lowercase().contains(&query))
        .cloned()
        .collect();
    let mut list = div()
        .id("presentations-list")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    if filtered.is_empty() {
        list = list.child(crate::ui::components::empty_state("No presentations yet"));
    }
    for pres in filtered {
        let id = pres.id.clone();
        let name = pres.name.clone();
        let count = pres.slides.len();
        let active = w
            .editor
            .editing
            .as_ref()
            .is_some_and(|editing| editing.id == id);
        let row_id = SharedString::from(format!("rail-presentation-{id}"));
        let id_click = id.clone();
        let id_right = id.clone();
        list = list.child(
            div()
                .id(row_id.clone())
                .test_support()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(1.))
                .px(px(8.))
                .py(px(6.))
                .rounded(px(4.))
                .bg(if active {
                    theme::c(theme::BG_ACTIVE)
                } else {
                    theme::c(theme::TRANSPARENT)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::OpenPresentation(id_click.clone()), window, cx);
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |w, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        w.shell.rail_cursor_pos = Some(Point::new(
                            f32::from(event.position.x),
                            f32::from(event.position.y),
                        ));
                        w.handle_message(
                            Message::ShowRailContextMenu(
                                crate::ui::messages::RailContextTarget::Presentation(
                                    id_right.clone(),
                                ),
                            ),
                            window,
                            cx,
                        );
                    }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!(
                            "{count} slide{}",
                            if count == 1 { "" } else { "s" }
                        )),
                ),
        );
    }
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(8.))
                .py(px(6.))
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("PRESENTATIONS"),
        )
        .child(list)
        .into_any_element()
}

/// Playlists list.
fn playlist_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    crate::ui::playlist::rail(w, cx)
}

/// Library list.
fn library_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    crate::ui::library::view(w, cx).into_any_element()
}

/// Songs list.
fn songs_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    crate::ui::songs::rail(w, cx)
}

/// Bible list.
fn bible_rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    crate::ui::bible::rail(w, cx)
}

/// The centre column: edit canvas + toolbar + slide list + timeline in
/// Edit mode; the header, live slide grid, bottom transition bar and
/// timeline in Show mode.
fn center(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let timeline = crate::ui::timeline::view(w, cx).into_any_element();
    match w.shell.current_mode {
        ViewMode::Edit => {
            let canvas = canvas_area(w, cx);
            let slide_list = slide_list(w, cx);
            let toolbar = crate::ui::shell::insert_bar::view(w, cx).into_any_element();
            let title = match &w.editor.editing {
                Some(pres) => pres.name.clone(),
                None => "No Presentation Selected".to_string(),
            };
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .bg(theme::c(theme::BG_DARKEST))
                .child(center_header(title))
                .child(canvas)
                .child(toolbar)
                .child(slide_list)
                .child(timeline)
                .into_any_element()
        }
        ViewMode::Show if w.presenting.stage_display_active => {
            crate::ui::stage::stage_view(w, cx).into_any_element()
        }
        ViewMode::Show => {
            let grid = match w.presenting.slide_view_mode {
                crate::ui::messages::SlideViewMode::Easy => show_easy_list(w, cx),
                crate::ui::messages::SlideViewMode::Grid => show_slide_grid(w, cx),
                crate::ui::messages::SlideViewMode::Table => show_table_rows(w, cx),
            };
            let bottom = show_bottom_bar(w, cx);
            let title = match &w.presenting.presentation {
                Some(pres) => pres.name.clone(),
                None => "No Presentation Selected".to_string(),
            };
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .bg(theme::c(theme::BG_DARKEST))
                .child(center_header(title))
                .child(grid)
                .child(bottom)
                .child(timeline)
                .into_any_element()
        }
    }
}

/// The slim title above the canvas / grid.
fn center_header(title: String) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .px(px(12.))
        .py(px(6.))
        .border_b_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .child(title)
        .into_any_element()
}

/// The Show-mode bottom bar: the global transition, the slide view mode
/// and the grid column count.
fn show_bottom_bar(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    use crate::domain::Transition;
    let current = w.presenting.global_transition;
    let trans_active = |label: &Transition| -> bool { current == *label };
    let duration_of = |t: &Transition| -> u64 {
        match t {
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
    };
    let duration_active =
        |ms: u64| -> bool { duration_of(&current) == ms && current != Transition::Cut };
    let mode = w.presenting.slide_view_mode;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(10.))
        .py(px(6.))
        .bg(theme::c(theme::BG_DARK))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Transition"),
        )
        .child(transition_btn(
            "show-trans-cut",
            "Cut",
            trans_active(&Transition::Cut),
            Transition::Cut,
            cx,
        ))
        .child(transition_btn(
            "show-trans-300",
            "0.3s",
            duration_active(300),
            Transition::Fade { duration_ms: 300 },
            cx,
        ))
        .child(transition_btn(
            "show-trans-500",
            "0.5s",
            duration_active(500),
            Transition::Fade { duration_ms: 500 },
            cx,
        ))
        .child(transition_btn(
            "show-trans-1000",
            "1.0s",
            duration_active(1000),
            Transition::Fade { duration_ms: 1000 },
            cx,
        ))
        .child(div().w(px(12.)))
        .child(view_mode_btn(
            "show-view-easy",
            "Easy",
            mode == crate::ui::messages::SlideViewMode::Easy,
            crate::ui::messages::SlideViewMode::Easy,
            cx,
        ))
        .child(view_mode_btn(
            "show-view-grid",
            "Grid",
            mode == crate::ui::messages::SlideViewMode::Grid,
            crate::ui::messages::SlideViewMode::Grid,
            cx,
        ))
        .child(view_mode_btn(
            "show-view-table",
            "Table",
            mode == crate::ui::messages::SlideViewMode::Table,
            crate::ui::messages::SlideViewMode::Table,
            cx,
        ))
        .child(div().w(px(12.)))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("{} cols", w.presenting.slide_grid_cols)),
        )
        .child(cols_btn(
            "show-cols-2",
            "2",
            2,
            w.presenting.slide_grid_cols,
            cx,
        ))
        .child(cols_btn(
            "show-cols-3",
            "3",
            3,
            w.presenting.slide_grid_cols,
            cx,
        ))
        .child(cols_btn(
            "show-cols-4",
            "4",
            4,
            w.presenting.slide_grid_cols,
            cx,
        ))
        .child(cols_btn(
            "show-cols-6",
            "6",
            6,
            w.presenting.slide_grid_cols,
            cx,
        ))
        .into_any_element()
}

/// One global-transition preset button.
fn transition_btn(
    id: &'static str,
    label: &'static str,
    active: bool,
    value: crate::domain::Transition,
    cx: &mut Context<MainWindow>,
) -> Button {
    let mut btn = Button::new(id).label(label).xsmall();
    if active {
        btn = btn.primary();
    }
    btn.on_click(cx.listener(move |w, _, window, cx| {
        w.handle_message(Message::SetGlobalTransition(value), window, cx);
    }))
}

/// One slide-view-mode button.
fn view_mode_btn(
    id: &'static str,
    label: &'static str,
    active: bool,
    value: crate::ui::messages::SlideViewMode,
    cx: &mut Context<MainWindow>,
) -> Button {
    let mut btn = Button::new(id).label(label).xsmall();
    if active {
        btn = btn.primary();
    }
    btn.on_click(cx.listener(move |w, _, window, cx| {
        w.handle_message(Message::SetSlideViewMode(value), window, cx);
    }))
}

/// One grid column count button.
fn cols_btn(
    id: &'static str,
    label: &'static str,
    value: usize,
    current: usize,
    cx: &mut Context<MainWindow>,
) -> Button {
    let mut btn = Button::new(id).label(label).xsmall();
    if value == current {
        btn = btn.primary();
    }
    btn.on_click(cx.listener(move |w, _, window, cx| {
        w.handle_message(Message::SetSlideGridCols(value), window, cx);
    }))
}

/// The Show-mode slide grid: one tile per slide, click to make live,
/// right-click for the slide menu.
fn show_slide_grid(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(pres) = w.presenting.presentation.clone() else {
        return div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Select a presentation from the Library to load slides")
            .into_any_element();
    };
    let reference = w.app_config.output.width as f32;
    let live_index = w.presenting.slide_index;
    let mut grid = div()
        .id("show-slide-grid")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_start()
        .content_start()
        .gap(px(8.))
        .p(px(12.))
        .overflow_y_scroll();
    for (index, slide) in pres.slides.iter().enumerate() {
        let live = index == live_index;
        let preview = crate::ui::canvas::slide_preview(
            slide,
            gpui_kit::size(gpui_kit::px(180.0), gpui_kit::px(101.0)),
            reference,
            w.video.frame.clone(),
        );
        grid = grid.child(
            div()
                .id(SharedString::from(format!("slide-tile-{index}")))
                .test_support()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .w(px(192.))
                .h(px(140.))
                .gap(px(4.))
                .p(px(6.))
                .rounded(px(4.))
                .border_1()
                .border_color(if live {
                    theme::c(theme::LIVE_GREEN)
                } else {
                    theme::c(theme::BORDER_PANEL)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::PresentingSelectSlide(index), window, cx);
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |w, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        w.presenting.slide_context_pos = Some(Point::new(
                            f32::from(event.position.x),
                            f32::from(event.position.y),
                        ));
                        w.handle_message(Message::ShowSlideContextMenu(index), window, cx);
                    }),
                )
                .child(preview)
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("Slide {}", index + 1)),
                ),
        );
    }
    grid.into_any_element()
}

/// One transition rendered as the label the operator reads.
fn transition_label(transition: &crate::domain::Transition) -> String {
    match transition {
        crate::domain::Transition::Cut => String::from("Cut"),
        t => {
            let name = match t {
                crate::domain::Transition::Cut => "",
                crate::domain::Transition::Fade { .. } => "Fade",
                crate::domain::Transition::Slide { .. } => "Slide",
                crate::domain::Transition::Dissolve { .. } => "Dissolve",
                crate::domain::Transition::Push { .. } => "Push",
                crate::domain::Transition::Zoom { .. } => "Zoom",
                crate::domain::Transition::Flip { .. } => "Flip",
                crate::domain::Transition::Clock { .. } => "Clock",
                crate::domain::Transition::Wipe { .. } => "Wipe",
            };
            format!("{name} {:.1}s", t.duration_ms() as f64 / 1000.0)
        }
    }
}

/// The Easy slide list: one row per slide for quick picking.
fn show_easy_list(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(pres) = w.presenting.presentation.clone() else {
        return show_empty_slide_view();
    };
    let reference = w.app_config.output.width as f32;
    let live_index = w.presenting.slide_index;
    let mut list = div()
        .id("show-slide-grid")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(12.))
        .overflow_y_scroll();
    for (index, slide) in pres.slides.iter().enumerate() {
        let live = index == live_index;
        let preview = crate::ui::canvas::slide_preview(
            slide,
            gpui_kit::size(gpui_kit::px(112.0), gpui_kit::px(63.0)),
            reference,
            w.video.frame.clone(),
        );
        let group = slide.group.clone().unwrap_or_default();
        let text = slide
            .editable_text(None)
            .map(|(text, _)| text.lines().next().unwrap_or_default().to_string())
            .unwrap_or_default();
        list = list.child(
            div()
                .id(SharedString::from(format!("slide-tile-{index}")))
                .test_support()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.))
                .p(px(6.))
                .rounded(px(4.))
                .border_1()
                .border_color(if live {
                    theme::c(theme::LIVE_GREEN)
                } else {
                    theme::c(theme::BORDER_PANEL)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::PresentingSelectSlide(index), window, cx);
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |w, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        w.presenting.slide_context_pos = Some(Point::new(
                            f32::from(event.position.x),
                            f32::from(event.position.y),
                        ));
                        w.handle_message(Message::ShowSlideContextMenu(index), window, cx);
                    }),
                )
                .child(preview)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_PRIMARY))
                                .child(format!("Slide {}", index + 1)),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .child(format!("{group} · {text}")),
                        ),
                ),
        );
    }
    list.into_any_element()
}

/// The Table slide list: slide number, group, text, transition and cues.
fn show_table_rows(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(pres) = w.presenting.presentation.clone() else {
        return show_empty_slide_view();
    };
    let live_index = w.presenting.slide_index;
    let mut table = div()
        .id("show-slide-grid")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .p(px(12.))
        .overflow_y_scroll()
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(10.))
                .px(px(8.))
                .py(px(4.))
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(table_cell("Slide", px(44.)))
                .child(table_cell("Group", px(90.)))
                .child(table_cell("Text", px(260.)))
                .child(table_cell("Transition", px(90.)))
                .child(table_cell("Cues", px(44.))),
        );
    for (index, slide) in pres.slides.iter().enumerate() {
        let live = index == live_index;
        let group = slide.group.clone().unwrap_or_default();
        let text = slide
            .editable_text(None)
            .map(|(text, _)| text.lines().next().unwrap_or_default().to_string())
            .unwrap_or_default();
        table = table.child(
            div()
                .id(SharedString::from(format!("slide-tile-{index}")))
                .test_support()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.))
                .p(px(6.))
                .rounded(px(4.))
                .border_1()
                .border_color(if live {
                    theme::c(theme::LIVE_GREEN)
                } else {
                    theme::c(theme::TRANSPARENT)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::PresentingSelectSlide(index), window, cx);
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |w, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        w.presenting.slide_context_pos = Some(Point::new(
                            f32::from(event.position.x),
                            f32::from(event.position.y),
                        ));
                        w.handle_message(Message::ShowSlideContextMenu(index), window, cx);
                    }),
                )
                .child(table_cell(format!("{}", index + 1), px(44.)))
                .child(table_cell(group, px(90.)))
                .child(table_cell(text, px(260.)))
                .child(table_cell(transition_label(&slide.transition), px(90.)))
                .child(table_cell(format!("{}", slide.cues.len()), px(44.))),
        );
    }
    table.into_any_element()
}

/// One table cell.
fn table_cell(text: impl Into<SharedString>, width: gpui_kit::Pixels) -> AnyElement {
    let text: SharedString = text.into();
    div()
        .w(width)
        .flex_shrink_0()
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .child(text)
        .into_any_element()
}

/// The empty state when no presentation is open.
fn show_empty_slide_view() -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child("Select a presentation from the Library to load slides")
        .into_any_element()
}

/// The edit canvas.
fn canvas_area(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(slide) = w.get_current_slide().cloned() else {
        return div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Open a presentation to edit a slide")
            .into_any_element();
    };
    let reference = w.app_config.output.width as f32;
    let editing = w.canvas.editing.map(|i| (i, w.canvas_text_input.clone()));
    let guides = CanvasGuides {
        slide_guides: w.canvas.slide_guides,
        theme_guides: w.canvas.theme_guides,
        show_rulers: w.canvas.show_rulers,
    };
    let size = if w.canvas.actual_size {
        let width = w.app_config.output.width as f32;
        let height = w.app_config.output.height as f32;
        gpui_kit::size(gpui_kit::px(width), gpui_kit::px(height))
    } else {
        gpui_kit::size(
            gpui_kit::px(960.0 * w.canvas.zoom),
            gpui_kit::px(540.0 * w.canvas.zoom),
        )
    };
    let slide_el = crate::ui::canvas::slide_element(
        &slide,
        size,
        reference,
        CanvasMode::Edit,
        w.layer.selected_index,
        w.video.frame.clone(),
        editing,
        guides,
        cx,
    );
    // Center the slide on the canvas workspace, so the canvas fills the
    // centre column the way the reference editor does instead of hugging
    // the top-left. The wrapper does not clip, so a slide wider than the
    // column still shows its handles.
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .child(slide_el)
        .into_any_element()
}

/// The slide list and the object row under the canvas.
fn slide_list(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(pres) = w.editor.editing.clone() else {
        return div().h(px(120.)).into_any_element();
    };
    let reference = w.app_config.output.width as f32;
    let mut col = div()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARK))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL));
    // Slide row: add button, then one card per slide with its up/down/
    // duplicate/delete controls.
    let mut slide_row = div()
        .id("edit-slide-row")
        .h(px(120.))
        .flex()
        .flex_row()
        .gap(px(6.))
        .p(px(6.))
        .overflow_x_scroll();
    slide_row = slide_row.child(
        Button::new("edit-add-slide")
            .label("+ Slide")
            .ghost()
            .small()
            .on_click(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Slides(crate::ui::slides::Message::AddSlide),
                    window,
                    cx,
                );
            })),
    );
    for (index, slide) in pres.slides.iter().enumerate() {
        let selected = w.editor.selected_slide_index == Some(index);
        let live = w.shell.current_mode == ViewMode::Show && w.presenting.slide_index == index;
        let border = if live {
            theme::c(theme::LIVE_GREEN)
        } else if selected {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        };
        let preview = crate::ui::canvas::slide_preview(
            slide,
            gpui_kit::size(gpui_kit::px(140.0), gpui_kit::px(79.0)),
            reference,
            w.video.frame.clone(),
        );
        let slide_id = slide.id.clone();
        slide_row = slide_row.child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .p(px(2.))
                .rounded(px(4.))
                .border_1()
                .border_color(border)
                .child(
                    div()
                        .id(SharedString::from(format!("edit-slide-{index}")))
                        .test_support()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(crate::ui::slides::Message::SelectSlide(index)),
                                window,
                                cx,
                            );
                        }))
                        .child(preview),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(2.))
                        .child(
                            Button::new(SharedString::from(format!("edit-slide-up-{index}")))
                                .label("↑")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    w.handle_message(
                                        Message::Slides(crate::ui::slides::Message::MoveSlideUp(
                                            index,
                                        )),
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("edit-slide-down-{index}")))
                                .label("↓")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    w.handle_message(
                                        Message::Slides(crate::ui::slides::Message::MoveSlideDown(
                                            index,
                                        )),
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("edit-slide-dup-{index}")))
                                .label("⧉")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    w.handle_message(
                                        Message::Slides(
                                            crate::ui::slides::Message::DuplicateSlide(index),
                                        ),
                                        window,
                                        cx,
                                    );
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("edit-slide-del-{index}")))
                                .label("×")
                                .ghost()
                                .xsmall()
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    w.handle_message(
                                        Message::Slides(crate::ui::slides::Message::DeleteSlide(
                                            slide_id.clone(),
                                        )),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                ),
        );
    }
    col = col.child(slide_row);
    // Object row: one row per object plus the add controls, so the slide's
    // contents are reachable without opening the inspector.
    let current = w.get_current_slide().cloned();
    let mut object_row = div()
        .id("edit-object-row")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(6.))
        .py(px(4.))
        .overflow_x_scroll()
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL));
    object_row = object_row
        .child(
            Button::new("edit-obj-add-text")
                .label("+ Text")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(crate::ui::layers::Message::AddTextLayer),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new("edit-obj-add-shape")
                .label("+ Shape")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(crate::ui::layers::Message::AddShapeLayer(
                            crate::domain::ShapeType::Rectangle,
                        )),
                        window,
                        cx,
                    );
                })),
        );
    if let Some(slide) = current {
        for (index, object) in slide.layers.iter().enumerate() {
            let selected = w.layer.selected_index == Some(index);
            object_row = object_row.child(
                div()
                    .id(SharedString::from(format!("edit-obj-{index}-row")))
                    .test_support()
                    .flex()
                    .items_center()
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if selected {
                        theme::c(theme::ACCENT_ORANGE)
                    } else {
                        theme::c(theme::TRANSPARENT)
                    })
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            Message::Layers(crate::ui::layers::Message::SelectLayer(Some(index))),
                            window,
                            cx,
                        );
                    }))
                    .child(object.display_name()),
            );
        }
    }
    col = col.child(object_row);
    col.into_any_element()
}

/// The slide context menu: backdrop plus the slide's own entries.
fn slide_context_menu(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(index) = w.presenting.slide_context_index else {
        return div().into_any_element();
    };
    let pos = w
        .presenting
        .slide_context_pos
        .unwrap_or(gpui_kit::Point { x: 0.0, y: 0.0 });
    let menu = div()
        .id("slide-context-menu")
        .test_support()
        .absolute()
        .left(px(pos.x))
        .top(px(pos.y))
        .w(px(180.))
        .flex()
        .flex_col()
        .p(px(4.))
        .rounded(px(6.))
        .border_1()
        .border_color(theme::c(theme::BORDER_STRONG))
        .bg(theme::c(theme::BG_PANEL))
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            Button::new("ctx-edit-slide")
                .label("Edit Slide")
                .ghost()
                .small()
                .w_full()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::EditSlide(index), window, cx);
                })),
        )
        .child(
            Button::new("ctx-go-slide")
                .label("Go to Slide")
                .ghost()
                .small()
                .w_full()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.presenting.slide_context_index = None;
                    w.handle_message(Message::PresentingSelectSlide(index), window, cx);
                })),
        );
    // The backdrop comes first and the menu above it, so a click on the
    // menu reaches the menu and not the backdrop that closes it.
    div()
        .absolute()
        .inset_0()
        .child(div().absolute().inset_0().on_mouse_down(
            MouseButton::Left,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::HideSlideContextMenu, window, cx);
            }),
        ))
        .child(menu)
        .into_any_element()
}

/// The rail context menu for the presentation under the cursor.
fn rail_context_menu(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let Some(target) = w.shell.rail_context_target.clone() else {
        return div().into_any_element();
    };
    let crate::ui::messages::RailContextTarget::Presentation(id) = target else {
        return div().into_any_element();
    };
    let pos = w
        .shell
        .rail_cursor_pos
        .unwrap_or(gpui_kit::Point { x: 240.0, y: 100.0 });
    let id_edit = id.clone();
    let id_dup = id.clone();
    let menu = div()
        .id("rail-context-menu")
        .test_support()
        .absolute()
        .left(px(pos.x))
        .top(px(pos.y))
        .w(px(200.))
        .flex()
        .flex_col()
        .p(px(4.))
        .rounded(px(6.))
        .border_1()
        .border_color(theme::c(theme::BORDER_STRONG))
        .bg(theme::c(theme::BG_PANEL))
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            Button::new("rail-ctx-edit")
                .label("Edit Presentation")
                .ghost()
                .small()
                .w_full()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::EditPresentation(id_edit.clone()), window, cx);
                })),
        )
        .child(
            Button::new("rail-ctx-duplicate")
                .label("Duplicate")
                .ghost()
                .small()
                .w_full()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::DuplicatePresentation(id_dup.clone()), window, cx);
                })),
        )
        .child(
            Button::new("rail-ctx-delete")
                .label("Delete…")
                .ghost()
                .small()
                .w_full()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::DeletePresentationClicked(id.clone()), window, cx);
                })),
        );
    // The backdrop comes first and the menu above it, so a click on the
    // menu reaches the menu and not the backdrop that closes it.
    div()
        .absolute()
        .inset_0()
        .child(div().absolute().inset_0().on_mouse_down(
            MouseButton::Left,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::HideRailContextMenu, window, cx);
            }),
        ))
        .child(menu)
        .into_any_element()
}

/// The right dock: inspector in Edit mode, show controls in Show mode.
fn right_dock(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    match w.shell.current_mode {
        ViewMode::Edit => crate::ui::inspector::inspector_panel(w, cx).into_any_element(),
        ViewMode::Show => show_dock(w, cx),
    }
}

/// The Show-mode dock: the Show Controls panel plus the panels the
/// operator switches between during a show.
///
/// Like the reference, the Show tab bar lives inside the Show Controls
/// panel, under the transport; the other panels carry it across the top so
/// the operator can always switch back.
fn show_dock(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let tab = w.shell.right_dock_tab;
    let body: AnyElement = match tab {
        RightDockTab::ShowControls => show_controls_panel(w, cx),
        RightDockTab::Props => crate::ui::props::props_panel(w, cx).into_any_element(),
        RightDockTab::Messages => messages_panel(w, cx),
        RightDockTab::Timers => timers_panel(w, cx),
        RightDockTab::Audio => crate::ui::audio::audio_panel(w, cx).into_any_element(),
        RightDockTab::Triggers => crate::ui::triggers::dock_view(w, cx).into_any_element(),
    };
    match tab {
        RightDockTab::ShowControls => body,
        _ => div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::c(theme::BG_PANEL))
            .child(dock_tab_bar(w, cx))
            .child(body)
            .into_any_element(),
    }
}

/// The Show-mode dock's tab bar: Show, Props, Messages, Timers, Audio,
/// Triggers. In the Show Controls panel it sits under the transport; in the
/// other panels it sits across the top.
fn dock_tab_bar(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    use crate::ui::components::{tab_bar as make_tab_bar, tab_btn};
    let tab = w.shell.right_dock_tab;
    let tabs = [
        (RightDockTab::ShowControls, "tab-show", "Show"),
        (RightDockTab::Props, "tab-props", "Props"),
        (RightDockTab::Messages, "tab-messages", "Messages"),
        (RightDockTab::Timers, "tab-timers", "Timers"),
        (RightDockTab::Audio, "tab-audio", "Audio"),
        (RightDockTab::Triggers, "tab-triggers", "Triggers"),
    ];
    let mut buttons = Vec::new();
    for (value, id, label) in tabs {
        buttons.push(
            tab_btn(
                id,
                label,
                tab == value,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::SelectRightDockTab(value), window, cx);
                }),
            )
            .into_any_element(),
        );
    }
    make_tab_bar(buttons).into_any_element()
}

/// One dock button: a labelled switch that dispatches a root message.
fn dock_button(
    id: &'static str,
    label: &'static str,
    message: Message,
    cx: &mut Context<MainWindow>,
) -> Button {
    Button::new(id)
        .label(label)
        .small()
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(message.clone(), window, cx);
        }))
}

/// The Show Controls panel: preview, look, clears, layers, transport,
/// the live alert and the links to the other show panels.
///
/// The preview and the Go transport come first: they are what the operator
/// works from during a show. The panel scrolls, so a control below the
/// fold is still reachable by scrolling; the test's on-screen check feeds
/// the headless window big enough to show the panel's controls together.
fn show_controls_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    let preview_name = match &w.presenting.presentation {
        Some(pres) => pres.name.clone(),
        None => "No presentation".to_string(),
    };
    let slide_label = match &w.presenting.presentation {
        Some(pres) => {
            let slides = pres.slides.len();
            let group = pres
                .slides
                .get(w.presenting.slide_index)
                .and_then(|s| s.group.clone())
                .unwrap_or_default();
            if group.is_empty() {
                format!("Slide {} of {slides}", w.presenting.slide_index + 1)
            } else {
                format!(
                    "Slide {} of {slides} · {group}",
                    w.presenting.slide_index + 1
                )
            }
        }
        None => "Slide 0 —".to_string(),
    };
    let screen_id = match w.presenting.preview_screen_target.as_str() {
        "stream" => crate::ui::props::ndi_screen_id(w),
        "stage" => crate::ui::stage::STAGE_SCREEN_ID.to_string(),
        _ => crate::ui::output::AUDIENCE_SCREEN_ID.to_string(),
    };
    let frame = crate::ui::compositor::frame_for_screen(w, &screen_id);
    let preview_body: AnyElement = if w.output.black_screen {
        div()
            .id("show-preview-black")
            .test_support()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .text_size(px(14.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("BLACK")
            .into_any_element()
    } else if let Some(frame) = frame {
        div()
            .id("show-preview-frame")
            .test_support()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(frame.object_fit(ObjectFit::Contain).size_full())
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme::c(theme::TEXT_PRIMARY))
                    .child(preview_name),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(slide_label),
            )
            .into_any_element()
    };
    let look_name = w
        .props
        .manager
        .looks
        .iter()
        .find(|look| Some(&look.id) == w.props.manager.active_look_id.as_ref())
        .map(|look| look.name.clone())
        .unwrap_or_else(|| {
            w.props
                .manager
                .looks
                .first()
                .map(|look| look.name.clone())
                .unwrap_or_else(|| "Default".to_string())
        });
    // Layer dots: slide/media track the live layer switches; the others
    // are on unless the operator cleared them, shown by the clear buttons.
    let slide_on = w.presenting.slide_layer_active;
    let media_on = w.presenting.media_layer_active;
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(8.))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("PREVIEW"),
                )
                .child(div().flex_1())
                .child(preview_target_btn(
                    "preview-audience",
                    "Audience",
                    &w.presenting.preview_screen_target,
                    cx,
                ))
                .child(preview_target_btn(
                    "preview-stream",
                    "Stream",
                    &w.presenting.preview_screen_target,
                    cx,
                ))
                .child(preview_target_btn(
                    "preview-stage",
                    "Stage",
                    &w.presenting.preview_screen_target,
                    cx,
                )),
        )
        .child(
            div()
                .id("show-preview")
                .test_support()
                .w_full()
                .h(px(120.))
                .flex()
                .overflow_hidden()
                .items_center()
                .justify_center()
                .bg(theme::c(theme::BG_DARKEST))
                .rounded(px(4.))
                .border_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .child(preview_body),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(format!("LOOK: {look_name}")),
                )
                .child(
                    Button::new("ctrl-matrix")
                        .label("Matrix")
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::ToggleLooksMatrixModal, window, cx);
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.))
                .child(
                    Button::new("dock-clear-all")
                        .label("Clear All")
                        .small()
                        .flex_1()
                        .primary()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::ClearAll, window, cx);
                        })),
                )
                .child(
                    Button::new("dock-black")
                        .label("Black")
                        .small()
                        .flex_1()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::ToggleOutputBlackScreen, window, cx);
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_start()
                .content_start()
                .gap(px(6.))
                .child(layer_btn(
                    "dock-clear-slide",
                    "Slide",
                    slide_on,
                    cx,
                    Message::ClearSlide,
                ))
                .child(layer_btn(
                    "dock-clear-media",
                    "Media",
                    media_on,
                    cx,
                    Message::ClearMedia,
                ))
                .child(layer_btn(
                    "dock-clear-props",
                    "Props",
                    true,
                    cx,
                    Message::ClearProps,
                ))
                .child(layer_btn(
                    "dock-clear-messages",
                    "Messages",
                    true,
                    cx,
                    Message::ClearMessages,
                ))
                .child(layer_btn(
                    "dock-clear-audio",
                    "Audio",
                    true,
                    cx,
                    Message::ClearAudio,
                ))
                .child(
                    Button::new("dock-clear-live")
                        .label("Live")
                        .small()
                        .w(px(108.))
                        .on_click(cx.listener(|w, _, _, cx| {
                            crate::ui::ndi::toggle_ndi_black_screen(w, cx);
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    Button::new("dock-prev")
                        .label("Prev")
                        .small()
                        .h(px(32.))
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::PresentingPrevSlide, window, cx);
                        })),
                )
                .child(
                    Button::new("dock-go")
                        .label("Go")
                        .small()
                        .h(px(32.))
                        .flex_1()
                        .primary()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::PresentingNextSlide, window, cx);
                        })),
                ),
        )
        .child(dock_tab_bar(w, cx))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("ALERT"),
                )
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&w.alert_input.clone()).aria_label("Live alert message")),
                )
                .child(
                    Button::new("ctrl-send-alert")
                        .label("Send")
                        .small()
                        .primary()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::SendLiveMessage(String::new()), window, cx);
                        })),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(dock_link(
                    "ctrl-matrix-link",
                    "Looks & Screens Matrix",
                    Message::ToggleLooksMatrixModal,
                    cx,
                ))
                .child(dock_link(
                    "ctrl-output",
                    "Open Output Screen",
                    Message::ToggleOutputSettings,
                    cx,
                ))
                .child(dock_link(
                    "ctrl-stage-display",
                    "Toggle Stage Display",
                    Message::ToggleStageDisplay,
                    cx,
                ))
                .child(dock_link(
                    "ctrl-stage-window",
                    "Toggle Stage Window",
                    Message::ToggleStageWindow,
                    cx,
                ))
                .child(dock_link("ctrl-ndi", "NDI", Message::ToggleNdi, cx))
                .child(dock_link(
                    "ctrl-triggers-panel",
                    "Triggers",
                    Message::ToggleTriggersPanel,
                    cx,
                ))
                .child(dock_link(
                    "ctrl-recording-panel",
                    "Recording",
                    Message::ToggleRecordingPanel,
                    cx,
                )),
        )
        .into_any_element()
}

/// A full-width quiet row in the Show Controls panel: under the alert, the
/// way the reference lists the show's links one per line.
fn dock_link(
    id: &'static str,
    label: &'static str,
    message: Message,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    div()
        .id(id.to_string())
        .test_support()
        .w_full()
        .py(px(5.))
        .px(px(4.))
        .rounded(px(4.))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(message.clone(), window, cx);
        }))
        .child(label)
        .into_any_element()
}

/// One preview-screen selector: Audience / Stream / Stage, with a green
/// dot on the active screen, like the reference.
fn preview_target_btn(
    id: &'static str,
    label: &'static str,
    active_target: &str,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let target = match id {
        "preview-audience" => "main",
        "preview-stream" => "stream",
        _ => "stage",
    };
    let active = active_target == target || (label == "Audience" && active_target == "main");
    let dot_color = if active {
        theme::c(theme::LIVE_GREEN)
    } else {
        theme::c(theme::TEXT_MUTED)
    };
    div()
        .id(id.to_string())
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(7.))
        .py(px(4.))
        .rounded(px(4.))
        .border_1()
        .border_color(if active {
            theme::c(theme::LIVE_GREEN)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .text_size(px(10.))
        .text_color(if active {
            theme::c(theme::TEXT_PRIMARY)
        } else {
            theme::c(theme::TEXT_MUTED)
        })
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(Message::SelectPreviewScreen(target.to_string()), window, cx);
        }))
        .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(dot_color))
        .child(label)
        .into_any_element()
}

/// One layer toggle with a status dot: on = on the output, off = cleared.
fn layer_btn(
    id: &'static str,
    label: &'static str,
    on: bool,
    cx: &mut Context<MainWindow>,
    message: Message,
) -> AnyElement {
    let dot_color = if on {
        theme::c(theme::ACCENT_ORANGE)
    } else {
        theme::c(theme::TEXT_MUTED)
    };
    div()
        .id(id.to_string())
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(5.))
        .w(px(108.))
        .px(px(6.))
        .py(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(if on {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(message.clone(), window, cx);
        }))
        .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(dot_color))
        .child(label)
        .into_any_element()
}

/// The Messages panel: the live alert and its dismiss.
fn messages_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(8.))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&w.alert_input.clone()).aria_label("Live alert message")),
                )
                .child(
                    Button::new("messages-send")
                        .label("Send")
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(Message::SendLiveMessage(String::new()), window, cx);
                        })),
                ),
        )
        .child(dock_button(
            "messages-dismiss",
            "Dismiss",
            Message::DismissLiveMessage,
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.))
                .child(
                    div()
                        .id("announcement-input")
                        .test_support()
                        .flex_1()
                        .child(
                            Input::new(&w.announcement_input.clone()).aria_label("Announcement"),
                        ),
                )
                .child(
                    Button::new("announcement-send")
                        .label("Announce")
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            crate::ui::navigation::send_announcement(w, window, cx);
                        })),
                ),
        )
        .into_any_element()
}

/// The Timers panel: the elapsed timer's transport.
fn timers_panel(_w: &mut MainWindow, cx: &mut Context<MainWindow>) -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_row()
        .flex_wrap()
        .content_start()
        .gap(px(6.))
        .p(px(8.))
        .child(dock_button("timer-start", "Start", Message::StartTimer, cx))
        .child(dock_button("timer-stop", "Stop", Message::StopTimer, cx))
        .child(dock_button("timer-reset", "Reset", Message::ResetTimer, cx))
        .into_any_element()
}
