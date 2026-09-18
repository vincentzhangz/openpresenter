use crate::config::Config;
use crate::db::{Database, LibraryRepository, ThemeRepository};
use crate::domain::{Slide, Transition};
use crate::ui::messages::{Message, SidebarTab, ViewMode};
use crate::ui::state::{AudioState, BibleState, LayerState, ServiceState, SongState};
use crate::ui::{output_window, theme};
use iced::{
    Element, Length, Subscription, Task, event,
    keyboard::{self, key},
    window,
};
use std::sync::Arc;
use std::time::Duration;

type TriggerRx = Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::triggers::Action>>>;

#[derive(Clone)]
struct TriggerSubKey(TriggerRx);

impl std::hash::Hash for TriggerSubKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.0) as usize).hash(state);
    }
}

fn trigger_stream(key: &TriggerSubKey) -> iced::futures::stream::BoxStream<'static, Message> {
    use iced::futures::SinkExt as _;
    use iced::futures::StreamExt as _;
    let rx = Arc::clone(&key.0);
    iced::stream::channel(
        16,
        async move |mut output: iced::futures::channel::mpsc::Sender<Message>| {
            let mut guard = rx.lock().await;
            loop {
                match guard.recv().await {
                    Some(action) => {
                        let _ = output.send(Message::TriggerFired(action)).await;
                    }
                    None => std::future::pending::<()>().await,
                }
            }
        },
    )
    .boxed()
}

pub struct MainWindow {
    pub(crate) shell: crate::ui::state::ShellState,
    pub(crate) editor: crate::ui::state::EditorState,
    pub(crate) presenting: crate::ui::state::PresentingState,
    pub(crate) library: crate::ui::state::LibraryState,
    pub(crate) theme_state: crate::ui::state::ThemesState,
    pub(crate) ui: crate::ui::state::UiState,
    pub(crate) services: crate::services::Services,
    pub(crate) song: SongState,
    pub(crate) video: crate::ui::state::VideoState,
    pub(crate) service: ServiceState,
    pub(crate) main_window_id: window::Id,
    pub(crate) app_config: Config,
    pub(crate) layer: LayerState,
    pub(crate) bible: BibleState,
    pub(crate) audio: AudioState,
    pub(crate) output: crate::ui::state::OutputState,
    pub(crate) import_export: crate::ui::state::ImportExportState,
    pub(crate) props: crate::ui::state::PropsState,
    pub(crate) triggers: crate::ui::state::TriggersState,
    pub(crate) recording: crate::ui::state::RecordingFeatureState,
}

impl MainWindow {
    pub fn new(db: Arc<Database>, main_window_id: window::Id) -> (Self, Task<Message>) {
        let app_config = Config::load();
        let services = crate::services::Services::new(db.clone());
        let mut trigger_manager = crate::triggers::TriggerManager::default();
        let trigger_rx = std::sync::Arc::new(tokio::sync::Mutex::new(trigger_manager.subscribe()));
        let mut window = Self {
            shell: crate::ui::state::ShellState::default(),
            editor: crate::ui::state::EditorState::default(),
            presenting: crate::ui::state::PresentingState::default(),
            library: crate::ui::state::LibraryState::new(LibraryRepository::new(db.clone())),
            theme_state: crate::ui::state::ThemesState::new(ThemeRepository::new(db.clone())),
            ui: crate::ui::state::UiState::default(),
            song: SongState::new(crate::db::SongRepository::new(db.clone())),
            video: crate::ui::state::VideoState::default(),
            service: ServiceState::new(),
            main_window_id,
            layer: LayerState::default(),
            bible: BibleState::new(crate::db::BibleRepository::new(db.clone())),
            audio: AudioState::default(),
            output: crate::ui::state::OutputState::new(&app_config),
            app_config,
            import_export: crate::ui::state::ImportExportState::default(),
            props: crate::ui::state::PropsState::default(),
            triggers: crate::ui::state::TriggersState {
                manager: trigger_manager,
                rx: trigger_rx,
                panel_open: false,
                http_port_str: String::from("9090"),
                osc_port_str: String::from("9000"),
                new_macro_name: String::new(),
                macro_running_handles: std::collections::HashMap::new(),
                macro_running_ids: std::collections::HashSet::new(),
            },
            recording: crate::ui::state::RecordingFeatureState::default(),
            services,
        };
        window.load_presentations();
        window.load_songs();
        window.load_service_plans();
        window.load_bible_translations();
        window.load_themes();
        window.load_lib_assets();
        window.load_props();
        (window, Task::none())
    }

