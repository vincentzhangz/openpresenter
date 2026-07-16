use crate::domain::{SlideContent, SlideTheme};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::Task;

/// Messages owned by the Themes feature module (see `AGENTS.md`).
///
/// NOTE: the theme controls are rendered inline inside the editor inspector's
/// Theme tab; this module owns the message enum and dispatch logic only.
#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    SaveSlideAsTheme,
    Apply(String),
    Delete(String),
    Select(String),
    Export,
    Import,
}

/// Dispatch a themes message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::NameChanged(name) => theme_name_changed(w, name),
        Message::SaveSlideAsTheme => save_slide_as_theme(w),
        Message::Apply(id) => apply_theme(w, id),
        Message::Delete(id) => delete_theme(w, id),
        Message::Select(id) => select_theme(w, id),
        Message::Export => export_themes(w),
        Message::Import => import_themes(w),
    }
}

pub(crate) fn theme_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.theme_state.new_theme_name = name;
    Task::none()
}

pub(crate) fn save_slide_as_theme(w: &mut MainWindow) -> Task<RootMessage> {
    let name = w.theme_state.new_theme_name.trim().to_owned();
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
        match w.theme_state.repo.save_theme(&theme) {
            Ok(_) => {
                w.theme_state.list.push(theme);
                w.theme_state.new_theme_name.clear();
            }
            Err(e) => w.set_error(format!("save theme: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn apply_theme(w: &mut MainWindow, theme_id: String) -> Task<RootMessage> {
    let theme = w
        .theme_state
        .list
        .iter()
        .find(|t| t.id == theme_id)
        .cloned();
    if let Some(theme) = theme {
        if let Some(slide) = w.get_current_slide_mut() {
            slide.background = theme.background.clone();
            slide.transition = theme.default_transition;
            if let SlideContent::Text { ref mut style, .. } = slide.content {
                *style = theme.default_text_style.clone();
            }
            let c = slide.clone();
            w.persist_slide(c);
        }
        w.theme_state.selected_theme_id = Some(theme_id);
    }
    Task::none()
}

pub(crate) fn delete_theme(w: &mut MainWindow, theme_id: String) -> Task<RootMessage> {
    match w.theme_state.repo.delete_theme(&theme_id) {
        Ok(_) => {
            w.theme_state.list.retain(|t| t.id != theme_id);
            if w.theme_state.selected_theme_id.as_deref() == Some(&theme_id) {
                w.theme_state.selected_theme_id = None;
            }
        }
        Err(e) => w.set_error(format!("delete theme: {e}")),
    }
    Task::none()
}

pub(crate) fn select_theme(w: &mut MainWindow, theme_id: String) -> Task<RootMessage> {
    w.theme_state.selected_theme_id =
        if w.theme_state.selected_theme_id.as_deref() == Some(&theme_id) {
            None
        } else {
            Some(theme_id)
        };
    Task::none()
}

pub(crate) fn export_themes(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Export Themes")
        .set_file_name("themes.json")
        .save_file()
    {
        match serde_json::to_string_pretty(&w.theme_state.list) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, &json) {
                    w.set_error(format!("export themes: {e}"));
                }
            }
            Err(e) => w.set_error(format!("serialize themes: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn import_themes(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Import Themes")
        .pick_file()
    {
        match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Vec<SlideTheme>>(&json) {
                Ok(imported) => {
                    for theme in imported {
                        if w.theme_state.list.iter().any(|t| t.id == theme.id) {
                            continue;
                        }
                        if let Err(e) = w.theme_state.repo.save_theme(&theme) {
                            w.set_error(format!("import theme save: {e}"));
                        } else {
                            w.theme_state.list.push(theme);
                        }
                    }
                }
                Err(e) => w.set_error(format!("parse themes JSON: {e}")),
            },
            Err(e) => w.set_error(format!("read themes file: {e}")),
        }
    }
    Task::none()
}
