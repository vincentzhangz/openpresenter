use crate::domain::{ImageFit, SlideContent};
use crate::ndi::{FrameRate, NdiOutputLoop};
use crate::output::{NamedOutput, OutputContentRoute, OutputType};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab, ViewMode};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Task,
    widget::{
        Column, Space, button, column, container, row, scrollable, text, text_input, toggler,
    },
};
use iced::{Size, window};
use std::time::{SystemTime, UNIX_EPOCH};

/// Messages owned by the Output feature module (see `AGENTS.md`).
///
/// `ToggleOutputSettings` (global panel visibility toggle, emitted from the
/// Presenter view) stays as a root variant.
#[derive(Debug, Clone)]
pub enum Message {
    SettingsOpen,
    SettingsClose,
    AddWindow,
    AddNdi,
    Remove(String),
    SetActive(String, bool),
    CycleContent(String),
    SetResolution(String, u32, u32),
    NewLabelChanged(String),
    NewNdiNameChanged(String),
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Output(msg)
}

/// Render the output settings panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    outputs_panel(
        w.output.manager.iter(),
        &w.output.new_label,
        &w.output.new_ndi_name,
    )
}

/// Dispatch an output message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::SettingsOpen => open_settings(w),
        Message::SettingsClose => close_settings(w),
        Message::AddWindow => add_window(w),
        Message::AddNdi => add_ndi(w),
        Message::Remove(id) => remove(w, id),
        Message::SetActive(id, v) => set_active(w, id, v),
        Message::CycleContent(id) => cycle_content(w, id),
        Message::SetResolution(id, width, height) => set_resolution(w, id, width, height),
        Message::NewLabelChanged(s) => new_label_changed(w, s),
        Message::NewNdiNameChanged(s) => new_ndi_name_changed(w, s),
    }
}

pub(crate) fn open_settings(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.settings_open = true;
    Task::none()
}

pub(crate) fn close_settings(w: &mut MainWindow) -> Task<RootMessage> {
    w.output.settings_open = false;
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

pub(crate) fn toggle_stage_display(w: &mut MainWindow) -> Task<RootMessage> {
    if w.shell.current_mode == ViewMode::Show {
        w.presenting.stage_display_active = !w.presenting.stage_display_active;
    }
    Task::none()
}

pub(crate) fn clock_tick(w: &mut MainWindow) -> Task<RootMessage> {
    w.presenting.clock_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86400)
        .unwrap_or(0);
    if w.presenting.timer_running {
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        w.presenting.timer_secs = now_epoch.saturating_sub(w.presenting.timer_start_epoch);
    }
    Task::none()
}

pub(crate) fn toggle_timer(w: &mut MainWindow) -> Task<RootMessage> {
    if w.presenting.timer_running {
        w.presenting.timer_running = false;
    } else {
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        w.presenting.timer_start_epoch = now_epoch.saturating_sub(w.presenting.timer_secs);
        w.presenting.timer_running = true;
    }
    Task::none()
}

pub(crate) fn reset_timer(w: &mut MainWindow) -> Task<RootMessage> {
    w.presenting.timer_secs = 0;
    w.presenting.timer_running = false;
    w.presenting.timer_start_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Task::none()
}

pub(crate) fn switch_sidebar_tab(w: &mut MainWindow, tab: SidebarTab) -> Task<RootMessage> {
    w.shell.sidebar_tab = tab;
    if tab == SidebarTab::Library && w.library.assets.is_empty() {
        w.load_lib_assets();
    }
    Task::none()
}

pub(crate) fn library_import_asset(w: &mut MainWindow) -> Task<RootMessage> {
    let image_exts = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];
    let video_exts = ["mp4", "mov", "avi", "mkv", "webm"];
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Images & Videos",
            &[
                "png", "jpg", "jpeg", "gif", "bmp", "webp", "mp4", "mov", "avi", "mkv", "webm",
            ],
        )
        .set_title("Import Asset")
        .pick_file()
    {
        let path_str = path.to_string_lossy().into_owned();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let media_type = if image_exts.contains(&ext.as_str()) {
            "image"
        } else if video_exts.contains(&ext.as_str()) {
            "video"
        } else {
            "image"
        };
        match w.library.repo.add_asset(&path_str, media_type) {
            Ok(_) => w.load_lib_assets(),
            Err(e) => w.set_error(format!("library import: {e}")),
        }
        w.shell.sidebar_tab = SidebarTab::Library;
    }
    Task::none()
}

pub(crate) fn library_apply_to_slide(w: &mut MainWindow, asset_id: String) -> Task<RootMessage> {
    let asset = w.library.assets.iter().find(|a| a.id == asset_id).cloned();
    if let Some(asset) = asset {
        if let Some(slide) = w.get_current_slide_mut() {
            if asset.is_image() {
                slide.content = SlideContent::Image {
                    path: asset.path.clone(),
                    fit: ImageFit::default(),
                };
            } else {
                slide.content = SlideContent::Video {
                    path: asset.path.clone(),
                    thumbnail: None,
                };
            }
            let c = slide.clone();
            w.persist_slide(c);
        }
        w.library.recently_used_ids.retain(|id| id != &asset_id);
        w.library.recently_used_ids.insert(0, asset_id);
        w.library.recently_used_ids.truncate(6);
    }
    Task::none()
}

pub(crate) fn library_delete_asset(w: &mut MainWindow, asset_id: String) -> Task<RootMessage> {
    match w.library.repo.delete_asset(&asset_id) {
        Ok(_) => {
            w.library.assets.retain(|a| a.id != asset_id);
            w.library.recently_used_ids.retain(|id| id != &asset_id);
            if w.library.selected_id.as_deref() == Some(&asset_id) {
                w.library.selected_id = None;
            }
        }
        Err(e) => w.set_error(format!("library delete: {e}")),
    }
    Task::none()
}

pub(crate) fn library_select_asset(w: &mut MainWindow, asset_id: String) -> Task<RootMessage> {
    w.library.selected_id = if w.library.selected_id.as_deref() == Some(&asset_id) {
        None
    } else {
        Some(asset_id)
    };
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
    Task::none()
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

pub fn outputs_panel<'a>(
    outputs: impl Iterator<Item = &'a NamedOutput>,
    new_output_label: &'a str,
    new_output_ndi_name: &'a str,
) -> Element<'a, RootMessage> {
    let header = row![text("Outputs").size(18.0), Space::new().width(Length::Fill),]
        .align_y(Alignment::Center);

    let mut output_rows: Column<'a, RootMessage> = column![].spacing(4);
    for output in outputs {
        output_rows = output_rows.push(output_row(output));
    }

    let add_form = add_output_form(new_output_label, new_output_ndi_name);

    container(
        column![
            header,
            scrollable(output_rows).height(Length::Fill),
            add_form,
        ]
        .spacing(8)
        .padding(12),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
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
