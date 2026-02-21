use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, ViewMode};
use iced::{Size, Task, window};

pub(crate) fn open(w: &mut MainWindow, id: String) -> Task<Message> {
    match w.repo.get_presentation(&id) {
        Ok(pres) => {
            if w.current_mode == ViewMode::Edit {
                w.editing_presentation_name = pres.name.clone();
                w.selected_slide_index = if pres.slides.is_empty() {
                    None
                } else {
                    Some(0)
                };
                w.editing_presentation = Some(pres);
                w.load_slide_for_editing();
            } else {
                w.presenting_slide_index = 0;
                if let Some(slide) = pres.slides.first()
                    && let Some(ref ndi) = w.ndi_output
                {
                    ndi.set_slide(slide.clone());
                }
                w.presenting_presentation = Some(pres);
            }
        }
        Err(e) => eprintln!("Failed to load presentation: {e}"),
    }
    Task::none()
}

pub(crate) fn rename(w: &mut MainWindow) -> Task<Message> {
    let name = w.editing_presentation_name.clone();
    if let Some(ref pres) = w.editing_presentation
        && !name.trim().is_empty()
    {
        let id = pres.id.clone();
        match w.repo.update_presentation(&id, &name) {
            Ok(_) => {
                if let Some(p) = &mut w.editing_presentation {
                    p.name = name;
                }
                w.load_presentations();
            }
            Err(e) => eprintln!("Failed to rename: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn rename_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.editing_presentation_name = name;
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(existing) = w.delete_confirm_window_id {
        return window::gain_focus(existing);
    }
    w.show_delete_confirmation = true;
    w.delete_target_id = Some(id);
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 260.0),
        resizable: false,
        ..Default::default()
    });
    w.delete_confirm_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.delete_target_id.take() {
        match w.repo.delete_presentation(&id) {
            Ok(_) => {
                w.editing_presentation = None;
                w.load_presentations();
            }
            Err(e) => eprintln!("Failed to delete: {e}"),
        }
    }
    w.show_delete_confirmation = false;
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<Message> {
    w.show_delete_confirmation = false;
    w.delete_target_id = None;
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}
