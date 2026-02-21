use crate::config::Config;
use crate::db::{Database, LibraryRepository, PresentationRepository, ThemeRepository};
use crate::ndi::NdiOutputLoop;
use crate::slides::{LibraryAsset, Slide, SlideTheme, Transition};
use crate::ui::components::error_toast;
use crate::ui::messages::{InspectorTab, Message, SidebarTab, ViewMode};
use crate::ui::presenter::TransitionState;
use crate::ui::state::{AudioState, BibleState, LayerState, ServiceState, SongState};
use crate::ui::{editor, navbar, output_window, presenter, sidebar, stage, theme};
use iced::{
    Alignment, Element, Length, Subscription, Task, event,
    keyboard::{self, key},
    widget::{Space, button, column, container, row, text},
    window,
};
use std::sync::Arc;
use std::time::Duration;

type TriggerRx =
    Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::triggers::TriggerAction>>>;

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
    pub(crate) current_mode: ViewMode,
    pub(crate) inspector_tab: InspectorTab,
    pub(crate) search_query: String,
    pub(crate) new_presentation_window_id: Option<window::Id>,
    pub(crate) new_presentation_name: String,
    pub(crate) show_delete_confirmation: bool,
    pub(crate) delete_target_id: Option<String>,
    pub(crate) delete_confirm_window_id: Option<window::Id>,
    pub(crate) presentations: Vec<crate::slides::Presentation>,
    pub(crate) editing_presentation: Option<crate::slides::Presentation>,
    pub(crate) editing_presentation_name: String,
    pub(crate) presenting_presentation: Option<crate::slides::Presentation>,
    pub(crate) presenting_slide_index: usize,
    pub(crate) selected_slide_index: Option<usize>,
    pub(crate) editing_slide_text: String,
    pub(crate) editing_slide_font_size: String,
    pub(crate) editing_transition_duration: String,
    pub(crate) editing_group_label: String,
    pub(crate) ndi_output: Option<NdiOutputLoop>,
    pub(crate) presenting_transition: Option<TransitionState>,
    pub(crate) stage_display_active: bool,
    pub(crate) editing_slide_notes: String,
    pub(crate) clock_secs: u64,
    pub(crate) timer_secs: u64,
    pub(crate) timer_running: bool,
    pub(crate) timer_start_epoch: u64,
    pub(crate) sidebar_tab: SidebarTab,
    pub(crate) lib_assets: Vec<LibraryAsset>,
    pub(crate) selected_asset_id: Option<String>,
    pub(crate) recently_used_asset_ids: Vec<String>,
    pub(crate) lib_repo: LibraryRepository,
    pub(crate) themes: Vec<SlideTheme>,
    pub(crate) selected_theme_id: Option<String>,
    pub(crate) new_theme_name: String,
    pub(crate) theme_repo: ThemeRepository,
    pub(crate) shortcuts_window_id: Option<window::Id>,
    pub(crate) reduce_motion: bool,
    pub(crate) undo_stack: Vec<crate::slides::Presentation>,
    pub(crate) redo_stack: Vec<crate::slides::Presentation>,
    #[allow(dead_code)]
    db: Arc<Database>,
    pub(crate) repo: PresentationRepository,
    pub(crate) song: SongState,
    pub(crate) media_player: Option<crate::media::MediaPlayer>,
    pub(crate) video_frame: Option<iced::widget::image::Handle>,
    pub(crate) video_position: f64,
    pub(crate) video_looping: bool,
    pub(crate) video_volume: f32,
    pub(crate) video_muted: bool,
    pub(crate) video_speed: f64,
    pub(crate) service: ServiceState,
    pub(crate) main_window_id: window::Id,
    pub(crate) output_window_id: Option<window::Id>,
    pub(crate) output_black_screen: bool,
    pub(crate) output_is_fullscreen: bool,
    pub(crate) show_output_settings: bool,
    pub(crate) editing_output_screen_x: String,
    pub(crate) editing_output_screen_y: String,
    pub(crate) app_config: Config,
    pub(crate) layer: LayerState,
    pub(crate) bible: BibleState,
    pub(crate) audio: AudioState,
    pub(crate) output_manager: crate::output::OutputManager,
    pub(crate) output_settings_open: bool,
    pub(crate) new_output_label: String,
    pub(crate) new_output_ndi_name: String,
    pub(crate) import_export_panel_open: bool,
    pub(crate) prop_manager: crate::slides::PropManager,
    pub(crate) props_panel_open: bool,
    pub(crate) new_prop_name: String,
    pub(crate) new_look_name: String,
    pub(crate) lower_third_title: String,
    pub(crate) lower_third_subtitle: String,
    pub(crate) trigger_manager: crate::triggers::TriggerManager,
    pub(crate) trigger_rx: std::sync::Arc<
        tokio::sync::Mutex<tokio::sync::mpsc::Receiver<crate::triggers::TriggerAction>>,
    >,
    pub(crate) triggers_panel_open: bool,
    pub(crate) trigger_http_port_str: String,
    pub(crate) trigger_osc_port_str: String,
    pub(crate) new_macro_name: String,
    pub(crate) macro_running_handles:
        std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
    pub(crate) macro_running_ids: std::collections::HashSet<String>,
    pub(crate) recording_manager: crate::recording::RecordingManager,
    pub(crate) recording_panel_open: bool,
    pub(crate) error_message: Option<String>,
}

