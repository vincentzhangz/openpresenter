use crate::slides::{SlideContent, SlideTheme};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub(crate) fn theme_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.new_theme_name = name;
    Task::none()
}

pub(crate) fn save_slide_as_theme(w: &mut MainWindow) -> Task<Message> {
    let name = w.new_theme_name.trim().to_owned();
    if name.is_empty() {
        return Task::none();
    }
    if let Some(slide) = w.get_current_slide()
        && let SlideContent::Text { ref style, .. } = slide.content
    {
        let theme = SlideTheme::new(
            name,
            slide.background.clone(),
            style.clone(),
            slide.transition,
        );
        match w.theme_repo.save_theme(&theme) {
            Ok(_) => {
                w.themes.push(theme);
                w.new_theme_name.clear();
            }
            Err(e) => eprintln!("save theme: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn apply_theme(w: &mut MainWindow, theme_id: String) -> Task<Message> {
    let theme = w.themes.iter().find(|t| t.id == theme_id).cloned();
    if let Some(theme) = theme {
        if let Some(slide) = w.get_current_slide_mut() {
            slide.background = theme.background.clone();
            slide.transition = theme.default_transition;
            if let SlideContent::Text { ref mut style, .. } = slide.content {
                *style = theme.default_text_style.clone();
            }
            let c = slide.clone();
            if let Err(e) = w.repo.update_slide(&c) {
                eprintln!("apply theme save: {e}");
            }
        }
        w.selected_theme_id = Some(theme_id);
    }
    Task::none()
}

pub(crate) fn delete_theme(w: &mut MainWindow, theme_id: String) -> Task<Message> {
    match w.theme_repo.delete_theme(&theme_id) {
        Ok(_) => {
            w.themes.retain(|t| t.id != theme_id);
            if w.selected_theme_id.as_deref() == Some(&theme_id) {
                w.selected_theme_id = None;
            }
        }
        Err(e) => eprintln!("delete theme: {e}"),
    }
    Task::none()
}

pub(crate) fn select_theme(w: &mut MainWindow, theme_id: String) -> Task<Message> {
    w.selected_theme_id = if w.selected_theme_id.as_deref() == Some(&theme_id) {
        None
    } else {
        Some(theme_id)
    };
    Task::none()
}

pub(crate) fn export_themes(w: &mut MainWindow) -> Task<Message> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Export Themes")
        .set_file_name("themes.json")
        .save_file()
    {
        match serde_json::to_string_pretty(&w.themes) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, &json) {
                    eprintln!("export themes: {e}");
                }
            }
            Err(e) => eprintln!("serialize themes: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn import_themes(w: &mut MainWindow) -> Task<Message> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Import Themes")
        .pick_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Vec<SlideTheme>>(&json) {
                Ok(imported) => {
                    for theme in imported {
                        if w.themes.iter().any(|t| t.id == theme.id) {
                            continue;
                        }
                        if let Err(e) = w.theme_repo.save_theme(&theme) {
                            eprintln!("import theme save: {e}");
                        } else {
                            w.themes.push(theme);
                        }
                    }
                }
                Err(e) => eprintln!("parse themes JSON: {e}"),
            },
            Err(e) => eprintln!("read themes file: {e}"),
        }
    }
    Task::none()
}