    pub(crate) fn props_path(&self) -> std::path::PathBuf {
        self.app_config
            .db_path
            .parent()
            .map(|p| p.join("props.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("props.json"))
    }

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

    pub(crate) fn save_props(&mut self) {
        let path = self.props_path();
        if let Err(e) = self.props.manager.save_to_file(&path) {
            self.set_error(format!("Failed to save props: {e}"));
        }
    }

    pub(crate) fn load_presentations(&mut self) {
        match self.services.presentations.list() {
            Ok(list) => self.editor.presentations = list,
            Err(e) => self.set_error(format!("Failed to load presentations: {e}")),
        }
    }

    pub(crate) fn load_songs(&mut self) {
        match self.song.repo.list_songs() {
            Ok(list) => self.song.songs = list,
            Err(e) => self.set_error(format!("Failed to load songs: {e}")),
        }
    }

    pub(crate) fn load_service_plans(&mut self) {
        match self.services.playlists.list() {
            Ok(list) => self.service.plans = list,
            Err(e) => self.set_error(format!("Failed to load service plans: {e}")),
        }
    }

    pub(crate) fn load_bible_translations(&mut self) {
        match self.bible.repo.list_translations() {
            Ok(list) => self.bible.translations = list,
            Err(e) => self.set_error(format!("Failed to load Bible translations: {e}")),
        }
    }

    pub(crate) fn load_song_for_editing(&mut self, song: crate::domain::Song) {
        self.song.edit_title = song.title.clone();
        self.song.edit_artist = song.artist.clone().unwrap_or_default();
        self.song.edit_copyright = song.copyright.clone().unwrap_or_default();
        self.song.edit_ccli = song.ccli_number.clone().unwrap_or_default();
        self.song.edit_key = song.key_signature.clone().unwrap_or_default();
        self.song.edit_bpm = song.bpm.map(|v| v.to_string()).unwrap_or_default();
        self.song.editing = Some(song);
    }

    pub(crate) fn load_lib_assets(&mut self) {
        match self.library.repo.list_assets() {
            Ok(list) => self.library.assets = list,
            Err(e) => self.set_error(format!("Failed to load library assets: {e}")),
        }
    }

    pub(crate) fn load_themes(&mut self) {
        match self.theme_state.repo.list_themes() {
            Ok(list) => self.theme_state.list = list,
            Err(e) => self.set_error(format!("Failed to load themes: {e}")),
        }
    }

    pub(crate) fn push_undo(&mut self) {
        if let Some(ref pres) = self.editor.editing {
            self.editor.undo_stack.push(pres.clone());
            if self.editor.undo_stack.len() > 50 {
                self.editor.undo_stack.remove(0);
            }
            self.editor.redo_stack.clear();
        }
    }

    /// Surface an error in the UI toast (replacing `eprintln!` paths).
    pub(crate) fn set_error(&mut self, msg: impl Into<String>) {
        self.ui.error_message = Some(msg.into());
    }

    /// Persist the given slide and reload the editing presentation on success.
    pub(crate) fn persist_slide(&mut self, slide: crate::domain::Slide) {
        if let Some(pres_id) = self.editor.editing.as_ref().map(|p| p.id.clone())
            && let Err(e) = self.services.presentations.update_slide(&pres_id, &slide)
        {
            self.set_error(format!("Failed to save slide: {e}"));
        }
    }

    pub(crate) fn load_slide_for_editing(&mut self) {
        let (text, font_size, transition_dur, group, notes) = if let Some(slide) =
            self.get_current_slide()
        {
            let dur = match slide.transition {
                Transition::Cut => String::from("500"),
                Transition::Fade { duration_ms }
                | Transition::Dissolve { duration_ms }
                | Transition::Slide { duration_ms }
                | Transition::Push { duration_ms, .. }
                | Transition::Zoom { duration_ms }
                | Transition::Flip { duration_ms }
                | Transition::Clock { duration_ms }
                | Transition::Wipe { duration_ms, .. } => duration_ms.to_string(),
            };
            let group = slide.group.clone().unwrap_or_default();
            let notes = slide.notes.clone().unwrap_or_default();
            if let crate::domain::SlideContent::Text {
                ref text,
                ref style,
            } = slide.content
            {
                (text.clone(), style.font_size, dur, group, notes)
            } else if let Some((txt, style)) = slide.layers.iter().find_map(|l| match &l.content {
                crate::domain::ObjectContent::Text { text, style, .. } => {
                    Some((text.clone(), style))
                }
                _ => None,
            }) {
                (txt, style.font_size, dur, group, notes)
            } else {
                (String::new(), 72.0, dur, group, notes)
            }
        } else {
            (
                String::new(),
                72.0,
                String::from("500"),
                String::new(),
                String::new(),
            )
        };
        self.editor.editing_slide_text = text;
        self.editor.editing_slide_font_size = font_size.to_string();
        self.editor.editing_transition_duration = transition_dur;
        self.editor.editing_group_label = group;
        self.editor.editing_slide_notes = notes;
        self.layer.selected_index = None;
        self.load_layer_for_editing();
    }

    pub(crate) fn load_layer_for_editing(&mut self) {
        use crate::domain::ObjectContent;
        let Some(idx) = self.layer.selected_index else {
            return;
        };
        #[allow(clippy::type_complexity)]
        let data: Option<(
            String,
            String,
            f32,
            f32,
            f32,
            f32,
            String,
            String,
            String,
            String,
            String,
            String,
        )> = self.get_current_slide().and_then(|slide| {
            let layer = slide.layers.get(idx)?;
            let (
                text,
                font_size,
                font_family,
                line_height,
                letter_spacing,
                glow_radius,
                text_stroke_width,
            ) = if let ObjectContent::Text {
                ref text,
                ref style,
                ..
            } = layer.content
            {
                (
                    text.clone(),
                    style.font_size.to_string(),
                    style.font_family.clone(),
                    format!("{:.2}", style.line_height_multiplier),
                    format!("{:.1}", style.letter_spacing),
                    format!("{:.1}", style.glow_radius),
                    format!("{:.1}", style.text_stroke_width),
                )
            } else {
                (
                    String::new(),
                    String::from("72"),
                    String::from("Arial"),
                    String::from("1.2"),
                    String::from("0"),
                    String::from("8"),
                    String::from("0"),
                )
            };
            let stroke_w = if let ObjectContent::Shape { stroke_width, .. } = layer.content {
                format!("{stroke_width}")
            } else {
                String::from("0")
            };
            Some((
                text,
                font_size,
                layer.position_x,
                layer.position_y,
                layer.width,
                layer.height,
                stroke_w,
                font_family,
                line_height,
                letter_spacing,
                glow_radius,
                text_stroke_width,
            ))
        });
        if let Some((text, font_size, px, py, w, h, sw, ff, lh, ls, gr, tsw)) = data {
            self.layer.text = text.clone();
            self.layer.font_size = font_size.clone();
            self.editor.editing_slide_text = text;
            self.editor.editing_slide_font_size = font_size;
            self.layer.pos_x = format!("{px:.3}");
            self.layer.pos_y = format!("{py:.3}");
            self.layer.width = format!("{w:.3}");
            self.layer.height = format!("{h:.3}");
            self.layer.stroke_width = sw;
            self.layer.font_family = ff;
            self.layer.line_height = lh;
            self.layer.letter_spacing = ls;
            self.layer.glow_radius = gr;
            self.layer.text_stroke_width = tsw;
        } else {
            self.layer.selected_index = None;
        }
    }

    pub(crate) fn get_current_slide(&self) -> Option<&Slide> {
        if let Some(index) = self.editor.selected_slide_index
            && let Some(ref pres) = self.editor.editing
        {
            return pres.slides.get(index);
        }
        None
    }

    pub(crate) fn get_current_slide_mut(&mut self) -> Option<&mut Slide> {
        if let Some(index) = self.editor.selected_slide_index
            && let Some(ref mut pres) = self.editor.editing
        {
            return pres.slides.get_mut(index);
        }
        None
    }
}

impl MainWindow {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => Task::none(),
            Message::EscapePressed => {
                if self.output.settings_open {
                    self.output.settings_open = false;
                    self.output.settings_status_message = None;
                    Task::none()
                } else if self.output.matrix_open {
                    self.output.matrix_open = false;
                    Task::none()
                } else {
                    crate::ui::slides::update(
                        self,
                        crate::ui::slides::Message::EndInlineTextEditing,
                    )
                }
            }
            Message::DismissError => {
                self.ui.error_message = None;
                Task::none()
            }
            Message::SwitchMode(mode) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                crate::ui::navigation::switch_mode(self, mode)
            }
            Message::SwitchInspectorTab(tab) => {
                crate::ui::navigation::switch_inspector_tab(self, tab)
            }
            Message::OpenTheme => crate::ui::navigation::open_theme(self),
            Message::OpenTextEditor => crate::ui::navigation::open_text_editor(self),
            Message::FocusSearch => {
                self.shell.sidebar_tab = SidebarTab::Presentations;
                crate::ui::polish::focus_search(self)
            }
            Message::SelectLeftSection(tab) => {
                self.shell.rail_context_target = None;
                self.shell.sidebar_tab = tab;
                Task::none()
            }
            Message::SelectRightDockTab(tab) => {
                self.shell.right_dock_tab = tab;
                Task::none()
            }
            Message::ToggleMediaBin => {
                self.shell.media_bin_open = !self.shell.media_bin_open;
                Task::none()
            }
            Message::ToggleEditMode => {
                let next = match self.shell.current_mode {
                    ViewMode::Edit => ViewMode::Show,
                    ViewMode::Show => ViewMode::Edit,
                };
                crate::ui::navigation::switch_mode(self, next)
            }
            Message::StartTimer => {
                self.presenting.timer_running = true;
                Task::none()
            }
            Message::StopTimer => {
                self.presenting.timer_running = false;
                Task::none()
            }
            Message::ResetTimer => {
                self.presenting.timer_running = false;
                self.presenting.timer_secs = 0;
                Task::none()
            }
            Message::BackToList => crate::ui::navigation::back_to_list(self),
            Message::Quit => crate::ui::navigation::quit(self),
            Message::SearchQueryChanged(q) => crate::ui::navigation::search_query_changed(self, q),
            Message::NewPresentationClicked => {
                crate::ui::navigation::new_presentation_clicked(self)
            }
            Message::CreatePresentation => crate::ui::navigation::create_presentation(self),
            Message::CancelNewPresentation => crate::ui::navigation::cancel_new_presentation(self),
            Message::NewPresentationNameChanged(name) => {
                crate::ui::navigation::new_presentation_name_changed(self, name)
            }
            Message::SelectPresentation(_) => Task::none(),
            Message::OpenPresentation(id) => {
                self.shell.rail_context_target = None;
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                crate::ui::presentations::open(self, id)
            }
            Message::RenamePresentation => crate::ui::presentations::rename(self),
            Message::RenamePresentationChanged(name) => {
                crate::ui::presentations::rename_changed(self, name)
            }
            Message::DeletePresentationClicked(id) => {
                self.shell.rail_context_target = None;
                crate::ui::presentations::delete_clicked(self, id)
            }
            Message::ConfirmDeletePresentation => crate::ui::presentations::confirm_delete(self),
            Message::CancelDelete => crate::ui::presentations::cancel_delete(self),
            Message::Slides(msg) => crate::ui::slides::update(self, msg),
            Message::ImageFilePicked(_) => Task::none(),
            Message::VideoFilePicked(_) => Task::none(),
            Message::VideoFrameTick => crate::ui::video::video_frame_tick(self),
            Message::Video(msg) => crate::ui::video::update(self, msg),
            Message::PresentingSelectSlide(i) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.slide_layer_active = true;
                self.presenting.media_layer_active = true;
                self.output.black_screen = false;
                crate::ui::presenter::select_slide(self, i)
            }
            Message::PresentingSelectLastSlide => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.slide_layer_active = true;
                self.presenting.media_layer_active = true;
                self.output.black_screen = false;
                let last = self
                    .presenting
                    .presentation
                    .as_ref()
                    .map(|p| p.slides.len().saturating_sub(1))
                    .unwrap_or(0);
                crate::ui::presenter::select_slide(self, last)
            }
            Message::PresentingNextSlide => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.slide_layer_active = true;
                self.presenting.media_layer_active = true;
                self.output.black_screen = false;
                crate::ui::presenter::next_slide(self)
            }
            Message::PresentingPrevSlide => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.slide_layer_active = true;
                self.presenting.media_layer_active = true;
                self.output.black_screen = false;
                crate::ui::presenter::prev_slide(self)
            }
            Message::ShowSlidesCursorMoved(p) => {
                if self.presenting.slide_context_index.is_none() {
                    self.presenting.slide_context_pos = Some(p);
                }
                Task::none()
            }
            Message::ShowSlideContextMenu(i) => {
                self.presenting.slide_context_index = Some(i);
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                if self.presenting.slide_context_pos.is_none() {
                    self.presenting.slide_context_pos = Some(iced::Point::new(24.0, 24.0));
                }
                Task::none()
            }
            Message::ShowSlideGroupSubmenu => {
                self.presenting.group_submenu = !self.presenting.group_submenu;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                Task::none()
            }
            Message::ShowSlideCueSubmenu => {
                self.presenting.cue_submenu = !self.presenting.cue_submenu;
                self.presenting.group_submenu = false;
                self.presenting.transition_submenu = false;
                Task::none()
            }
            Message::ShowSlideTransitionSubmenu => {
                self.presenting.transition_submenu = !self.presenting.transition_submenu;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                Task::none()
            }
            Message::HideSlideContextMenu => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                Task::none()
            }
            Message::ShowRailContextMenu(target) => {
                self.shell.rail_context_target = Some(target);
                if self.shell.rail_cursor_pos.is_none() {
                    self.shell.rail_cursor_pos = Some(iced::Point::new(40.0, 100.0));
                }
                Task::none()
            }
            Message::HideRailContextMenu => {
                self.shell.rail_context_target = None;
                Task::none()
            }
            Message::RailCursorMoved(p) => {
                if self.shell.rail_context_target.is_none() {
                    self.shell.rail_cursor_pos = Some(p);
                }
                Task::none()
            }
            Message::DuplicatePresentation(id) => {
                self.shell.rail_context_target = None;
                match self.services.presentations.duplicate_presentation(&id) {
                    Ok(pres) => {
                        self.load_presentations();
                        self.editor.editing = Some(pres.clone());
                        if self.shell.current_mode == crate::ui::messages::ViewMode::Show {
                            self.presenting.presentation = Some(pres);
                            self.presenting.slide_index = 0;
                            let _ = crate::ui::presenter::activate_slide(self, 0, false);
                        }
                    }
                    Err(e) => self.set_error(format!("Failed to duplicate presentation: {e}")),
                }
                Task::none()
            }
            Message::EditPresentation(id) => {
                self.shell.rail_context_target = None;
                self.shell.current_mode = crate::ui::messages::ViewMode::Edit;
                crate::ui::presentations::open(self, id)
            }
            Message::ExportPresentation(id) => {
                self.shell.rail_context_target = None;
                crate::ui::import_export::export_presentation_by_id(self, &id)
            }
            Message::SongToPresentation(id) => {
                self.shell.rail_context_target = None;
                let _ = crate::ui::songs::open_song(self, id);
                crate::ui::songs::song_to_presentation(self)
            }
            Message::AddSlideCue(idx, cue) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                if let Some(ref mut pres) = self.presenting.presentation
                    && let Some(slide) = pres.slides.get_mut(idx)
                {
                    slide.cues.push(cue);
                    let updated = slide.clone();
                    let pres_id = pres.id.clone();
                    let _ = self.services.presentations.update_slide(&pres_id, &updated);
                }
                Task::none()
            }
            Message::ClearSlideCues(idx) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                if let Some(ref mut pres) = self.presenting.presentation
                    && let Some(slide) = pres.slides.get_mut(idx)
                {
                    slide.cues.clear();
                    let updated = slide.clone();
                    let pres_id = pres.id.clone();
                    let _ = self.services.presentations.update_slide(&pres_id, &updated);
                }
                Task::none()
            }
            Message::SetSlideTransition(idx, trans) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                self.presenting.transition_submenu = false;
                if let Some(ref mut pres) = self.presenting.presentation
                    && let Some(slide) = pres.slides.get_mut(idx)
                {
                    slide.transition = trans;
                    let updated = slide.clone();
                    let pres_id = pres.id.clone();
                    let _ = self.services.presentations.update_slide(&pres_id, &updated);
                }
                Task::none()
            }
            Message::EditSlide(idx) => {
                self.presenting.slide_context_index = None;
                self.presenting.group_submenu = false;
                self.presenting.cue_submenu = false;
                if let Some(ref pres) = self.presenting.presentation {
                    self.editor.editing = Some(pres.clone());
                    self.editor.selected_slide_index = Some(idx);
                    self.shell.current_mode = ViewMode::Edit;
                    self.load_slide_for_editing();
                }
                Task::none()
            }
            Message::SetSlideGridCols(cols) => {
                self.presenting.slide_grid_cols = cols.clamp(2, 6);
                Task::none()
            }
            Message::SetSlideViewMode(mode) => {
                self.presenting.slide_view_mode = mode;
                Task::none()
            }
            Message::JumpToGroup(group_prefix) => {
                if let Some(ref pres) = self.presenting.presentation {
                    let pfx = group_prefix.to_lowercase();
                    let cur = self.presenting.slide_index;
                    let next_match = pres
                        .slides
                        .iter()
                        .enumerate()
                        .skip(cur + 1)
                        .find(|(_, s)| {
                            s.group
                                .as_deref()
                                .map(|g| g.to_lowercase().starts_with(&pfx))
                                .unwrap_or(false)
                        })
                        .or_else(|| {
                            pres.slides.iter().enumerate().find(|(_, s)| {
                                s.group
                                    .as_deref()
                                    .map(|g| g.to_lowercase().starts_with(&pfx))
                                    .unwrap_or(false)
                            })
                        });
                    if let Some((idx, _)) = next_match {
                        return self.update(Message::PresentingSelectSlide(idx));
                    } else if pfx == "chorus" {
                        return self.update(Message::ClearSlide);
                    }
                }
                Task::none()
            }
            Message::SetGlobalTransition(t) => {
                self.presenting.global_transition = t;
                Task::none()
            }
            Message::AnimationTick => crate::ui::presenter::animation_tick(self),
            Message::ToggleNdi => crate::ui::output::toggle_ndi(self),
            Message::NdiSendCurrent => crate::ui::output::ndi_send_current(self),
            Message::NdiBlackScreen => crate::ui::output::ndi_black_screen(self),
            Message::ClearOutput => {
                self.output.black_screen = true;
                self.presenting.slide_layer_active = false;
                self.presenting.media_layer_active = false;
                for p in &mut self.props.manager.props {
                    p.visible = false;
                }
                self.props.manager.clear_message();
                crate::ui::output::ndi_black_screen(self)
            }
            Message::ClearAll => {
                self.output.black_screen = false;
                self.presenting.slide_layer_active = false;
                self.presenting.media_layer_active = false;
                for p in &mut self.props.manager.props {
                    p.visible = false;
                }
                self.props.manager.clear_message();
                crate::ui::audio::stop(self)
            }
            Message::ClearSlide => {
                self.presenting.slide_layer_active = false;
                crate::ui::output::ndi_send_current(self)
            }
            Message::ClearMedia => {
                self.presenting.media_layer_active = false;
                crate::ui::output::ndi_send_current(self)
            }
            Message::ClearProps => {
                for p in &mut self.props.manager.props {
                    p.visible = false;
                }
                crate::ui::output::ndi_send_current(self)
            }
            Message::ClearMessages => {
                self.props.manager.clear_message();
                crate::ui::output::ndi_send_current(self)
            }
            Message::ClearAudio => crate::ui::audio::stop(self),
            Message::SelectPreviewScreen(screen_id) => {
                self.presenting.preview_screen_target = screen_id;
                Task::none()
            }
            Message::ToggleLooksMatrixModal => {
                self.output.matrix_open = !self.output.matrix_open;
                Task::none()
            }
            Message::SelectLook(id) => {
                self.props.manager.active_look_id = Some(id.clone());
                self.output.selected_look_id = Some(id);
                self.save_props();
                Task::none()
            }
            Message::CreateLook(name) => {
                let look = crate::domain::Look::with_default_screens(name);
                let id = look.id.clone();
                self.props.manager.add_look(look);
                self.props.manager.active_look_id = Some(id.clone());
                self.output.selected_look_id = Some(id);
                self.output.editing_look_name.clear();
                self.save_props();
                Task::none()
            }
            Message::LookNameInputChanged(val) => {
                self.output.editing_look_name = val;
                Task::none()
            }
            Message::DeleteLook(id) => {
                self.props.manager.looks.retain(|l| l.id != id);
                if self.props.manager.active_look_id.as_deref() == Some(&id) {
                    self.props.manager.active_look_id =
                        self.props.manager.looks.first().map(|l| l.id.clone());
                }
                if self.output.selected_look_id.as_deref() == Some(&id) {
                    self.output.selected_look_id = self.props.manager.active_look_id.clone();
                }
                self.save_props();
                Task::none()
            }
            Message::ToggleLookScreenLayer(look_id, screen_id, layer_type) => {
                if let Some(look) = self
                    .props
                    .manager
                    .looks
                    .iter_mut()
                    .find(|l| l.id == look_id)
                {
                    let mut target = look.target_or_default(&screen_id);
                    match layer_type {
                        crate::ui::messages::LookLayerType::Media => {
                            target.media_enabled = !target.media_enabled
                        }
                        crate::ui::messages::LookLayerType::Slide => {
                            target.slide_enabled = !target.slide_enabled
                        }
                        crate::ui::messages::LookLayerType::Props => {
                            target.props_enabled = !target.props_enabled
                        }
                        crate::ui::messages::LookLayerType::Messages => {
                            target.messages_enabled = !target.messages_enabled
                        }
                        crate::ui::messages::LookLayerType::Mask => {
                            target.mask_enabled = !target.mask_enabled
                        }
                    }
                    look.set_screen_target(target);
                    self.save_props();
                }
                Task::none()
            }
            Message::SetLookScreenTheme(look_id, screen_id, theme_id) => {
                if let Some(look) = self
                    .props
                    .manager
                    .looks
                    .iter_mut()
                    .find(|l| l.id == look_id)
                {
                    let mut target = look.target_or_default(&screen_id);
                    target.theme_id = theme_id;
                    look.set_screen_target(target);
                    self.save_props();
                }
                Task::none()
            }
            Message::SendLiveMessage(text) => {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    self.props.manager.set_message(trimmed);
                    self.output.message_input.clear();
                }
                Task::none()
            }
            Message::DismissLiveMessage => {
                self.props.manager.clear_message();
                Task::none()
            }
            Message::LiveMessageInputChanged(val) => {
                self.output.message_input = val;
                Task::none()
            }
            Message::Ndi(msg) => crate::ui::ndi::update(self, msg),
            Message::ToggleStageDisplay => crate::ui::stage::toggle_stage_display(self),
            Message::ClockTick => crate::ui::stage::clock_tick(self),
            Message::Stage(msg) => crate::ui::stage::update(self, msg),
            Message::SwitchSidebarTab(tab) => crate::ui::navigation::switch_sidebar_tab(self, tab),
            Message::Library(msg) => crate::ui::library::update(self, msg),
            Message::Themes(msg) => crate::ui::themes::update(self, msg),
            Message::ToggleShortcutsOverlay => crate::ui::polish::toggle_shortcuts_overlay(self),
            Message::ToggleReduceMotion => crate::ui::polish::toggle_reduce_motion(self),
            Message::Undo => crate::ui::polish::undo(self),
            Message::Redo => crate::ui::polish::redo(self),
            Message::Songs(msg) => crate::ui::songs::update(self, msg),
            Message::Bible(msg) => crate::ui::bible::update(self, msg),
            Message::Playlist(msg) => crate::ui::playlist::update(self, msg),
            Message::OpenOutputWindow => crate::ui::output::open_output_window(self),
            Message::CloseOutputWindow => crate::ui::output::close_output_window(self),
            Message::ToggleOutputWindow => {
                if self.output.window_id.is_some() {
                    crate::ui::output::close_output_window(self)
                } else {
                    crate::ui::output::open_output_window(self)
                }
            }
            Message::OutputWindowOpened => Task::none(),
            Message::ToggleOutputBlackScreen => crate::ui::output::toggle_output_black_screen(self),
            Message::WindowClosed(id) => crate::ui::output::window_closed(self, id),
            Message::ToggleOutputSettings => crate::ui::output::toggle_output_settings(self),
            Message::OutputScreenXChanged(v) => crate::ui::output::output_screen_x_changed(self, v),
            Message::OutputScreenYChanged(v) => crate::ui::output::output_screen_y_changed(self, v),
            Message::OutputAutoFullscreenToggled(v) => {
                crate::ui::output::output_auto_fullscreen_toggled(self, v)
            }
            Message::OutputFullscreenToggled => crate::ui::output::output_fullscreen_toggled(self),
            Message::OutputMonitorSizeFetched(_) => Task::none(),
            Message::Layers(msg) => crate::ui::layers::update(self, msg),
            Message::Typography(msg) => crate::ui::typography::update(self, msg),
            Message::Audio(msg) => crate::ui::audio::update(self, msg),
            Message::Output(msg) => crate::ui::output::update(self, msg),
            Message::ImportExport(msg) => crate::ui::import_export::update(self, msg),
            Message::ToggleImportExportPanel => crate::ui::import_export::toggle_panel(self),
            Message::Props(msg) => crate::ui::props::update(self, msg),
            Message::SetMask(mask) => crate::ui::props::set_mask(self, mask),
            Message::TogglePropsPanel => crate::ui::props::toggle_panel(self),
            Message::ToggleTriggersPanel => crate::ui::triggers::toggle_panel(self),
            Message::Triggers(msg) => crate::ui::triggers::update(self, msg),
            Message::TriggerFired(action) => crate::ui::triggers::trigger_fired(self, action),
            Message::Recording(msg) => crate::ui::recording::update(self, msg),
            Message::ToggleRecordingPanel => crate::ui::recording::toggle_panel(self),
        }
    }
}