impl MainWindow {
    pub fn new(db: Arc<Database>, main_window_id: window::Id) -> (Self, Task<Message>) {
        let app_config = Config::load();
        let repo = PresentationRepository::new(db.clone());
        let mut trigger_manager = crate::triggers::TriggerManager::default();
        let trigger_rx = std::sync::Arc::new(tokio::sync::Mutex::new(trigger_manager.subscribe()));
        let mut window = Self {
            current_mode: ViewMode::Edit,
            inspector_tab: InspectorTab::Text,
            search_query: String::new(),
            new_presentation_window_id: None,
            new_presentation_name: String::new(),
            show_delete_confirmation: false,
            delete_target_id: None,
            delete_confirm_window_id: None,
            presentations: Vec::new(),
            editing_presentation: None,
            editing_presentation_name: String::new(),
            presenting_presentation: None,
            presenting_slide_index: 0,
            selected_slide_index: None,
            editing_slide_text: String::new(),
            editing_slide_font_size: String::from("72"),
            editing_transition_duration: String::from("500"),
            editing_group_label: String::new(),
            ndi_output: None,
            presenting_transition: None,
            stage_display_active: false,
            editing_slide_notes: String::new(),
            clock_secs: 0,
            timer_secs: 0,
            timer_running: false,
            timer_start_epoch: 0,
            sidebar_tab: SidebarTab::default(),
            lib_assets: Vec::new(),
            selected_asset_id: None,
            recently_used_asset_ids: Vec::new(),
            lib_repo: LibraryRepository::new(db.clone()),
            themes: Vec::new(),
            selected_theme_id: None,
            new_theme_name: String::new(),
            theme_repo: ThemeRepository::new(db.clone()),
            shortcuts_window_id: None,
            reduce_motion: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            song: SongState::new(crate::db::SongRepository::new(db.clone())),
            media_player: None,
            video_frame: None,
            video_position: 0.0,
            video_looping: true,
            video_volume: 1.0,
            video_muted: false,
            video_speed: 1.0,
            service: ServiceState::new(crate::db::ServiceRepository::new(db.clone())),
            main_window_id,
            output_window_id: None,
            output_black_screen: false,
            output_is_fullscreen: false,
            show_output_settings: false,
            editing_output_screen_x: app_config.output.screen_x.to_string(),
            editing_output_screen_y: app_config.output.screen_y.to_string(),
            app_config,
            layer: LayerState::default(),
            bible: BibleState::new(crate::db::BibleRepository::new(db.clone())),
            audio: AudioState::default(),
            output_manager: crate::output::OutputManager::with_defaults(),
            output_settings_open: false,
            new_output_label: String::new(),
            new_output_ndi_name: String::new(),
            import_export_panel_open: false,
            prop_manager: crate::slides::PropManager::default(),
            props_panel_open: false,
            new_prop_name: String::new(),
            new_look_name: String::new(),
            lower_third_title: String::new(),
            lower_third_subtitle: String::new(),
            trigger_manager,
            trigger_rx,
            triggers_panel_open: false,
            trigger_http_port_str: String::from("9090"),
            trigger_osc_port_str: String::from("9000"),
            new_macro_name: String::new(),
            macro_running_handles: std::collections::HashMap::new(),
            macro_running_ids: std::collections::HashSet::new(),
            recording_manager: crate::recording::RecordingManager::new(),
            recording_panel_open: false,
            error_message: None,
            db,
            repo,
        };
        window.load_presentations();
        window.load_songs();
        window.load_service_plans();
        window.load_bible_translations();
        (window, Task::none())
    }

    pub(crate) fn load_presentations(&mut self) {
        match self.repo.list_presentations() {
            Ok(list) => self.presentations = list,
            Err(e) => eprintln!("Failed to load presentations: {}", e),
        }
    }

    pub(crate) fn load_songs(&mut self) {
        match self.song.repo.list_songs() {
            Ok(list) => self.song.songs = list,
            Err(e) => eprintln!("Failed to load songs: {}", e),
        }
    }

    pub(crate) fn load_service_plans(&mut self) {
        match self.service.repo.list_plans() {
            Ok(list) => self.service.plans = list,
            Err(e) => eprintln!("Failed to load service plans: {}", e),
        }
    }

    pub(crate) fn load_bible_translations(&mut self) {
        match self.bible.repo.list_translations() {
            Ok(list) => self.bible.translations = list,
            Err(e) => eprintln!("Failed to load Bible translations: {}", e),
        }
    }

    pub(crate) fn load_song_for_editing(&mut self, song: crate::slides::Song) {
        self.song.edit_title = song.title.clone();
        self.song.edit_artist = song.artist.clone().unwrap_or_default();
        self.song.edit_copyright = song.copyright.clone().unwrap_or_default();
        self.song.edit_ccli = song.ccli_number.clone().unwrap_or_default();
        self.song.edit_key = song.key_signature.clone().unwrap_or_default();
        self.song.edit_bpm = song.bpm.map(|v| v.to_string()).unwrap_or_default();
        self.song.editing = Some(song);
    }

    pub(crate) fn load_lib_assets(&mut self) {
        match self.lib_repo.list_assets() {
            Ok(list) => self.lib_assets = list,
            Err(e) => eprintln!("Failed to load library assets: {}", e),
        }
    }

    pub(crate) fn load_themes(&mut self) {
        match self.theme_repo.list_themes() {
            Ok(list) => self.themes = list,
            Err(e) => eprintln!("Failed to load themes: {}", e),
        }
    }

