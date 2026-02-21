use crate::ndi::{FrameRate, NdiOutputLoop};
use crate::slides::{ImageFit, SlideContent};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, SidebarTab, ViewMode};
use iced::{Size, Task, window};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn toggle_ndi(w: &mut MainWindow) -> Task<Message> {
    if w.ndi_output.is_some() {
        if let Some(ndi) = w.ndi_output.take() {
            ndi.stop();
        }
    } else {
        match NdiOutputLoop::start("OpenPresenter", 1920, 1080, FrameRate::FPS_30) {
            Ok(ndi) => {
                let slide = w
                    .presenting_presentation
                    .as_ref()
                    .and_then(|p| p.slides.get(w.presenting_slide_index))
                    .cloned();
                if let Some(s) = slide {
                    ndi.set_slide(s);
                }
                w.ndi_output = Some(ndi);
            }
            Err(e) => eprintln!("Failed to start NDI: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn ndi_send_current(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref pres) = w.presenting_presentation
        && let Some(slide) = pres.slides.get(w.presenting_slide_index)
        && let Some(ref ndi) = w.ndi_output
    {
        ndi.set_slide(slide.clone());
    }
    Task::none()
}

pub(crate) fn ndi_black_screen(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref ndi) = w.ndi_output {
        ndi.black_screen();
    }
    Task::none()
}

pub(crate) fn toggle_stage_display(w: &mut MainWindow) -> Task<Message> {
    if w.current_mode == ViewMode::Show {
        w.stage_display_active = !w.stage_display_active;
    }
    Task::none()
}

pub(crate) fn clock_tick(w: &mut MainWindow) -> Task<Message> {
    w.clock_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() % 86400)
        .unwrap_or(0);
    if w.timer_running {
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        w.timer_secs = now_epoch.saturating_sub(w.timer_start_epoch);
    }
    Task::none()
}

pub(crate) fn toggle_timer(w: &mut MainWindow) -> Task<Message> {
    if w.timer_running {
        w.timer_running = false;
    } else {
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        w.timer_start_epoch = now_epoch.saturating_sub(w.timer_secs);
        w.timer_running = true;
    }
    Task::none()
}

pub(crate) fn reset_timer(w: &mut MainWindow) -> Task<Message> {
    w.timer_secs = 0;
    w.timer_running = false;
    w.timer_start_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Task::none()
}

pub(crate) fn switch_sidebar_tab(w: &mut MainWindow, tab: SidebarTab) -> Task<Message> {
    w.sidebar_tab = tab;
    if tab == SidebarTab::Library && w.lib_assets.is_empty() {
        w.load_lib_assets();
    }
    Task::none()
}

pub(crate) fn library_import_asset(w: &mut MainWindow) -> Task<Message> {
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
        match w.lib_repo.add_asset(&path_str, media_type) {
            Ok(_) => w.load_lib_assets(),
            Err(e) => eprintln!("library import: {e}"),
        }
        w.sidebar_tab = SidebarTab::Library;
    }
    Task::none()
}

pub(crate) fn library_apply_to_slide(w: &mut MainWindow, asset_id: String) -> Task<Message> {
    let asset = w.lib_assets.iter().find(|a| a.id == asset_id).cloned();
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
            if let Err(e) = w.repo.update_slide(&c) {
                eprintln!("save: {e}");
            }
        }
        w.recently_used_asset_ids.retain(|id| id != &asset_id);
        w.recently_used_asset_ids.insert(0, asset_id);
        w.recently_used_asset_ids.truncate(6);
    }
    Task::none()
}

pub(crate) fn library_delete_asset(w: &mut MainWindow, asset_id: String) -> Task<Message> {
    match w.lib_repo.delete_asset(&asset_id) {
        Ok(_) => {
            w.lib_assets.retain(|a| a.id != asset_id);
            w.recently_used_asset_ids.retain(|id| id != &asset_id);
            if w.selected_asset_id.as_deref() == Some(&asset_id) {
                w.selected_asset_id = None;
            }
        }
        Err(e) => eprintln!("library delete: {e}"),
    }
    Task::none()
}

pub(crate) fn library_select_asset(w: &mut MainWindow, asset_id: String) -> Task<Message> {
    w.selected_asset_id = if w.selected_asset_id.as_deref() == Some(&asset_id) {
        None
    } else {
        Some(asset_id)
    };
    Task::none()
}

pub(crate) fn open_output_window(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.output_window_id {
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
    w.output_window_id = Some(id);
    w.output_is_fullscreen = false;

    if cfg.auto_fullscreen {
        w.output_is_fullscreen = true;
        Task::batch([
            open_task.map(|_| Message::OutputWindowOpened),
            window::set_mode(id, window::Mode::Fullscreen),
        ])
    } else {
        open_task.map(|_| Message::OutputWindowOpened)
    }
}

pub(crate) fn close_output_window(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.output_window_id.take() {
        w.output_black_screen = false;
        w.output_is_fullscreen = false;
        return window::close(id);
    }
    Task::none()
}

pub(crate) fn toggle_output_black_screen(w: &mut MainWindow) -> Task<Message> {
    w.output_black_screen = !w.output_black_screen;
    Task::none()
}

pub(crate) fn output_fullscreen_toggled(w: &mut MainWindow) -> Task<Message> {
    let Some(id) = w.output_window_id else {
        return Task::none();
    };
    if w.output_is_fullscreen {
        w.output_is_fullscreen = false;
        window::set_mode(id, window::Mode::Windowed)
    } else {
        w.output_is_fullscreen = true;
        window::set_mode(id, window::Mode::Fullscreen)
    }
}

pub(crate) fn toggle_output_settings(w: &mut MainWindow) -> Task<Message> {
    w.show_output_settings = !w.show_output_settings;
    Task::none()
}

pub(crate) fn output_screen_x_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    if let Ok(x) = v.parse::<f32>() {
        w.app_config.output.screen_x = x;
        let _ = w.app_config.save();
    }
    w.editing_output_screen_x = v;
    Task::none()
}

pub(crate) fn output_screen_y_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    if let Ok(y) = v.parse::<f32>() {
        w.app_config.output.screen_y = y;
        let _ = w.app_config.save();
    }
    w.editing_output_screen_y = v;
    Task::none()
}

pub(crate) fn output_auto_fullscreen_toggled(w: &mut MainWindow, enabled: bool) -> Task<Message> {
    w.app_config.output.auto_fullscreen = enabled;
    let _ = w.app_config.save();
    Task::none()
}

pub(crate) fn window_closed(w: &mut MainWindow, id: window::Id) -> Task<Message> {
    if Some(id) == w.output_window_id {
        w.output_window_id = None;
        w.output_black_screen = false;
        w.output_is_fullscreen = false;
        return window::close(id);
    }
    if Some(id) == w.delete_confirm_window_id {
        w.delete_confirm_window_id = None;
        w.show_delete_confirmation = false;
        w.delete_target_id = None;
        w.song.to_delete = None;
        w.service.to_delete = None;
        return Task::none();
    }
    if Some(id) == w.new_presentation_window_id {
        w.new_presentation_window_id = None;
        w.new_presentation_name.clear();
        return Task::none();
    }
    if Some(id) == w.shortcuts_window_id {
        w.shortcuts_window_id = None;
        return Task::none();
    }
    if id == w.main_window_id {
        return iced::exit();
    }
    Task::none()
}
