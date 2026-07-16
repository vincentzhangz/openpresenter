use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Size, Task, window};

pub(crate) fn toggle_shortcuts_overlay(w: &mut MainWindow) -> Task<Message> {
    if let Some(win_id) = w.ui.shortcuts_window_id.take() {
        return window::close(win_id);
    }
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 380.0),
        resizable: false,
        ..Default::default()
    });
    w.ui.shortcuts_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn toggle_reduce_motion(w: &mut MainWindow) -> Task<Message> {
    w.ui.reduce_motion = !w.ui.reduce_motion;
    if w.ui.reduce_motion {
        w.presenting.transition = None;
    }
    Task::none()
}

/// Bring focus to the global search field by switching to the Presentations
/// rail (where the search input lives).
pub(crate) fn focus_search(w: &mut MainWindow) -> Task<Message> {
    w.shell.sidebar_tab = crate::ui::messages::SidebarTab::Presentations;
    Task::none()
}

pub(crate) fn undo(w: &mut MainWindow) -> Task<Message> {
    if let Some(snapshot) = w.editor.undo_stack.pop() {
        if let Some(ref current) = w.editor.editing {
            w.editor.redo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let slides = snapshot.slides.clone();
        w.editor.editing = Some(snapshot);
        w.load_slide_for_editing();
        if let Err(e) = w.services.presentations.replace_slides(&pres_id, &slides) {
            w.set_error(format!("undo: {e}"));
        }
    }
    Task::none()
}

pub(crate) fn redo(w: &mut MainWindow) -> Task<Message> {
    if let Some(snapshot) = w.editor.redo_stack.pop() {
        if let Some(ref current) = w.editor.editing {
            w.editor.undo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let slides = snapshot.slides.clone();
        w.editor.editing = Some(snapshot);
        w.load_slide_for_editing();
        if let Err(e) = w.services.presentations.replace_slides(&pres_id, &slides) {
            w.set_error(format!("redo: {e}"));
        }
    }
    Task::none()
}
