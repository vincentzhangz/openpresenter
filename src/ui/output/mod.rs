use crate::domain::Transition;
pub mod matrix;
use crate::ndi::{FrameRate, NdiOutputLoop};
use crate::output::{NamedOutput, OutputContentRoute, OutputType};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SettingsTab, ViewMode};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Task,
    widget::{
        Column, Row, Space, button, column, container, row, scrollable, slider, text, text_input,
        toggler,
    },
};
use iced::{Size, window};
use iced_font_awesome::fa_icon_solid;

/// Messages owned by the Output feature module.
///
/// `ToggleOutputSettings` (global panel visibility toggle, emitted from the
/// Presenter view) stays as a root variant.
#[derive(Debug, Clone)]
pub enum Message {
    SettingsOpen,
    SettingsClose,
    SelectTab(SettingsTab),
    AddWindow,
    AddNdi,
    Remove(String),
    SetActive(String, bool),
    CycleContent(String),
    SetResolution(String, u32, u32),
    NewLabelChanged(String),
    NewNdiNameChanged(String),
    SetStartupMode(ViewMode),
    BrowseDbPath,
    RevealDbPath,
    NdiSourceNameChanged(String),
    NdiFrameRateChanged(u32),
    HttpPortChanged(String),
    OscPortChanged(String),
    SetGlobalTransitionType(Transition),
    SetGlobalTransitionDuration(u64),
    ToggleReduceMotion(bool),
    SetDefaultGridCols(usize),
    SaveConfig,
    ResetToDefaults,
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Output(msg)
}

/// Render the global preferences / settings panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    preferences_window(w)
}

/// Dispatch an output message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::SettingsOpen => open_settings(w),
        Message::SettingsClose => close_settings(w),
        Message::SelectTab(tab) => select_tab(w, tab),
        Message::AddWindow => add_window(w),
        Message::AddNdi => add_ndi(w),
        Message::Remove(id) => remove(w, id),
        Message::SetActive(id, v) => set_active(w, id, v),
        Message::CycleContent(id) => cycle_content(w, id),
        Message::SetResolution(id, width, height) => set_resolution(w, id, width, height),
        Message::NewLabelChanged(s) => new_label_changed(w, s),
        Message::NewNdiNameChanged(s) => new_ndi_name_changed(w, s),
        Message::SetStartupMode(mode) => set_startup_mode(w, mode),
        Message::BrowseDbPath => browse_db_path(w),
        Message::RevealDbPath => reveal_db_path(w),
        Message::NdiSourceNameChanged(s) => ndi_source_name_changed(w, s),
        Message::NdiFrameRateChanged(fps) => ndi_frame_rate_changed(w, fps),
        Message::HttpPortChanged(s) => http_port_changed(w, s),
        Message::OscPortChanged(s) => osc_port_changed(w, s),
        Message::SetGlobalTransitionType(t) => set_global_transition_type(w, t),
        Message::SetGlobalTransitionDuration(d) => set_global_transition_duration(w, d),
        Message::ToggleReduceMotion(v) => toggle_reduce_motion(w, v),
        Message::SetDefaultGridCols(c) => set_default_grid_cols(w, c),
        Message::SaveConfig => save_config(w),
        Message::ResetToDefaults => reset_to_defaults(w),
    }
}

pub(crate) fn open_settings(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.settings_open = true;
    w.output.settings_status_message = None;
    Task::none()
}

pub(crate) fn close_settings(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.settings_open = false;
    w.output.settings_status_message = None;
    Task::none()
}

pub(crate) fn select_tab(w: &mut MainWindow, tab: SettingsTab) -> Task<RootMessage> {
    w.output.settings_tab = tab;
    Task::none()
}

pub(crate) fn set_startup_mode(w: &mut MainWindow, mode: ViewMode) -> Task<RootMessage> {
    w.app_config.general.startup_mode = match mode {
        ViewMode::Show => "show".to_string(),
        ViewMode::Edit => "edit".to_string(),
    };
    let _ = w.app_config.save();
    w.output.settings_status_message = Some("Startup mode updated".to_string());
    Task::none()
}

pub(crate) fn browse_db_path(w: &mut MainWindow) -> Task<RootMessage> {
    let dest = rfd::FileDialog::new()
        .set_title("Select OpenPresenter Database")
        .add_filter("SQLite Database", &["db", "sqlite"])
        .pick_file();
    if let Some(path) = dest {
        w.app_config.db_path = path;
        let _ = w.app_config.save();
        w.output.settings_status_message = Some("Database location updated".to_string());
    }
    Task::none()
}

pub(crate) fn reveal_db_path(w: &mut MainWindow) -> Task<RootMessage> {
    let path = &w.app_config.db_path;
    let target = if path.exists() {
        path.clone()
    } else if let Some(p) = path.parent() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(".")
    };

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(&target)
        .spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,\"{}\"", target.display()))
        .spawn();

    #[cfg(target_os = "linux")]
    if let Some(parent) = target.parent().or(Some(&target)) {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let _ = target;

    Task::none()
}