impl MainWindow {
    pub fn title(&self, id: window::Id) -> String {
        if Some(id) == self.output.window_id {
            String::from("OpenPresenter — Output")
        } else if Some(id) == self.editor.delete_confirm_window_id {
            String::from("Confirm Delete")
        } else if Some(id) == self.editor.new_presentation_window_id {
            String::from("New Presentation")
        } else if Some(id) == self.ui.shortcuts_window_id {
            String::from("Keyboard Shortcuts")
        } else {
            String::from("OpenPresenter")
        }
    }

    pub fn composite_layers_for_screen(
        &self,
        screen_id: &str,
    ) -> crate::ui::presenter::canvas::CompositeLayers {
        let active_look = self.props.manager.current_look();
        let target = active_look
            .map(|l| l.target_or_default(screen_id))
            .unwrap_or_else(|| crate::domain::ScreenLookTarget::default_for_screen(screen_id));

        let theme_override = target
            .theme_id
            .as_ref()
            .and_then(|tid| self.theme_state.list.iter().find(|t| &t.id == tid).cloned());

        let mask = if target.mask_enabled {
            active_look
                .map(|l| l.mask.clone())
                .unwrap_or(crate::domain::Mask::None)
        } else {
            crate::domain::Mask::None
        };

        crate::ui::presenter::canvas::CompositeLayers {
            media_enabled: target.media_enabled && self.presenting.media_layer_active,
            slide_enabled: target.slide_enabled && self.presenting.slide_layer_active,
            props_enabled: target.props_enabled,
            messages_enabled: target.messages_enabled,
            mask_enabled: target.mask_enabled,
            props: self.props.manager.props.clone(),
            live_message: self
                .props
                .manager
                .live_message
                .as_ref()
                .map(|m| m.text.clone()),
            mask,
            theme_override,
        }
    }

