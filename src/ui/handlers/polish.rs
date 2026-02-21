use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Size, Task, window};

pub(crate) fn toggle_shortcuts_overlay(w: &mut MainWindow) -> Task<Message> {
    if let Some(win_id) = w.shortcuts_window_id.take() {
        return window::close(win_id);
    }
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 380.0),
        resizable: false,
        ..Default::default()
    });
    w.shortcuts_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn toggle_reduce_motion(w: &mut MainWindow) -> Task<Message> {
    w.reduce_motion = !w.reduce_motion;
    if w.reduce_motion {
        w.presenting_transition = None;
    }
    Task::none()
}

pub(crate) fn undo(w: &mut MainWindow) -> Task<Message> {
    if let Some(snapshot) = w.undo_stack.pop() {
        if let Some(ref current) = w.editing_presentation {
            w.redo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let slides = snapshot.slides.clone();
        w.editing_presentation = Some(snapshot);
        w.load_slide_for_editing();
        if let Err(e) = w.repo.replace_presentation_slides(&pres_id, &slides) {
            eprintln!("undo: {e}");
        }
    }
    Task::none()
}

pub(crate) fn redo(w: &mut MainWindow) -> Task<Message> {
    if let Some(snapshot) = w.redo_stack.pop() {
        if let Some(ref current) = w.editing_presentation {
            w.undo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let slides = snapshot.slides.clone();
        w.editing_presentation = Some(snapshot);
        w.load_slide_for_editing();
        if let Err(e) = w.repo.replace_presentation_slides(&pres_id, &slides) {
            eprintln!("redo: {e}");
        }
    }
    Task::none()
}