pub(crate) fn ndi_source_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.app_config.ndi.source_name = name;
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn ndi_frame_rate_changed(w: &mut MainWindow, fps: u32) -> Task<RootMessage> {
    w.app_config.ndi.frame_rate = fps;
    let _ = w.app_config.save();
    w.output.settings_status_message = Some(format!("NDI frame rate set to {fps} FPS"));
    Task::none()
}

pub(crate) fn http_port_changed(w: &mut MainWindow, port_str: String) -> Task<RootMessage> {
    if let Ok(p) = port_str.trim().parse::<u16>() {
        w.app_config.triggers.http_port = p;
        let _ = w.app_config.save();
        w.output.settings_status_message = Some(format!("HTTP trigger port set to {p}"));
    }
    Task::none()
}

pub(crate) fn osc_port_changed(w: &mut MainWindow, port_str: String) -> Task<RootMessage> {
    if let Ok(p) = port_str.trim().parse::<u16>() {
        w.app_config.triggers.osc_port = p;
        let _ = w.app_config.save();
        w.output.settings_status_message = Some(format!("OSC listener port set to {p}"));
    }
    Task::none()
}

pub(crate) fn set_global_transition_type(
    w: &mut MainWindow,
    transition: Transition,
) -> Task<RootMessage> {
    w.presenting.global_transition = transition;
    w.app_config.transitions.default_type = match transition {
        Transition::Cut => "cut".to_string(),
        Transition::Fade { .. } => "fade".to_string(),
        Transition::Push { direction, .. } => format!("push_{direction}").to_lowercase(),
        Transition::Wipe { angle_deg, .. } => format!("wipe_{angle_deg}").to_lowercase(),
        Transition::Zoom { .. } => "zoom".to_string(),
        Transition::Slide { .. } => "slide".to_string(),
        Transition::Dissolve { .. } => "dissolve".to_string(),
        Transition::Flip { .. } => "flip".to_string(),
        Transition::Clock { .. } => "clock".to_string(),
    };
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn set_global_transition_duration(
    w: &mut MainWindow,
    duration_ms: u64,
) -> Task<RootMessage> {
    let dur = duration_ms.clamp(50, 5000);
    w.app_config.transitions.default_duration_ms = dur;
    w.presenting.global_transition = w.presenting.global_transition.with_duration(dur);
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn toggle_reduce_motion(w: &mut MainWindow, enabled: bool) -> Task<RootMessage> {
    w.ui.reduce_motion = enabled;
    w.app_config.ui.reduce_motion = enabled;
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn set_default_grid_cols(w: &mut MainWindow, cols: usize) -> Task<RootMessage> {
    let c = cols.clamp(2, 6);
    w.presenting.slide_grid_cols = c;
    w.app_config.ui.default_grid_cols = c;
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn save_config(w: &mut MainWindow) -> Task<RootMessage> {
    match w.app_config.save() {
        Ok(_) => w.output.settings_status_message = Some("Settings saved successfully".to_string()),
        Err(e) => w.set_error(format!("Failed to save settings: {e}")),
    }
    Task::none()
}

pub(crate) fn reset_to_defaults(w: &mut MainWindow) -> Task<RootMessage> {
    w.app_config = crate::config::Config::default();
    let _ = w.app_config.save();
    w.output.screen_x = w.app_config.output.screen_x.to_string();
    w.output.screen_y = w.app_config.output.screen_y.to_string();
    w.ui.reduce_motion = false;
    w.presenting.slide_grid_cols = 4;
    w.presenting.global_transition = Transition::Fade { duration_ms: 500 };
    w.output.settings_status_message = Some("Reset all settings to factory defaults".to_string());
    Task::none()
}

pub(crate) fn add_window(w: &mut MainWindow) -> Task<RootMessage> {
    let label = w.output.new_label.trim().to_string();
    if label.is_empty() {
        return Task::none();
    }
    let id = label.to_lowercase().replace(' ', "_");
    w.output.manager.add(NamedOutput::new_window(id, label));
    w.output.new_label.clear();
    Task::none()
}

pub(crate) fn add_ndi(w: &mut MainWindow) -> Task<RootMessage> {
    let label = w.output.new_label.trim().to_string();
    if label.is_empty() {
        return Task::none();
    }
    let stream_name = if w.output.new_ndi_name.trim().is_empty() {
        label.clone()
    } else {
        w.output.new_ndi_name.trim().to_string()
    };
    let id = label.to_lowercase().replace(' ', "_") + "_ndi";
    w.output
        .manager
        .add(NamedOutput::new_ndi(id, label, stream_name));
    w.output.new_label.clear();
    w.output.new_ndi_name.clear();
    Task::none()
}

pub(crate) fn remove(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    if id == "main" {
        return Task::none();
    }
    w.output.manager.remove(&id);
    Task::none()
}

pub(crate) fn set_active(w: &mut MainWindow, id: String, active: bool) -> Task<RootMessage> {
    w.output.manager.set_active(&id, active);
    Task::none()
}

pub(crate) fn cycle_content(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    if let Some(output) = w.output.manager.get(&id) {
        let next = match &output.content {
            OutputContentRoute::LiveSlide => OutputContentRoute::Stage,
            OutputContentRoute::Stage => OutputContentRoute::Blank,
            OutputContentRoute::Blank => OutputContentRoute::LiveSlide,
            OutputContentRoute::Mirror { .. } => OutputContentRoute::LiveSlide,
        };
        w.output.manager.set_content(&id, next);
    }
    Task::none()
}

pub(crate) fn set_resolution(
    w: &mut MainWindow,
    id: String,
    width: u32,
    height: u32,
) -> Task<RootMessage> {
    w.output.manager.set_resolution(&id, width, height);
    Task::none()
}

pub(crate) fn new_label_changed(w: &mut MainWindow, label: String) -> Task<RootMessage> {
    w.output.new_label = label;
    Task::none()
}

pub(crate) fn new_ndi_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.output.new_ndi_name = name;
    Task::none()
}

pub(crate) fn toggle_ndi(w: &mut MainWindow) -> Task<RootMessage> {
    if w.presenting.ndi_output.is_some() {
        if let Some(ndi) = w.presenting.ndi_output.take() {
            ndi.stop();
        }
    } else {
        match NdiOutputLoop::start("OpenPresenter", 1920, 1080, FrameRate::FPS_30) {
            Ok(ndi) => {
                let slide = w
                    .presenting
                    .presentation
                    .as_ref()
                    .and_then(|p| p.slides.get(w.presenting.slide_index))
                    .cloned();
                if let Some(s) = slide {
                    ndi.set_slide(s);
                }
                w.presenting.ndi_output = Some(ndi);
            }
            Err(e) => w.set_error(format!("Failed to start NDI: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn ndi_send_current(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref pres) = w.presenting.presentation
        && let Some(slide) = pres.slides.get(w.presenting.slide_index)
        && let Some(ref ndi) = w.presenting.ndi_output
    {
        ndi.set_slide(slide.clone());
    }
    Task::none()
}

pub(crate) fn ndi_black_screen(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref ndi) = w.presenting.ndi_output {
        ndi.black_screen();
    }
    Task::none()
}

pub(crate) fn open_output_window(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.output.window_id {
        return window::gain_focus(id);
    }
    let cfg = &w.app_config.output;
    let pos = if cfg.screen_x != 0.0 || cfg.screen_y != 0.0 {
        window::Position::Specific(iced::Point::new(cfg.screen_x, cfg.screen_y))
    } else {
        window::Position::Default
    };
    let (id, open_task) = window::open(window::Settings {
        size: Size::new(cfg.width as f32, cfg.height as f32),
        position: pos,
        decorations: false,
        exit_on_close_request: false,
        ..Default::default()
    });
    w.output.window_id = Some(id);
    w.output.is_fullscreen = false;

    if cfg.auto_fullscreen {
        w.output.is_fullscreen = true;
        Task::batch([
            open_task.map(|_| RootMessage::OutputWindowOpened),
            window::set_mode(id, window::Mode::Fullscreen),
        ])
    } else {
        open_task.map(|_| RootMessage::OutputWindowOpened)
    }
}

pub(crate) fn close_output_window(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.output.window_id.take() {
        w.output.black_screen = false;
        w.output.is_fullscreen = false;
        return window::close(id);
    }
    Task::none()
}

pub(crate) fn toggle_output_black_screen(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.black_screen = !w.output.black_screen;
    if w.output.black_screen {
        ndi_black_screen(w)
    } else {
        ndi_send_current(w)
    }
}

pub(crate) fn output_fullscreen_toggled(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(id) = w.output.window_id else {
        return Task::none();
    };
    if w.output.is_fullscreen {
        w.output.is_fullscreen = false;
        window::set_mode(id, window::Mode::Windowed)
    } else {
        w.output.is_fullscreen = true;
        window::set_mode(id, window::Mode::Fullscreen)
    }
}

pub(crate) fn toggle_output_settings(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.show_settings = !w.output.show_settings;
    Task::none()
}

pub(crate) fn output_screen_x_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    if let Ok(x) = v.parse::<f32>() {
        w.app_config.output.screen_x = x;
        let _ = w.app_config.save();
    }
    w.output.screen_x = v;
    Task::none()
}

pub(crate) fn output_screen_y_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    if let Ok(y) = v.parse::<f32>() {
        w.app_config.output.screen_y = y;
        let _ = w.app_config.save();
    }
    w.output.screen_y = v;
    Task::none()
}

pub(crate) fn output_auto_fullscreen_toggled(
    w: &mut MainWindow,
    enabled: bool,
) -> Task<RootMessage> {
    w.app_config.output.auto_fullscreen = enabled;
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn window_closed(w: &mut MainWindow, id: window::Id) -> Task<RootMessage> {
    if Some(id) == w.output.window_id {
        w.output.window_id = None;
        w.output.black_screen = false;
        w.output.is_fullscreen = false;
        return window::close(id);
    }
    if Some(id) == w.editor.delete_confirm_window_id {
        w.editor.delete_confirm_window_id = None;
        w.editor.show_delete_confirmation = false;
        w.editor.delete_target_id = None;
        w.song.to_delete = None;
        w.service.to_delete = None;
        return Task::none();
    }
    if Some(id) == w.editor.new_presentation_window_id {
        w.editor.new_presentation_window_id = None;
        w.editor.new_presentation_name.clear();
        return Task::none();
    }
    if Some(id) == w.ui.shortcuts_window_id {
        w.ui.shortcuts_window_id = None;
        return Task::none();
    }
    if id == w.main_window_id {
        return iced::exit();
    }
    Task::none()
}

pub fn preferences_window<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let current_tab = w.output.settings_tab;

    let icon_el = fa_icon_solid("gear")
        .size(16.0_f32)
        .color(theme::ACCENT_ORANGE);
    let title_el = text("PREFERENCES").size(15).color(theme::TEXT_PRIMARY);

    let status_el: Element<'a, RootMessage> =
        if let Some(ref msg) = w.output.settings_status_message {
            container(
                row![
                    fa_icon_solid("circle-check")
                        .size(11.0_f32)
                        .color(theme::LIVE_GREEN),
                    Space::new().width(6),
                    text(msg.clone()).size(11).color(theme::LIVE_GREEN),
                ]
                .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(0.2, 0.78, 0.35, 0.12))),
                border: Border {
                    color: Color::from_rgba(0.2, 0.78, 0.35, 0.4),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            Space::new().width(0).into()
        };

    let close_btn = button(fa_icon_solid("xmark").size(14.0_f32))
        .on_press(wrap(Message::SettingsClose))
        .padding([6, 10])
        .style(theme::ghost_button);

    let header = row![
        icon_el,
        title_el,
        Space::new().width(12),
        status_el,
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 14]);

    let tab_bar = row![
        settings_tab_btn(SettingsTab::General, current_tab, "sliders", "General"),
        settings_tab_btn(
            SettingsTab::Screens,
            current_tab,
            "desktop",
            "Screens & Outputs"
        ),
        settings_tab_btn(
            SettingsTab::NetworkNdi,
            current_tab,
            "network-wired",
            "Network & NDI"
        ),
        settings_tab_btn(
            SettingsTab::Transitions,
            current_tab,
            "wand-magic-sparkles",
            "Transitions"
        ),
        settings_tab_btn(SettingsTab::Advanced, current_tab, "microchip", "Advanced"),
    ]
    .spacing(4)
    .padding([0, 14]);

    let body: Element<'a, RootMessage> = match current_tab {
        SettingsTab::General => tab_general(w),
        SettingsTab::Screens => tab_screens(w),
        SettingsTab::NetworkNdi => tab_network(w),
        SettingsTab::Transitions => tab_transitions(w),
        SettingsTab::Advanced => tab_advanced(w),
    };

    let config_path_str = crate::config::Config::config_path()
        .to_string_lossy()
        .into_owned();
    let config_note = row![
        fa_icon_solid("file-lines")
            .size(11.0_f32)
            .color(theme::TEXT_MUTED),
        Space::new().width(6),
        text(format!("Config: {config_path_str}"))
            .size(10)
            .color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    let save_btn = button(
        row![
            fa_icon_solid("floppy-disk").size(11.0_f32),
            Space::new().width(6),
            text("Save Settings").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::SaveConfig))
    .padding([6, 12])
    .style(theme::primary_button);

    let done_btn = button(text("Close").size(11))
        .on_press(wrap(Message::SettingsClose))
        .padding([6, 12])
        .style(theme::ghost_button);

    let footer = row![
        config_note,
        Space::new().width(Length::Fill),
        save_btn,
        done_btn,
    ]
    .spacing(8)
    .padding([10, 14])
    .align_y(Alignment::Center);

    container(
        column![
            header,
            tab_bar,
            divider_line(),
            scrollable(body).height(Length::Fill).width(Length::Fill),
            divider_line(),
            footer,
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn tab_general<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let mode_str = w.app_config.general.startup_mode.as_str();

    let mode_show = mode_picker_btn(
        "Show (Live Presentation)",
        ViewMode::Show,
        mode_str == "show",
    );
    let mode_edit = mode_picker_btn("Edit (Slide Editor)", ViewMode::Edit, mode_str == "edit");

    let startup_card = section_card(
        "STARTUP WORKSPACE",
        "desktop",
        column![
            text("Choose which workspace opens when launching OpenPresenter:")
                .size(12)
                .color(theme::TEXT_SECONDARY),
            row![mode_show, mode_edit].spacing(8),
        ]
        .spacing(8),
    );

    let db_path_str = w.app_config.db_path.to_string_lossy().into_owned();
    let db_box = container(text(db_path_str).size(11).color(theme::TEXT_PRIMARY))
        .padding([6, 10])
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb(0.06, 0.06, 0.07))),
            border: Border {
                color: theme::BORDER_PANEL,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

    let browse_btn = button(
        row![
            fa_icon_solid("folder-open").size(11.0_f32),
            Space::new().width(6),
            text("Change Location...").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::BrowseDbPath))
    .padding([6, 10])
    .style(theme::secondary_button);

    let reveal_btn = button(
        row![
            fa_icon_solid("arrow-up-right-from-square").size(11.0_f32),
            Space::new().width(6),
            text("Reveal in File Manager").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::RevealDbPath))
    .padding([6, 10])
    .style(theme::ghost_button);

    let stats_row = row![
        badge_pill("SQLite Connected", theme::LIVE_GREEN),
        Space::new().width(10),
        text(format!("{} Presentations", w.editor.presentations.len()))
            .size(11)
            .color(theme::TEXT_MUTED),
        text(" • ").size(11).color(theme::TEXT_MUTED),
        text(format!("{} Songs", w.song.songs.len()))
            .size(11)
            .color(theme::TEXT_MUTED),
        text(" • ").size(11).color(theme::TEXT_MUTED),
        text(format!("{} Media Assets", w.library.assets.len()))
            .size(11)
            .color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    let db_card = section_card(
        "DATABASE & LIBRARY STORAGE",
        "database",
        column![
            text("SQLite storage file holding all presentations, songs, playlists, and metadata:")
                .size(12)
                .color(theme::TEXT_SECONDARY),
            db_box,
            row![
                browse_btn,
                reveal_btn,
                Space::new().width(Length::Fill),
                stats_row,
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        ]
        .spacing(8),
    );

    column![startup_card, db_card]
        .spacing(12)
        .padding([8, 14])
        .into()
}

fn tab_screens<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let screen_x_input = text_input("0.0", &w.output.screen_x)
        .on_input(RootMessage::OutputScreenXChanged)
        .padding([4, 8])
        .width(90);

    let screen_y_input = text_input("0.0", &w.output.screen_y)
        .on_input(RootMessage::OutputScreenYChanged)
        .padding([4, 8])
        .width(90);

    let fullscreen_toggle = toggler(w.app_config.output.auto_fullscreen)
        .label("Auto-Fullscreen secondary display")
        .on_toggle(RootMessage::OutputAutoFullscreenToggled);

    let res_preset_1080 = button(text("1080p (1920×1080)").size(11))
        .on_press(wrap(Message::SetResolution(
            "audience".to_string(),
            1920,
            1080,
        )))
        .padding([4, 8])
        .style(theme::ghost_button);

    let res_preset_720 = button(text("720p (1280×720)").size(11))
        .on_press(wrap(Message::SetResolution(
            "audience".to_string(),
            1280,
            720,
        )))
        .padding([4, 8])
        .style(theme::ghost_button);

    let res_preset_4k = button(text("4K (3840×2160)").size(11))
        .on_press(wrap(Message::SetResolution(
            "audience".to_string(),
            3840,
            2160,
        )))
        .padding([4, 8])
        .style(theme::ghost_button);

    let screen_offsets_card = section_card(
        "PHYSICAL DISPLAY PLACEMENT",
        "display",
        column![
            row![
                text("Screen X:").size(12).color(theme::TEXT_SECONDARY),
                screen_x_input,
                Space::new().width(12),
                text("Screen Y:").size(12).color(theme::TEXT_SECONDARY),
                screen_y_input,
                Space::new().width(16),
                fullscreen_toggle,
            ]
            .align_y(Alignment::Center)
            .spacing(6),
            row![
                text("Quick Resolution Presets:")
                    .size(11)
                    .color(theme::TEXT_MUTED),
                res_preset_1080,
                res_preset_720,
                res_preset_4k,
            ]
            .align_y(Alignment::Center)
            .spacing(6),
        ]
        .spacing(8),
    );

    let mut output_rows: Column<'a, RootMessage> = column![].spacing(4);
    for output in w.output.manager.iter() {
        output_rows = output_rows.push(output_row(output));
    }

    let add_form = add_output_form(&w.output.new_label, &w.output.new_ndi_name);

    let routing_card = section_card(
        "OUTPUT DESTINATIONS & KEY/FILL ROUTING",
        "network-wired",
        column![output_rows, add_form,].spacing(8),
    );

    column![screen_offsets_card, routing_card]
        .spacing(12)
        .padding([8, 14])
        .into()
}

fn tab_network<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let ndi_source_input = text_input("OpenPresenter", &w.app_config.ndi.source_name)
        .on_input(|v| wrap(Message::NdiSourceNameChanged(v)))
        .padding([4, 8])
        .width(180);

    let fps_30_active = w.app_config.ndi.frame_rate == 30;
    let fps_60_active = w.app_config.ndi.frame_rate == 60;

    let fps_30 = button(text("30 FPS").size(11))
        .on_press(wrap(Message::NdiFrameRateChanged(30)))
        .padding([4, 10])
        .style(if fps_30_active {
            theme::primary_button
        } else {
            theme::ghost_button
        });

    let fps_60 = button(text("60 FPS").size(11))
        .on_press(wrap(Message::NdiFrameRateChanged(60)))
        .padding([4, 10])
        .style(if fps_60_active {
            theme::primary_button
        } else {
            theme::ghost_button
        });

    let ndi_is_live = w.presenting.ndi_output.is_some();
    let ndi_toggle_btn = button(
        row![
            fa_icon_solid(if ndi_is_live { "stop" } else { "play" }).size(11.0_f32),
            Space::new().width(6),
            text(if ndi_is_live {
                "Stop NDI Broadcast"
            } else {
                "Start NDI Broadcast"
            })
            .size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(RootMessage::ToggleNdi)
    .padding([5, 12])
    .style(if ndi_is_live {
        theme::danger_button
    } else {
        theme::primary_button
    });

    let ndi_status_badge = if ndi_is_live {
        badge_pill("NDI BROADCASTING", theme::LIVE_GREEN)
    } else {
        badge_pill("NDI STANDBY", theme::TEXT_MUTED)
    };

    let ndi_card = section_card(
        "NDI NETWORK VIDEO BROADCAST",
        "video",
        column![
            row![
                text("Source Name:").size(12).color(theme::TEXT_SECONDARY),
                ndi_source_input,
                Space::new().width(12),
                text("Frame Rate:").size(12).color(theme::TEXT_SECONDARY),
                fps_30,
                fps_60,
                Space::new().width(Length::Fill),
                ndi_status_badge,
                ndi_toggle_btn,
            ]
            .align_y(Alignment::Center)
            .spacing(6),
            text("Zero-configuration broadcast stream discoverable on the local network by OBS Studio, vMix, TriCaster, and standard video switchers.")
                .size(11)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(8)
    );

    let http_port_str = w.app_config.triggers.http_port.to_string();
    let http_input = text_input("9090", &http_port_str)
        .on_input(|v| wrap(Message::HttpPortChanged(v)))
        .padding([4, 8])
        .width(80);

    let osc_port_str = w.app_config.triggers.osc_port.to_string();
    let osc_input = text_input("9000", &osc_port_str)
        .on_input(|v| wrap(Message::OscPortChanged(v)))
        .padding([4, 8])
        .width(80);

    let triggers_card = section_card(
        "REMOTE CONTROL & AUTOMATION TRIGGERS",
        "bolt",
        column![
            row![
                text("HTTP REST API Port:")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
                http_input,
                Space::new().width(16),
                text("OSC UDP Port:").size(12).color(theme::TEXT_SECONDARY),
                osc_input,
                Space::new().width(Length::Fill),
                badge_pill("TRIGGERS ACTIVE", theme::LIVE_GREEN),
            ]
            .align_y(Alignment::Center)
            .spacing(6),
            column![
                text("HTTP REST API Endpoints:")
                    .size(11)
                    .color(theme::TEXT_MUTED),
                text("  • POST /action/next          — Advance to next slide")
                    .size(10)
                    .color(theme::TEXT_MUTED),
                text("  • POST /action/prev          — Return to previous slide")
                    .size(10)
                    .color(theme::TEXT_MUTED),
                text("  • POST /action/clear         — Clear live slide output")
                    .size(10)
                    .color(theme::TEXT_MUTED),
                text("  • POST /action/slide/:index  — Jump directly to slide index")
                    .size(10)
                    .color(theme::TEXT_MUTED),
                Space::new().height(4),
                text("OSC (Open Sound Control) UDP Commands:")
                    .size(11)
                    .color(theme::TEXT_MUTED),
                text("  • /next, /prev, /clear, /black, /slide/N")
                    .size(10)
                    .color(theme::TEXT_MUTED),
            ]
            .spacing(2),
        ]
        .spacing(8),
    );

    column![ndi_card, triggers_card]
        .spacing(12)
        .padding([8, 14])
        .into()
}

fn tab_transitions<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let dur = w.presenting.global_transition.duration_ms();

    let transitions_list: &[(&str, Transition)] = &[
        ("Cut", Transition::Cut),
        (
            "Fade",
            Transition::Fade {
                duration_ms: dur.max(100),
            },
        ),
        (
            "Push Left",
            Transition::Push {
                duration_ms: dur.max(100),
                direction: 0,
            },
        ),
        (
            "Push Right",
            Transition::Push {
                duration_ms: dur.max(100),
                direction: 1,
            },
        ),
        (
            "Wipe Left",
            Transition::Wipe {
                duration_ms: dur.max(100),
                angle_deg: 0,
            },
        ),
        (
            "Wipe Right",
            Transition::Wipe {
                duration_ms: dur.max(100),
                angle_deg: 180,
            },
        ),
        (
            "Zoom",
            Transition::Zoom {
                duration_ms: dur.max(100),
            },
        ),
    ];

    let mut trans_buttons = Row::new().spacing(6).align_y(Alignment::Center);
    for (name, trans) in transitions_list {
        let is_selected = match (&w.presenting.global_transition, trans) {
            (Transition::Cut, Transition::Cut) => true,
            (Transition::Fade { .. }, Transition::Fade { .. }) => true,
            (Transition::Push { direction: d1, .. }, Transition::Push { direction: d2, .. }) => {
                d1 == d2
            }
            (Transition::Wipe { angle_deg: a1, .. }, Transition::Wipe { angle_deg: a2, .. }) => {
                a1 == a2
            }
            (Transition::Zoom { .. }, Transition::Zoom { .. }) => true,
            _ => false,
        };

        trans_buttons = trans_buttons.push(
            button(text(*name).size(11))
                .on_press(wrap(Message::SetGlobalTransitionType(*trans)))
                .padding([5, 10])
                .style(if is_selected {
                    theme::primary_button
                } else {
                    theme::ghost_button
                }),
        );
    }

    let dur_slider = slider(50..=2000, dur as u32, |v| {
        wrap(Message::SetGlobalTransitionDuration(v as u64))
    })
    .step(25u32)
    .width(260);

    let dur_readout = text(format!("{dur} ms"))
        .size(12)
        .color(theme::TEXT_PRIMARY);

    let transition_card = section_card(
        "GLOBAL SLIDE TRANSITION",
        "wand-magic-sparkles",
        column![
            text("Default transition applied when advancing slides in Show and Presenter views:")
                .size(12)
                .color(theme::TEXT_SECONDARY),
            trans_buttons,
            row![
                text("Duration:").size(12).color(theme::TEXT_SECONDARY),
                dur_slider,
                dur_readout,
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        ]
        .spacing(10),
    );

    let media_card = section_card(
        "MEDIA PLAYBACK DEFAULTS",
        "photo-film",
        column![
            row![
                badge_pill("GPU TEXT PIPELINE ACTIVE", theme::ACCENT_ORANGE),
                Space::new().width(12),
                badge_pill("RODIO AUDIO ENGINE (44.1/48kHz)", theme::LIVE_GREEN),
            ]
            .align_y(Alignment::Center),
            text("Media playback supports MP4, WebM, MOV video via FFmpeg and MP3, WAV, FLAC, Vorbis audio.")
                .size(11)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(8),
    );

    column![transition_card, media_card]
        .spacing(12)
        .padding([8, 14])
        .into()
}

fn tab_advanced<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let motion_toggle = toggler(w.ui.reduce_motion)
        .label("Reduce Motion animations")
        .on_toggle(|v| wrap(Message::ToggleReduceMotion(v)));

    let cols = w.presenting.slide_grid_cols as u32;
    let cols_slider = slider(2..=6, cols, |v| {
        wrap(Message::SetDefaultGridCols(v as usize))
    })
    .width(180);
    let cols_label = text(format!("{cols} columns"))
        .size(12)
        .color(theme::TEXT_PRIMARY);

    let ui_card = section_card(
        "ACCESSIBILITY & PERFORMANCE",
        "gauge-high",
        column![
            motion_toggle,
            text("Suppresses decorative sliding animations for lower CPU/GPU overhead.")
                .size(11)
                .color(theme::TEXT_MUTED),
            row![
                text("Default Slide Grid Zoom:")
                    .size(12)
                    .color(theme::TEXT_SECONDARY),
                cols_slider,
                cols_label,
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        ]
        .spacing(8),
    );

    let reset_btn = button(
        row![
            fa_icon_solid("rotate-left").size(11.0_f32),
            Space::new().width(6),
            text("Reset All Settings to Factory Defaults").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::ResetToDefaults))
    .padding([6, 12])
    .style(theme::danger_button);

    let system_card = section_card(
        "SYSTEM & FACTORY RESET",
        "triangle-exclamation",
        column![
            text(
                "Restore all configuration options in config.toml back to factory default values."
            )
            .size(12)
            .color(theme::TEXT_SECONDARY),
            reset_btn,
            divider_line(),
            row![
                text("OpenPresenter v0.1.0")
                    .size(11)
                    .color(theme::TEXT_MUTED),
                text(" • ").size(11).color(theme::TEXT_MUTED),
                text(format!("Target OS: {}", std::env::consts::OS))
                    .size(11)
                    .color(theme::TEXT_MUTED),
                text(" • ").size(11).color(theme::TEXT_MUTED),
                text(format!("Arch: {}", std::env::consts::ARCH))
                    .size(11)
                    .color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    );

    column![ui_card, system_card]
        .spacing(12)
        .padding([8, 14])
        .into()
}

fn settings_tab_btn<'a>(
    tab: SettingsTab,
    current: SettingsTab,
    icon: &'static str,
    label: &'static str,
) -> Element<'a, RootMessage> {
    let is_active = tab == current;
    button(
        row![
            fa_icon_solid(icon).size(12.0_f32).color(if is_active {
                theme::ACCENT_ORANGE
            } else {
                theme::TEXT_SECONDARY
            }),
            Space::new().width(6),
            text(label).size(12).color(if is_active {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            }),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::SelectTab(tab)))
    .padding([6, 12])
    .style(move |_t: &iced::Theme, status| {
        let bg = if is_active {
            Color::from_rgba(0.941, 0.216, 0.031, 0.16)
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            theme::BG_HOVER
        } else {
            theme::TRANSPARENT
        };
        iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: if is_active {
                    theme::ACCENT_ORANGE
                } else {
                    theme::BORDER_PANEL
                },
                width: if is_active { 1.5 } else { 1.0 },
                radius: 5.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn section_card<'a>(
    title: &'static str,
    icon: &'static str,
    content: impl Into<Element<'a, RootMessage>>,
) -> Element<'a, RootMessage> {
    let header = row![
        fa_icon_solid(icon)
            .size(12.0_f32)
            .color(theme::ACCENT_ORANGE),
        Space::new().width(6),
        text(title).size(11).color(theme::TEXT_MUTED),
    ]
    .align_y(Alignment::Center);

    container(column![header, content.into()].spacing(8))
        .width(Length::Fill)
        .padding(12)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_PANEL)),
            border: Border {
                color: theme::BORDER_PANEL,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn mode_picker_btn<'a>(
    label: &'static str,
    mode: ViewMode,
    is_active: bool,
) -> Element<'a, RootMessage> {
    button(
        row![
            fa_icon_solid(if is_active { "circle-check" } else { "circle" })
                .size(11.0_f32)
                .color(if is_active {
                    theme::ACCENT_ORANGE
                } else {
                    theme::TEXT_MUTED
                }),
            Space::new().width(6),
            text(label).size(11).color(if is_active {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            }),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::SetStartupMode(mode)))
    .padding([6, 12])
    .style(move |_t: &iced::Theme, status| {
        let bg = if is_active {
            Color::from_rgba(0.941, 0.216, 0.031, 0.12)
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            theme::BG_HOVER
        } else {
            theme::TRANSPARENT
        };
        iced::widget::button::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: if is_active {
                    theme::ACCENT_ORANGE
                } else {
                    theme::BORDER_PANEL
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn badge_pill<'a>(label: &'static str, color: Color) -> Element<'a, RootMessage> {
    container(text(label).size(9).color(color))
        .padding([2, 6])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color { a: 0.12, ..color })),
            border: Border {
                color,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn divider_line<'a>() -> Element<'a, RootMessage> {
    container(Space::new().height(1.0).width(Length::Fill))
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BORDER_PANEL)),
            ..Default::default()
        })
        .into()
}

fn output_row<'a>(output: &'a NamedOutput) -> Element<'a, RootMessage> {
    let type_label = match &output.output_type {
        OutputType::Window => "Window".to_string(),
        OutputType::Ndi { stream_name } => format!("NDI: {stream_name}"),
    };

    let resolution = format!("{}×{}", output.width, output.height);

    let content_label = output.content.to_string();

    let content_btn = button(text(content_label).size(11.0))
        .on_press(wrap(Message::CycleContent(output.id.clone())))
        .padding([3, 8])
        .style(theme::ghost_button);

    let res_720 = button(text("720p").size(11.0))
        .on_press(wrap(Message::SetResolution(output.id.clone(), 1280, 720)))
        .padding([3, 6])
        .style(theme::ghost_button);
    let res_1080 = button(text("1080p").size(11.0))
        .on_press(wrap(Message::SetResolution(output.id.clone(), 1920, 1080)))
        .padding([3, 6])
        .style(theme::ghost_button);

    let active_toggle =
        toggler(output.active).on_toggle(move |v| wrap(Message::SetActive(output.id.clone(), v)));

    let delete_btn = button(text("X").size(11.0))
        .on_press(wrap(Message::Remove(output.id.clone())))
        .padding([3, 6])
        .style(theme::danger_button);

    let row = row![
        active_toggle,
        text(&output.label).size(13.0).width(120),
        text(type_label).size(11.0).width(100),
        text(resolution).size(11.0).width(70),
        content_btn,
        Space::new().width(Length::Fill),
        res_720,
        res_1080,
        delete_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(4)
    .padding([4, 0]);

    container(row)
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn add_output_form<'a>(label: &'a str, ndi_name: &'a str) -> Element<'a, RootMessage> {
    let label_input = text_input("Output label…", label)
        .on_input(move |v| wrap(Message::NewLabelChanged(v)))
        .padding([4, 8])
        .width(160);

    let ndi_input = text_input("NDI stream name (optional)…", ndi_name)
        .on_input(move |v| wrap(Message::NewNdiNameChanged(v)))
        .padding([4, 8])
        .width(200);

    let add_window_btn = button(text("+ Window Output").size(12.0))
        .on_press(wrap(Message::AddWindow))
        .padding([5, 10])
        .style(theme::primary_button);

    let add_ndi_btn = button(text("+ NDI Output").size(12.0))
        .on_press(wrap(Message::AddNdi))
        .padding([5, 10])
        .style(theme::ghost_button);

    row![label_input, ndi_input, add_window_btn, add_ndi_btn,]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}