    pub(crate) fn push_undo(&mut self) {
        if let Some(ref pres) = self.editing_presentation {
            self.undo_stack.push(pres.clone());
            if self.undo_stack.len() > 50 {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
    }

    pub(crate) fn load_slide_for_editing(&mut self) {
        let (text, font_size, transition_dur, group_label, notes) =
            if let Some(slide) = self.get_current_slide() {
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
                let group = slide.group_label.clone().unwrap_or_default();
                let notes = slide.notes.clone().unwrap_or_default();
                if let crate::slides::SlideContent::Text {
                    ref text,
                    ref style,
                } = slide.content
                {
                    (text.clone(), style.font_size, dur, group, notes)
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
        self.editing_slide_text = text;
        self.editing_slide_font_size = font_size.to_string();
        self.editing_transition_duration = transition_dur;
        self.editing_group_label = group_label;
        self.editing_slide_notes = notes;
        self.layer.selected_index = None;
        self.load_layer_for_editing();
    }

    pub(crate) fn load_layer_for_editing(&mut self) {
        use crate::slides::LayerContent;
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
            ) = if let LayerContent::Text {
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
            let stroke_w = if let LayerContent::Shape { stroke_width, .. } = layer.content {
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
            self.layer.text = text;
            self.layer.font_size = font_size;
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
        if let Some(index) = self.selected_slide_index
            && let Some(ref pres) = self.editing_presentation
        {
            return pres.slides.get(index);
        }
        None
    }

    pub(crate) fn get_current_slide_mut(&mut self) -> Option<&mut Slide> {
        if let Some(index) = self.selected_slide_index
            && let Some(ref mut pres) = self.editing_presentation
        {
            return pres.slides.get_mut(index);
        }
        None
    }
}

impl MainWindow {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        use crate::ui::handlers::{
            audio as audio_h, bible as bible_h, import_export as ie_h, layers as layer_h, media,
            navigation, outputs as outputs_h, planning as plan_h, polish, presentations,
            presenter as pres_h, props as props_h, recording as rec_h, slides as slide_h,
            songs as song_h, themes as theme_h, triggers as trig_h, typography as typo_h,
            video as video_h,
        };
        match message {
            Message::Noop => Task::none(),
            Message::DismissError => {
                self.error_message = None;
                Task::none()
            }
            Message::SwitchMode(mode) => navigation::switch_mode(self, mode),
            Message::SwitchInspectorTab(tab) => navigation::switch_inspector_tab(self, tab),
            Message::BackToList => navigation::back_to_list(self),
            Message::Quit => navigation::quit(self),
            Message::SearchQueryChanged(q) => navigation::search_query_changed(self, q),
            Message::NewPresentationClicked => navigation::new_presentation_clicked(self),
            Message::CreatePresentation => navigation::create_presentation(self),
            Message::CancelNewPresentation => navigation::cancel_new_presentation(self),
            Message::NewPresentationNameChanged(name) => {
                navigation::new_presentation_name_changed(self, name)
            }
            Message::SelectPresentation(_) => Task::none(),
            Message::OpenPresentation(id) => presentations::open(self, id),
            Message::RenamePresentation => presentations::rename(self),
            Message::RenamePresentationChanged(name) => presentations::rename_changed(self, name),
            Message::DeletePresentationClicked(id) => presentations::delete_clicked(self, id),
            Message::ConfirmDeletePresentation => presentations::confirm_delete(self),
            Message::CancelDelete => presentations::cancel_delete(self),
            Message::AddSlide => slide_h::add_slide(self),
            Message::AddSlideAfter(i) => slide_h::add_slide_after(self, i),
            Message::DuplicateSlide(i) => slide_h::duplicate_slide(self, i),
            Message::SelectSlide(i) => slide_h::select_slide(self, i),
            Message::DeleteSlide(id) => slide_h::delete_slide(self, id),
            Message::MoveSlideUp(i) => slide_h::move_slide_up(self, i),
            Message::MoveSlideDown(i) => slide_h::move_slide_down(self, i),
            Message::GroupLabelChanged(label) => slide_h::group_label_changed(self, label),
            Message::SetSlideGroupLabel(i, label) => slide_h::set_slide_group_label(self, i, label),
            Message::SlideTextChanged(t) => slide_h::slide_text_changed(self, t),
            Message::SlideFontSizeChanged(s) => slide_h::slide_font_size_changed(self, s),
            Message::SlideAlignmentChanged(a) => slide_h::slide_alignment_changed(self, a),
            Message::SlideColorChanged(c) => slide_h::slide_color_changed(self, c),
            Message::SlideShadowToggled(v) => slide_h::slide_shadow_toggled(self, v),
            Message::SlideOutlineToggled(v) => slide_h::slide_outline_toggled(self, v),
            Message::SlideBoldToggled(v) => slide_h::slide_bold_toggled(self, v),
            Message::SlideItalicToggled(v) => slide_h::slide_italic_toggled(self, v),
            Message::SlidePositionPreset(x, y) => slide_h::slide_position_preset(self, x, y),
            Message::SlideBackgroundChanged(bg) => slide_h::slide_background_changed(self, bg),
            Message::SlideTransitionChanged(t) => slide_h::slide_transition_changed(self, t),
            Message::TransitionDurationChanged(s) => slide_h::transition_duration_changed(self, s),
            Message::SaveSlide => slide_h::save_slide(self),
            Message::TextDragStarted => Task::none(),
            Message::TextDragged(p) => slide_h::text_dragged(self, p),
            Message::TextDragEnded => slide_h::text_drag_ended(self),
            Message::SlideNotesChanged(notes) => slide_h::slide_notes_changed(self, notes),
            Message::ConvertSlideToText => slide_h::convert_to_text(self),
            Message::ConvertSlideToImage => slide_h::convert_to_image(self),
            Message::ConvertSlideToVideo => slide_h::convert_to_video(self),
            Message::PickImageFile => slide_h::pick_image_file(self),
            Message::ImageFilePicked(_) => Task::none(),
            Message::SlideImageFitChanged(fit) => slide_h::slide_image_fit_changed(self, fit),
            Message::PickVideoFile => slide_h::pick_video_file(self),
            Message::VideoFilePicked(_) => Task::none(),
            Message::VideoFrameTick => video_h::video_frame_tick(self),
            Message::VideoPlayToggled => video_h::video_play_toggled(self),
            Message::VideoLoopToggled(l) => video_h::video_loop_toggled(self, l),
            Message::VideoVolumeChanged(v) => video_h::video_volume_changed(self, v),
            Message::VideoSeekChanged(s) => video_h::video_seek_changed(self, s),
            Message::VideoMuteToggled => video_h::video_mute_toggled(self),
            Message::VideoSpeedChanged(s) => video_h::video_speed_changed(self, s),
            Message::PresentingSelectSlide(i) => pres_h::select_slide(self, i),
            Message::PresentingNextSlide => pres_h::next_slide(self),
            Message::PresentingPrevSlide => pres_h::prev_slide(self),
            Message::AnimationTick => pres_h::animation_tick(self),
            Message::ToggleNdi => media::toggle_ndi(self),
            Message::NdiSendCurrent => media::ndi_send_current(self),
            Message::NdiBlackScreen => media::ndi_black_screen(self),
            Message::ClearOutput => media::ndi_black_screen(self),
            Message::ToggleStageDisplay => media::toggle_stage_display(self),
            Message::ClockTick => media::clock_tick(self),
            Message::ToggleTimer => media::toggle_timer(self),
            Message::ResetTimer => media::reset_timer(self),
            Message::SwitchSidebarTab(tab) => media::switch_sidebar_tab(self, tab),
            Message::LibraryImportAsset => media::library_import_asset(self),
            Message::LibraryApplyToSlide(id) => media::library_apply_to_slide(self, id),
            Message::LibraryDeleteAsset(id) => media::library_delete_asset(self, id),
            Message::LibrarySelectAsset(id) => media::library_select_asset(self, id),
            Message::ThemeNameChanged(name) => theme_h::theme_name_changed(self, name),
            Message::SaveSlideAsTheme => theme_h::save_slide_as_theme(self),
            Message::ApplyTheme(id) => theme_h::apply_theme(self, id),
            Message::DeleteTheme(id) => theme_h::delete_theme(self, id),
            Message::SelectTheme(id) => theme_h::select_theme(self, id),
            Message::ExportThemes => theme_h::export_themes(self),
            Message::ImportThemes => theme_h::import_themes(self),
            Message::ToggleShortcutsOverlay => polish::toggle_shortcuts_overlay(self),
            Message::ToggleReduceMotion => polish::toggle_reduce_motion(self),
            Message::Undo => polish::undo(self),
            Message::Redo => polish::redo(self),
            Message::SongSearchChanged(q) => song_h::search_changed(self, q),
            Message::NewSong => song_h::new_song(self),
            Message::OpenSong(id) => song_h::open_song(self, id),
            Message::SaveSong => song_h::save_song(self),
            Message::DeleteSongClicked(id) => song_h::delete_clicked(self, id),
            Message::ConfirmDeleteSong => song_h::confirm_delete(self),
            Message::CancelDeleteSong => song_h::cancel_delete(self),
            Message::SongTitleChanged(v) => song_h::title_changed(self, v),
            Message::SongArtistChanged(v) => song_h::artist_changed(self, v),
            Message::SongCopyrightChanged(v) => song_h::copyright_changed(self, v),
            Message::SongCcliChanged(v) => song_h::ccli_changed(self, v),
            Message::SongKeyChanged(v) => song_h::key_changed(self, v),
            Message::SongBpmChanged(v) => song_h::bpm_changed(self, v),
            Message::AddVerse => song_h::add_verse(self),
            Message::DeleteVerse(i) => song_h::delete_verse(self, i),
            Message::MoveVerseUp(i) => song_h::move_verse_up(self, i),
            Message::MoveVerseDown(i) => song_h::move_verse_down(self, i),
            Message::VerseContentChanged(i, v) => song_h::verse_content_changed(self, i, v),
            Message::VerseLabelChanged(i, v) => song_h::verse_label_changed(self, i, v),
            Message::SongToPresentation => song_h::song_to_presentation(self),
            Message::NewServicePlan => plan_h::new_plan(self),
            Message::OpenServicePlan(id) => plan_h::open_plan(self, id),
            Message::SaveServicePlan => plan_h::save_plan(self),
            Message::DeleteServicePlanClicked(id) => plan_h::delete_clicked(self, id),
            Message::ConfirmDeleteServicePlan => plan_h::confirm_delete(self),
            Message::CancelDeleteServicePlan => plan_h::cancel_delete(self),
            Message::ServicePlanNameChanged(name) => plan_h::plan_name_changed(self, name),
            Message::AddPresentationItem(id) => plan_h::add_presentation_item(self, id),
            Message::AddSongItem(id) => plan_h::add_song_item(self, id),
            Message::AddHeaderItem => plan_h::add_header(self),
            Message::AddBlankItem => plan_h::add_blank(self),
            Message::ServiceItemHeaderChanged(i, t) => plan_h::item_header_changed(self, i, t),
            Message::RemoveServiceItem(i) => plan_h::remove_item(self, i),
            Message::MoveServiceItemUp(i) => plan_h::move_item_up(self, i),
            Message::MoveServiceItemDown(i) => plan_h::move_item_down(self, i),
            Message::DuplicateServiceItem(i) => plan_h::duplicate_item(self, i),
            Message::StartService => plan_h::start_service(self),
            Message::ServiceNextItem => plan_h::service_next(self),
            Message::ServicePrevItem => plan_h::service_prev(self),
            Message::ServiceJumpToItem(i) => plan_h::jump_to_item(self, i),
            Message::EndService => plan_h::end_service(self),
            Message::OpenOutputWindow => media::open_output_window(self),
            Message::CloseOutputWindow => media::close_output_window(self),
            Message::OutputWindowOpened => Task::none(),
            Message::ToggleOutputBlackScreen => media::toggle_output_black_screen(self),
            Message::WindowClosed(id) => media::window_closed(self, id),
            Message::ToggleOutputSettings => media::toggle_output_settings(self),
            Message::OutputScreenXChanged(v) => media::output_screen_x_changed(self, v),
            Message::OutputScreenYChanged(v) => media::output_screen_y_changed(self, v),
            Message::OutputAutoFullscreenToggled(v) => {
                media::output_auto_fullscreen_toggled(self, v)
            }
            Message::OutputFullscreenToggled => media::output_fullscreen_toggled(self),
            Message::OutputMonitorSizeFetched(_) => Task::none(),
            Message::AddTextLayer => layer_h::add_text_layer(self),
            Message::AddShapeLayer(s) => layer_h::add_shape_layer(self, s),
            Message::SelectLayer(i) => layer_h::select_layer(self, i),
            Message::DeleteSelectedLayer => layer_h::delete_selected_layer(self),
            Message::MoveSelectedLayerUp => layer_h::move_selected_layer_up(self),
            Message::MoveSelectedLayerDown => layer_h::move_selected_layer_down(self),
            Message::ToggleSelectedLayerVisibility => {
                layer_h::toggle_selected_layer_visibility(self)
            }
            Message::ToggleSelectedLayerLock => layer_h::toggle_selected_layer_lock(self),
            Message::SelectedLayerOpacityChanged(v) => {
                layer_h::selected_layer_opacity_changed(self, v)
            }
            Message::SelectedLayerTextChanged(t) => layer_h::selected_layer_text_changed(self, t),
            Message::SelectedLayerFontSizeChanged(s) => {
                layer_h::selected_layer_font_size_changed(self, s)
            }
            Message::SelectedLayerTextColorR(v) => layer_h::selected_layer_text_color_r(self, v),
            Message::SelectedLayerTextColorG(v) => layer_h::selected_layer_text_color_g(self, v),
            Message::SelectedLayerTextColorB(v) => layer_h::selected_layer_text_color_b(self, v),
            Message::SelectedLayerTextShadowToggled => {
                layer_h::selected_layer_text_shadow_toggled(self)
            }
            Message::SelectedLayerTextOutlineToggled => {
                layer_h::selected_layer_text_outline_toggled(self)
            }
            Message::SelectedLayerTextBoldToggled => {
                layer_h::selected_layer_text_bold_toggled(self)
            }
            Message::SelectedLayerTextItalicToggled => {
                layer_h::selected_layer_text_italic_toggled(self)
            }
            Message::SelectedLayerShapeFillR(v) => layer_h::selected_layer_shape_fill_r(self, v),
            Message::SelectedLayerShapeFillG(v) => layer_h::selected_layer_shape_fill_g(self, v),
            Message::SelectedLayerShapeFillB(v) => layer_h::selected_layer_shape_fill_b(self, v),
            Message::SelectedLayerShapeFillA(v) => layer_h::selected_layer_shape_fill_a(self, v),
            Message::SelectedLayerShapeStrokeWidthChanged(s) => {
                layer_h::selected_layer_shape_stroke_width_changed(self, s)
            }
            Message::SelectedLayerPositionXChanged(s) => {
                layer_h::selected_layer_position_x_changed(self, s)
            }
            Message::SelectedLayerPositionYChanged(s) => {
                layer_h::selected_layer_position_y_changed(self, s)
            }
            Message::SelectedLayerWidthChanged(s) => layer_h::selected_layer_width_changed(self, s),
            Message::SelectedLayerHeightChanged(s) => {
                layer_h::selected_layer_height_changed(self, s)
            }
            Message::LayerDragStarted(i) => layer_h::layer_drag_started(self, i),
            Message::LayerDragged(p) => layer_h::layer_dragged(self, p),
            Message::LayerDragEnded => layer_h::layer_drag_ended(self),
            Message::SelectedLayerFontFamilyChanged(v) => typo_h::font_family_changed(self, v),
            Message::SelectedLayerLineHeightChanged(v) => typo_h::line_height_changed(self, v),
            Message::SelectedLayerLetterSpacingChanged(v) => {
                typo_h::letter_spacing_changed(self, v)
            }
            Message::SelectedLayerTextTransform(t) => typo_h::text_transform_changed(self, t),
            Message::SelectedLayerGlowToggled => typo_h::glow_toggled(self),
            Message::SelectedLayerGlowColorR(v) => typo_h::glow_color_r(self, v),
            Message::SelectedLayerGlowColorG(v) => typo_h::glow_color_g(self, v),
            Message::SelectedLayerGlowColorB(v) => typo_h::glow_color_b(self, v),
            Message::SelectedLayerGlowRadiusChanged(v) => typo_h::glow_radius_changed(self, v),
            Message::SelectedLayerTextStrokeWidthChanged(v) => {
                typo_h::text_stroke_width_changed(self, v)
            }
            Message::SelectedLayerTextStrokeColorR(v) => typo_h::text_stroke_color_r(self, v),
            Message::SelectedLayerTextStrokeColorG(v) => typo_h::text_stroke_color_g(self, v),
            Message::SelectedLayerTextStrokeColorB(v) => typo_h::text_stroke_color_b(self, v),
            Message::SelectedLayerTextColorHex(v) => typo_h::text_color_hex(self, v),
            Message::BibleTranslationSelected(id) => bible_h::translation_selected(self, id),
            Message::BibleBookSelected(book) => bible_h::book_selected(self, book),
            Message::BibleChapterSelected(ch) => bible_h::chapter_selected(self, ch),
            Message::BibleVerseToggled(idx) => bible_h::verse_toggled(self, idx),
            Message::BibleSearchChanged(q) => bible_h::search_changed(self, q),
            Message::BibleVersesPerSlideChanged(n) => bible_h::verses_per_slide_changed(self, n),
            Message::BibleSendToPresentation => bible_h::send_to_presentation(self),
            Message::BibleImportFile => bible_h::import_file(self),
            Message::BibleDeleteTranslationClicked(id) => bible_h::delete_clicked(self, id),
            Message::BibleConfirmDeleteTranslation => bible_h::confirm_delete(self),
            Message::BibleCancelDeleteTranslation => bible_h::cancel_delete(self),
            Message::BibleSelectAll => bible_h::select_all(self),
            Message::BibleClearSelection => bible_h::clear_selection(self),
            Message::AudioPickFile => audio_h::pick_file(self),
            Message::AudioLoad(path) => audio_h::load(self, path),
            Message::AudioPlay => audio_h::play(self),
            Message::AudioPause => audio_h::pause(self),
            Message::AudioStop => audio_h::stop(self),
            Message::AudioVolumeChanged(v) => audio_h::volume_changed(self, v),
            Message::AudioToggleLoop => audio_h::toggle_loop(self),
            Message::AudioTogglePanel => audio_h::toggle_panel(self),
            Message::OutputSettingsOpen => outputs_h::open_settings(self),
            Message::OutputSettingsClose => outputs_h::close_settings(self),
            Message::OutputAddWindow => outputs_h::add_window(self),
            Message::OutputAddNdi => outputs_h::add_ndi(self),
            Message::OutputRemove(id) => outputs_h::remove(self, id),
            Message::OutputSetActive(id, v) => outputs_h::set_active(self, id, v),
            Message::OutputCycleContent(id) => outputs_h::cycle_content(self, id),
            Message::OutputSetResolution(id, w, h) => outputs_h::set_resolution(self, id, w, h),
            Message::OutputNewLabelChanged(s) => outputs_h::new_label_changed(self, s),
            Message::OutputNewNdiNameChanged(s) => outputs_h::new_ndi_name_changed(self, s),
            Message::ExportOppFile => ie_h::export_opp(self),
            Message::ImportOppFile => ie_h::import_opp_pick(self),
            Message::ImportOppFileChosen(path) => ie_h::import_opp_load(self, path),
            Message::ExportOpenLyrics => ie_h::export_openlyrics(self),
            Message::ImportOpenLyrics => ie_h::import_openlyrics_pick(self),
            Message::ImportOpenLyricsChosen(path) => ie_h::import_openlyrics_load(self, path),
            Message::ToggleImportExportPanel => ie_h::toggle_panel(self),
            Message::PropToggle(id) => props_h::prop_toggle(self, id),
            Message::PropRemove(id) => props_h::prop_remove(self, id),
            Message::PropAddText => props_h::prop_add_text(self),
            Message::PropAddImage => props_h::prop_add_image(self),
            Message::PropNewNameChanged(s) => props_h::new_name_changed(self, s),
            Message::LowerThirdTitleChanged(s) => props_h::lower_third_title_changed(self, s),
            Message::LowerThirdSubtitleChanged(s) => props_h::lower_third_subtitle_changed(self, s),
            Message::CreateLowerThird => props_h::create_lower_third(self),
            Message::ApplyLook(id) => props_h::apply_look(self, id),
            Message::RemoveLook(id) => props_h::remove_look(self, id),
            Message::SaveLook => props_h::save_look(self),
            Message::LookNameChanged(s) => props_h::look_name_changed(self, s),
            Message::SetMask(mask) => props_h::set_mask(self, mask),
            Message::TogglePropsPanel => props_h::toggle_panel(self),
            Message::ToggleTriggersPanel => trig_h::toggle_panel(self),
            Message::TriggerHttpStart => trig_h::http_start(self),
            Message::TriggerHttpStop => trig_h::http_stop(self),
            Message::TriggerHttpPortChanged(s) => trig_h::http_port_changed(self, s),
            Message::TriggerOscStart => trig_h::osc_start(self),
            Message::TriggerOscStop => trig_h::osc_stop(self),
            Message::TriggerOscPortChanged(s) => trig_h::osc_port_changed(self, s),
            Message::TriggerFired(action) => trig_h::trigger_fired(self, action),
            Message::MacroAdd => trig_h::macro_add(self),
            Message::MacroRemove(id) => trig_h::macro_remove(self, id),
            Message::MacroRun(id) => trig_h::macro_run(self, id),
            Message::MacroStop(id) => trig_h::macro_stop(self, id),
            Message::MacroToggleLoop(id) => trig_h::macro_toggle_loop(self, id),
            Message::MacroNameChanged(s) => trig_h::macro_name_changed(self, s),
            Message::RecordingStart => rec_h::start(self),
            Message::RecordingStop => rec_h::stop(self),
            Message::RecordingPathChanged(s) => rec_h::path_changed(self, s),
            Message::RecordingFpsChanged(s) => rec_h::fps_changed(self, s),
            Message::ToggleRecordingPanel => rec_h::toggle_panel(self),
        }
    }
}

impl MainWindow {
    pub fn title(&self, id: window::Id) -> String {
        if Some(id) == self.output_window_id {
            String::from("OpenPresenter — Output")
        } else if Some(id) == self.delete_confirm_window_id {
            String::from("Confirm Delete")
        } else if Some(id) == self.new_presentation_window_id {
            String::from("New Presentation")
        } else if Some(id) == self.shortcuts_window_id {
            String::from("Keyboard Shortcuts")
        } else {
            String::from("OpenPresenter")
        }
    }

    pub fn view_for_window(&self, id: window::Id) -> Element<'_, Message> {
        if Some(id) == self.output_window_id {
            let current = self
                .presenting_presentation
                .as_ref()
                .and_then(|p| p.slides.get(self.presenting_slide_index));
            let (from_slide, trans, progress) = match &self.presenting_transition {
                Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
                None => (None, Transition::Cut, 1.0),
            };
            output_window::view(
                current,
                from_slide,
                trans,
                progress,
                self.video_frame.as_ref(),
                self.output_black_screen,
            )
        } else if Some(id) == self.delete_confirm_window_id {
            self.delete_confirm_view()
        } else if Some(id) == self.new_presentation_window_id {
            self.new_presentation_view()
        } else if Some(id) == self.shortcuts_window_id {
            self.shortcuts_view()
        } else {
            self.view()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let nav = navbar::navbar(
            self.current_mode,
            self.ndi_output.is_some(),
            self.stage_display_active,
            self.reduce_motion,
            self.audio.panel_visible,
            self.triggers_panel_open,
            self.recording_manager.state == crate::recording::RecordingState::Recording,
        );

        let active_id = if self.current_mode == ViewMode::Edit {
            self.editing_presentation.as_ref().map(|p| p.id.as_str())
        } else {
            self.presenting_presentation.as_ref().map(|p| p.id.as_str())
        };
        let side = sidebar::sidebar(
            &self.presentations,
            &self.search_query,
            active_id,
            self.sidebar_tab,
            &self.lib_assets,
            self.selected_asset_id.as_deref(),
            &self.recently_used_asset_ids,
        );
        let content = self.main_content();
        let body: Element<'_, Message> = if self.current_mode == ViewMode::Plan {
            crate::ui::planning::planning_panel(
                &self.service.plans,
                self.service.editing.as_ref(),
                &self.service.name_edit,
                &self.presentations,
                &self.song.songs,
                self.service.active.as_ref().map(|p| p.id.as_str()),
                self.service.item_index,
            )
        } else if self.sidebar_tab == SidebarTab::Songs {
            crate::ui::songs::songs_panel(
                &self.song.songs,
                &self.song.search,
                self.song.editing.as_ref(),
                &self.song.edit_title,
                &self.song.edit_artist,
                &self.song.edit_copyright,
                &self.song.edit_ccli,
                &self.song.edit_key,
                &self.song.edit_bpm,
                self.sidebar_tab,
            )
        } else if self.sidebar_tab == SidebarTab::Bible {
            crate::ui::bible::bible_panel(
                &self.bible.translations,
                self.bible.selected_translation.as_deref(),
                &self.bible.books,
                self.bible.selected_book.as_deref(),
                &self.bible.chapters,
                self.bible.selected_chapter,
                &self.bible.verses,
                &self.bible.selected_verse_indices,
                &self.bible.search,
                self.bible.verses_per_slide,
                self.sidebar_tab,
            )
        } else {
            row![side, content]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let mut layers = column![nav, body];

        if self.audio.panel_visible {
            layers = layers.push(crate::ui::audio::audio_bar(
                self.audio.track.as_deref(),
                self.audio.playing,
                self.audio.volume,
                self.audio.looping,
            ));
        }

        if self.output_settings_open {
            layers = layers.push(crate::ui::settings::outputs::outputs_panel(
                self.output_manager.iter(),
                &self.new_output_label,
                &self.new_output_ndi_name,
            ));
        }

        if self.props_panel_open {
            layers = layers.push(crate::ui::props::props_panel(
                &self.prop_manager,
                &self.new_prop_name,
                &self.new_look_name,
                &self.lower_third_title,
                &self.lower_third_subtitle,
            ));
        }

        if self.triggers_panel_open {
            layers = layers.push(crate::ui::settings::triggers::triggers_panel(
                self.trigger_manager.http_running,
                &self.trigger_http_port_str,
                self.trigger_manager.osc_running,
                &self.trigger_osc_port_str,
                &self.trigger_manager.macros,
                &self.new_macro_name,
                &self.macro_running_ids,
            ));
        }

        if self.recording_panel_open {
            let elapsed_secs = self.recording_manager.elapsed().map(|d| d.as_secs());
            let frames = self.recording_manager.frames_captured();
            layers = layers.push(crate::ui::recording::recording_panel(
                self.recording_manager.state,
                &self.recording_manager.output_path,
                self.recording_manager.fps,
                elapsed_secs,
                frames,
            ));
        }

        if let Some(ref err) = self.error_message {
            layers = layers.push(error_toast(err, Message::DismissError));
        }

        container(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::base_style)
            .into()
    }

    fn main_content(&self) -> Element<'_, Message> {
        match self.current_mode {
            ViewMode::Edit => {
                if let Some(ref pres) = self.editing_presentation {
                    let layer_state = editor::inspector::LayerPanelState {
                        selected_layer_index: self.layer.selected_index,
                        editing_text: self.layer.text.clone(),
                        editing_font_size: self.layer.font_size.clone(),
                        editing_pos_x: self.layer.pos_x.clone(),
                        editing_pos_y: self.layer.pos_y.clone(),
                        editing_width: self.layer.width.clone(),
                        editing_height: self.layer.height.clone(),
                        editing_stroke_width: self.layer.stroke_width.clone(),
                        editing_font_family: self.layer.font_family.clone(),
                        editing_line_height: self.layer.line_height.clone(),
                        editing_letter_spacing: self.layer.letter_spacing.clone(),
                        editing_glow_radius: self.layer.glow_radius.clone(),
                        editing_text_stroke_width: self.layer.text_stroke_width.clone(),
                    };
                    editor::editor_view(
                        pres,
                        self.selected_slide_index,
                        self.inspector_tab,
                        &self.editing_slide_text,
                        &self.editing_slide_font_size,
                        &self.editing_transition_duration,
                        &self.editing_group_label,
                        &self.editing_slide_notes,
                        &self.themes,
                        self.selected_theme_id.as_deref(),
                        &self.new_theme_name,
                        self.media_player.as_ref().is_some_and(|p| p.is_playing()),
                        self.video_looping,
                        self.video_volume,
                        self.video_muted,
                        self.video_speed,
                        self.video_position,
                        self.media_player
                            .as_ref()
                            .map(|p| p.duration_secs())
                            .unwrap_or(0.0),
                        self.video_frame.as_ref(),
                        layer_state,
                    )
                } else {
                    self.welcome_view()
                }
            }
            ViewMode::Show => {
                if let Some(ref pres) = self.presenting_presentation {
                    if self.stage_display_active {
                        stage::stage_view(
                            pres,
                            self.presenting_slide_index,
                            self.presenting_transition.as_ref(),
                            self.clock_secs,
                            self.timer_secs,
                            self.timer_running,
                        )
                    } else {
                        presenter::presenting_view(
                            pres,
                            self.presenting_slide_index,
                            self.ndi_output.is_some(),
                            self.output_window_id.is_some(),
                            self.output_black_screen,
                            self.output_is_fullscreen,
                            self.show_output_settings,
                            &self.editing_output_screen_x,
                            &self.editing_output_screen_y,
                            self.app_config.output.auto_fullscreen,
                            self.presenting_transition.as_ref(),
                            self.video_frame.as_ref(),
                            self.recording_manager.state
                                == crate::recording::RecordingState::Recording,
                        )
                    }
                } else {
                    self.presenting_welcome_view()
                }
            }
            ViewMode::Plan => container(Space::new()).into(),
        }
    }

    fn welcome_view(&self) -> Element<'_, Message> {
        let hero = column![
            text("Welcome to OpenPresenter")
                .size(30)
                .color(theme::TEXT_PRIMARY),
            Space::new().height(12),
            text("Create or select a presentation from the library to get started.")
                .size(15)
                .color(theme::TEXT_SECONDARY),
            Space::new().height(32),
            button(text("+ New Presentation").size(14))
                .on_press(Message::NewPresentationClicked)
                .padding([11, 28])
                .style(theme::primary_button),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill);

        container(hero)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(theme::base_style)
            .into()
    }

    fn presenting_welcome_view(&self) -> Element<'_, Message> {
        let body = column![
            text("Show Mode").size(26).color(theme::TEXT_PRIMARY),
            Space::new().height(10),
            text("Select a presentation from the library to start presenting.")
                .size(14)
                .color(theme::TEXT_SECONDARY),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(theme::base_style)
            .into()
    }

    fn delete_confirm_view(&self) -> Element<'_, Message> {
        use iced::widget::{Space, button, column, row, text};
        let (kind, name, confirm_msg, cancel_msg): (&str, String, Message, Message) =
            if let Some(ref id) = self.delete_target_id {
                let n = self
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
                    Message::ConfirmDeleteSong,
                    Message::CancelDeleteSong,
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
                    Message::ConfirmDeleteServicePlan,
                    Message::CancelDeleteServicePlan,
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
            text_input("Presentation name...", &self.new_presentation_name)
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
            text("PRESENTER").size(10).color(theme::TEXT_MUTED),
            Space::new().height(4),
            shortcut("Right / Space", "Next slide"),
            shortcut("Left", "Previous slide"),
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
        let kb_sub =
            if self.current_mode == ViewMode::Show && self.presenting_presentation.is_some() {
                event::listen_with(|ev, _status, _id| {
                    if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = ev {
                        return match key {
                            keyboard::Key::Named(key::Named::ArrowRight)
                            | keyboard::Key::Named(key::Named::Space) => {
                                Some(Message::PresentingNextSlide)
                            }
                            keyboard::Key::Named(key::Named::ArrowLeft) => {
                                Some(Message::PresentingPrevSlide)
                            }
                            keyboard::Key::Named(key::Named::Escape) => Some(Message::BackToList),
                            _ => None,
                        };
                    }
                    None
                })
            } else {
                Subscription::none()
            };

        let anim_sub = if self.presenting_transition.is_some() {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        let video_sub = if self.media_player.as_ref().is_some_and(|p| p.is_playing()) {
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
            Subscription::run_with(TriggerSubKey(Arc::clone(&self.trigger_rx)), trigger_stream);

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
