//! Slide theme library logic: save the current slide as a theme, apply,
//! select, delete, and JSON import/export.
//!
//! Ported from the iced-era `ui/themes/mod.rs`. The theme controls are
//! rendered inline inside the inspector's Theme tab (see `ui/inspector.rs`);
//! this module owns the message vocabulary and the update logic.

use crate::domain::SlideTheme;
use crate::ui::main_window::MainWindow;
use gpui_kit::Context;

/// Messages owned by the Themes feature module. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Themes(..)`.
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
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::NameChanged(name) => theme_name_changed(w, name, cx),
        Message::SaveSlideAsTheme => save_slide_as_theme(w, cx),
        Message::Apply(id) => apply_theme(w, id, cx),
        Message::Delete(id) => delete_theme(w, id, cx),
        Message::Select(id) => select_theme(w, id, cx),
        Message::Export => export_themes(w, cx),
        Message::Import => import_themes(w, cx),
    }
}

pub(crate) fn theme_name_changed(w: &mut MainWindow, name: String, cx: &mut Context<MainWindow>) {
    w.theme_state.new_theme_name = name;
    cx.notify();
}

pub(crate) fn save_slide_as_theme(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let name = w.theme_state.new_theme_name.trim().to_owned();
    if name.is_empty() {
        return;
    }
    if let Some(slide) = w.get_current_slide() {
        // A slide without a text object still has a look worth keeping: its
        // background and its transition. The text style is the default one,
        // so the theme is saved instead of the press doing nothing.
        let style = slide
            .editable_text(w.layer.selected_index)
            .map(|(_, style)| style.clone())
            .unwrap_or_default();
        let theme = SlideTheme::new(name, slide.background.clone(), style, slide.transition);
        match w.theme_state.repo.save_theme(&theme) {
            Ok(_) => {
                w.theme_state.list.push(theme);
                w.theme_state.new_theme_name.clear();
            }
            Err(e) => w.set_error(format!("save theme: {e}")),
        }
    }
    cx.notify();
}

pub(crate) fn apply_theme(w: &mut MainWindow, theme_id: String, cx: &mut Context<MainWindow>) {
    let theme = w
        .theme_state
        .list
        .iter()
        .find(|t| t.id == theme_id)
        .cloned();
    if let Some(theme) = theme {
        // Applying a theme changes the slide, so it is an edit like any
        // other and can be undone.
        w.push_undo();
        let sel_layer = w.layer.selected_index;
        if let Some(slide) = w.get_current_slide_mut() {
            slide.background = theme.background.clone();
            slide.transition = theme.default_transition;
            // The style the renderer draws is the object's, so the theme is
            // applied there.
            if let Some((_, style)) = slide.editable_text_mut(sel_layer) {
                // A theme carries the look, not the place: the object's own
                // position is kept, so applying a theme does not move the
                // text.
                let position_x = style.position_x;
                let position_y = style.position_y;
                *style = theme.default_text_style.clone();
                style.position_x = position_x;
                style.position_y = position_y;
            }
            let clone = slide.clone();
            w.persist_slide(clone, cx);
        }
        // The panels show the editor's mirrors, so they have to follow the
        // theme — otherwise the panel shows the old size and the old
        // duration while the slide is drawn with the new ones.
        if let Some(slide) = w.get_current_slide()
            && let Some((_, style)) = slide.editable_text(sel_layer)
        {
            w.editor.editing_slide_font_size = style.font_size.to_string();
        }
        w.editor.editing_transition_duration =
            crate::ui::navigation::transition_duration_ms(&theme.default_transition).to_string();
        crate::ui::layers::load_layer_for_editing(w);
        w.theme_state.selected_theme_id = Some(theme_id);
    }
    cx.notify();
}

pub(crate) fn delete_theme(w: &mut MainWindow, theme_id: String, cx: &mut Context<MainWindow>) {
    match w.theme_state.repo.delete_theme(&theme_id) {
        Ok(_) => {
            w.theme_state.list.retain(|t| t.id != theme_id);
            if w.theme_state.selected_theme_id.as_deref() == Some(&theme_id) {
                w.theme_state.selected_theme_id = None;
            }
            // A screen that named the deleted theme would quietly fall back
            // to the default look, so the name is cleared.
            for look in w.props.manager.looks.iter_mut() {
                for target in look.screens.iter_mut() {
                    if target.theme_id.as_deref() == Some(theme_id.as_str()) {
                        target.theme_id = None;
                    }
                }
            }
            w.save_props();
            // The screens that named it changed, so the broadcast has to be
            // told.
            crate::ui::props::push_overlay(w);
        }
        Err(e) => w.set_error(format!("delete theme: {e}")),
    }
    cx.notify();
}

pub(crate) fn select_theme(w: &mut MainWindow, theme_id: String, cx: &mut Context<MainWindow>) {
    w.theme_state.selected_theme_id =
        if w.theme_state.selected_theme_id.as_deref() == Some(&theme_id) {
            None
        } else {
            Some(theme_id)
        };
    cx.notify();
}

pub(crate) fn export_themes(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
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
    cx.notify();
}

pub(crate) fn import_themes(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
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
    cx.notify();
}
