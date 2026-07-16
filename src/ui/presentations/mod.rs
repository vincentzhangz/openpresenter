use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, ViewMode};
use iced::Task;

pub(crate) fn open(w: &mut MainWindow, id: String) -> Task<Message> {
    match w.services.presentations.get(&id) {
        Ok(pres) => {
            if w.shell.current_mode == ViewMode::Edit {
                w.editor.editing_name = pres.name.clone();
                w.editor.selected_slide_index = if pres.slides.is_empty() {
                    None
                } else {
                    Some(0)
                };
                w.editor.editing = Some(pres);
                w.load_slide_for_editing();
            } else {
                w.presenting.slide_index = 0;
                w.presenting.transition = None;
                w.presenting.presentation = Some(pres);
                let _ = crate::ui::presenter::activate_slide(w, 0, false);
            }
        }
        Err(e) => w.set_error(format!("Failed to load presentation: {e}")),
    }
    Task::none()
}

pub(crate) fn rename(w: &mut MainWindow) -> Task<Message> {
    let name = w.editor.editing_name.clone();
    if let Some(ref pres) = w.editor.editing
        && !name.trim().is_empty()
    {
        let id = pres.id.clone();
        match w.services.presentations.rename(&id, &name) {
            Ok(_) => {
                if let Some(p) = &mut w.editor.editing {
                    p.name = name;
                }
                w.load_presentations();
            }
            Err(e) => w.set_error(format!("Failed to rename: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn rename_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.editor.editing_name = name;
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<Message> {
    crate::ui::editor::delete_confirm::open_delete_dialog(
        w,
        crate::ui::editor::delete_confirm::DeleteTarget::Presentation,
        id,
    )
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.editor.delete_target_id.take() {
        match w.services.presentations.delete(&id) {
            Ok(_) => {
                w.editor.editing = None;
                w.load_presentations();
            }
            Err(e) => w.set_error(format!("Failed to delete: {e}")),
        }
    }
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<Message> {
    w.editor.delete_target_id = None;
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}