    pub fn view_for_window(&self, id: window::Id) -> Element<'_, Message> {
        if Some(id) == self.output.window_id {
            let current = self
                .presenting
                .presentation
                .as_ref()
                .and_then(|p| p.slides.get(self.presenting.slide_index));
            let (from_slide, trans, progress) = match &self.presenting.transition {
                Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
                None => (None, Transition::Cut, 1.0),
            };
            let layers = self.composite_layers_for_screen("main");
            output_window::view(
                current,
                from_slide,
                trans,
                progress,
                self.video.frame.as_ref(),
                self.output.black_screen,
                &layers,
            )
        } else if Some(id) == self.editor.delete_confirm_window_id {
            self.delete_confirm_view()
        } else if Some(id) == self.editor.new_presentation_window_id {
            self.new_presentation_view()
        } else if Some(id) == self.ui.shortcuts_window_id {
            self.shortcuts_view()
        } else {
            self.view()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        crate::ui::shell::unified::view(self)
    }

    fn delete_confirm_view(&self) -> Element<'_, Message> {
        use iced::widget::{Space, button, column, row, text};
        let (kind, name, confirm_msg, cancel_msg): (&str, String, Message, Message) =
            if let Some(ref id) = self.editor.delete_target_id {
                let n = self
                    .editor
                    .presentations
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "this presentation".to_string());
                (
                    "Presentation",
                    n,
                    Message::ConfirmDeletePresentation,
                    Message::CancelDelete,
                )
            } else if let Some(ref id) = self.song.to_delete {
                let n = self
                    .song
                    .songs
                    .iter()
                    .find(|s| &s.id == id)
                    .map(|s| s.title.clone())
                    .unwrap_or_else(|| "this song".to_string());
                (
                    "Song",
                    n,
                    Message::Songs(crate::ui::songs::Message::ConfirmDelete),
                    Message::Songs(crate::ui::songs::Message::CancelDelete),
                )
            } else if let Some(ref id) = self.service.to_delete {
                let n = self
                    .service
                    .plans
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "this plan".to_string());
                (
                    "Service Plan",
                    n,
                    Message::Playlist(crate::ui::playlist::Message::ConfirmDelete),
                    Message::Playlist(crate::ui::playlist::Message::CancelDelete),
                )
            } else {
                (
                    "Item",
                    "this item".to_string(),
                    Message::CancelDelete,
                    Message::CancelDelete,
                )
            };

        let content = column![
            text(format!("Delete {kind}"))
                .size(18)
                .color(theme::TEXT_PRIMARY),
            Space::new().height(12),
            text(format!("Delete \"{}\"?", name))
                .size(14)
                .color(theme::TEXT_SECONDARY),
            Space::new().height(6),
            text("This action cannot be undone.")
                .size(12)
                .color(theme::TEXT_MUTED),
            Space::new().height(24),
            row![
                button(text("Delete").size(13))
                    .on_press(confirm_msg)
                    .padding([9, 28])
                    .style(theme::danger_button),
                button(text("Cancel").size(13))
                    .on_press(cancel_msg)
                    .padding([9, 20])
                    .style(theme::secondary_button),
            ]
            .spacing(10),
        ]
        .padding(32)
        .spacing(2);

        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::dialog_card_style)
            .into()
    }

    fn new_presentation_view(&self) -> Element<'_, Message> {
        use iced::widget::{Space, button, column, row, text, text_input};
        let content = column![
            text("New Presentation").size(18).color(theme::TEXT_PRIMARY),
            Space::new().height(16),
            text("Name:").size(13).color(theme::TEXT_SECONDARY),
            Space::new().height(6),
            text_input("Presentation name...", &self.editor.new_presentation_name)
                .on_input(Message::NewPresentationNameChanged)
                .on_submit(Message::CreatePresentation)
                .padding([10, 12])
                .size(14)
                .width(Length::Fill),
            Space::new().height(20),
            row![
                button(text("Create").size(13))
                    .on_press(Message::CreatePresentation)
                    .padding([9, 28])
                    .style(theme::primary_button),
                button(text("Cancel").size(13))
                    .on_press(Message::CancelNewPresentation)
                    .padding([9, 20])
                    .style(theme::secondary_button),
            ]
            .spacing(10),
        ]
        .padding(32)
        .spacing(2);

        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::dialog_card_style)
            .into()
    }

    fn shortcuts_view(&self) -> Element<'_, Message> {
        use iced::widget::{Space, button, column, row, scrollable, text};
        let shortcut = |key: &'static str, desc: &'static str| -> Element<'_, Message> {
            row![
                iced::widget::container(text(key).size(11).color(theme::TEXT_SECONDARY)).width(150),
                text(desc).size(11).color(theme::TEXT_MUTED),
            ]
            .spacing(8)
            .into()
        };

        let content = column![
            text("Keyboard Shortcuts")
                .size(16)
                .color(theme::TEXT_PRIMARY),
            Space::new().height(14),
            text("EDITOR").size(10).color(theme::TEXT_MUTED),
            Space::new().height(4),
            shortcut("?", "Toggle shortcuts"),
            shortcut("Cmd+Z / Ctrl+Z", "Undo"),
            shortcut("Cmd+Shift+Z / Ctrl+Y", "Redo"),
            Space::new().height(10),
            text("PRESENTER (SHOW MODE)")
                .size(10)
                .color(theme::TEXT_MUTED),
            Space::new().height(4),
            shortcut("Right / Space / PgDn / Down", "Next slide"),
            shortcut("Left / PgUp / Up", "Previous slide"),
            shortcut("Home / End", "First / Last slide"),
            shortcut("F1", "Clear All"),
            shortcut("F2 / C", "Clear Slide"),
            shortcut("F3", "Clear Media"),
            shortcut("F4", "Clear Props"),
            shortcut("F5", "Clear Audio"),
            shortcut("F6", "Clear Messages"),
            shortcut("B", "Toggle Black Screen"),
            shortcut("Escape", "Return to library"),
            Space::new().height(20),
            button(text("Close").size(13))
                .on_press(Message::ToggleShortcutsOverlay)
                .padding([8, 24])
                .style(theme::secondary_button),
        ]
        .padding(32)
        .spacing(4);

        iced::widget::container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::dialog_card_style)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let kb_sub = if self.shell.current_mode == ViewMode::Show
            && self.presenting.presentation.is_some()
        {
            event::listen_with(|ev, _status, _id| {
                if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) = ev
                {
                    return match key {
                        keyboard::Key::Named(key::Named::ArrowRight)
                        | keyboard::Key::Named(key::Named::ArrowDown)
                        | keyboard::Key::Named(key::Named::PageDown)
                        | keyboard::Key::Named(key::Named::Space) => {
                            Some(Message::PresentingNextSlide)
                        }
                        keyboard::Key::Named(key::Named::ArrowLeft)
                        | keyboard::Key::Named(key::Named::ArrowUp)
                        | keyboard::Key::Named(key::Named::PageUp) => {
                            Some(Message::PresentingPrevSlide)
                        }
                        keyboard::Key::Named(key::Named::Home) => {
                            Some(Message::PresentingSelectSlide(0))
                        }
                        keyboard::Key::Named(key::Named::End) => {
                            Some(Message::PresentingSelectLastSlide)
                        }
                        keyboard::Key::Named(key::Named::F1) => Some(Message::ClearAll),
                        keyboard::Key::Named(key::Named::F2) => Some(Message::ClearSlide),
                        keyboard::Key::Named(key::Named::F3) => Some(Message::ClearMedia),
                        keyboard::Key::Named(key::Named::F4) => Some(Message::ClearProps),
                        keyboard::Key::Named(key::Named::F5) => Some(Message::ClearAudio),
                        keyboard::Key::Named(key::Named::F6) => Some(Message::ClearMessages),
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "1" => {
                            Some(Message::ClearAll)
                        }
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "2" => {
                            Some(Message::ClearSlide)
                        }
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "3" => {
                            Some(Message::ClearMedia)
                        }
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "4" => {
                            Some(Message::ClearProps)
                        }
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "5" => {
                            Some(Message::ClearAudio)
                        }
                        keyboard::Key::Character(c) if modifiers.command() && c.as_str() == "6" => {
                            Some(Message::ClearMessages)
                        }
                        keyboard::Key::Character(c)
                            if (c.as_str() == "b" || c.as_str() == "B")
                                && !modifiers.command()
                                && !modifiers.control() =>
                        {
                            Some(Message::ToggleOutputBlackScreen)
                        }
                        keyboard::Key::Character(c)
                            if (c.as_str() == "v" || c.as_str() == "V")
                                && !modifiers.command()
                                && !modifiers.control() =>
                        {
                            Some(Message::JumpToGroup("Verse".to_string()))
                        }
                        keyboard::Key::Character(c)
                            if (c.as_str() == "c" || c.as_str() == "C")
                                && !modifiers.command()
                                && !modifiers.control() =>
                        {
                            Some(Message::JumpToGroup("Chorus".to_string()))
                        }
                        keyboard::Key::Character(c)
                            if !modifiers.command()
                                && !modifiers.control()
                                && c.as_str().len() == 1
                                && c.as_str().chars().all(|ch| ch.is_ascii_digit()) =>
                        {
                            let n: usize = c.as_str().parse().unwrap_or(0);
                            if n >= 1 {
                                Some(Message::PresentingSelectSlide(n - 1))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                }
                None
            })
        } else {
            Subscription::none()
        };

        let anim_sub = if self.presenting.transition.is_some() {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        let video_sub = if self.video.player.as_ref().is_some_and(|p| p.is_playing()) {
            iced::time::every(Duration::from_millis(33)).map(|_| Message::VideoFrameTick)
        } else {
            Subscription::none()
        };

        let clock_sub = iced::time::every(Duration::from_secs(1)).map(|_| Message::ClockTick);

        let global_kb_sub = event::listen_with(|ev, _status, _id| {
            let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = ev
            else {
                return None;
            };
            match key {
                keyboard::Key::Character(c) if c.as_str() == "?" => {
                    Some(Message::ToggleShortcutsOverlay)
                }
                keyboard::Key::Character(c)
                    if (c.as_str() == "," || c.as_str() == "<") && modifiers.command() =>
                {
                    Some(Message::Output(crate::ui::output::Message::SettingsOpen))
                }
                keyboard::Key::Character(c)
                    if c.as_str() == "z" && modifiers.command() && modifiers.shift() =>
                {
                    Some(Message::Redo)
                }
                keyboard::Key::Character(c) if c.as_str() == "z" && modifiers.command() => {
                    Some(Message::Undo)
                }
                keyboard::Key::Character(c) if c.as_str() == "y" && modifiers.command() => {
                    Some(Message::Redo)
                }
                keyboard::Key::Named(key::Named::Escape) => Some(Message::EscapePressed),
                _ => None,
            }
        });

        let window_close_sub = event::listen_with(|ev, _status, id| {
            if let iced::Event::Window(iced::window::Event::CloseRequested) = ev {
                Some(Message::WindowClosed(id))
            } else {
                None
            }
        });

        let trigger_sub =
            Subscription::run_with(TriggerSubKey(Arc::clone(&self.triggers.rx)), trigger_stream);

        Subscription::batch([
            kb_sub,
            anim_sub,
            video_sub,
            clock_sub,
            global_kb_sub,
            window_close_sub,
            trigger_sub,
        ])
    }
}
