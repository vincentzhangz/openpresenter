use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, Message, ViewMode};
use iced::{Size, Task, window};

#[allow(clippy::collapsible_if)]
pub(crate) fn switch_mode(w: &mut MainWindow, mode: ViewMode) -> Task<Message> {
    let was_edit = w.shell.current_mode == ViewMode::Edit;
    w.shell.current_mode = mode;
    if mode == ViewMode::Show {
        // Promote the currently-edited presentation to the live one if none.
        if w.presenting.presentation.is_none() {
            if let Some(ref pres) = w.editor.editing {
                w.presenting.presentation = Some(pres.clone());
            }
        }
        w.editor.editing = None;
        w.editor.selected_slide_index = None;
        let idx = w.presenting.slide_index;
        crate::ui::video::on_presenter_slide_changed(w, idx);
    } else if !was_edit {
        // Entering Edit from Show: load the live presentation for editing.
        if let Some(ref pres) = w.presenting.presentation {
            w.editor.editing = Some(pres.clone());
            w.editor.selected_slide_index = Some(w.presenting.slide_index);
            w.load_slide_for_editing();
        }
    }
    Task::none()
}

pub(crate) fn switch_inspector_tab(w: &mut MainWindow, tab: InspectorTab) -> Task<Message> {
    w.shell.inspector_tab = tab;
    if tab == InspectorTab::Theme && w.theme_state.list.is_empty() {
        w.load_themes();
    }
    Task::none()
}

pub(crate) fn back_to_list(w: &mut MainWindow) -> Task<Message> {
    w.editor.editing = None;
    w.presenting.presentation = None;
    w.editor.selected_slide_index = None;
    w.presenting.slide_index = 0;
    w.presenting.transition = None;
    w.editor.editing_slide_text.clear();
    w.editor.editing_slide_font_size = String::from("72");
    crate::ui::video::stop_video(w);
    Task::none()
}

pub(crate) fn quit(w: &mut MainWindow) -> Task<Message> {
    if let Some(ndi) = w.presenting.ndi_output.take() {
        ndi.stop();
    }
    iced::exit()
}

pub(crate) fn search_query_changed(w: &mut MainWindow, q: String) -> Task<Message> {
    w.shell.search_query = q;
    Task::none()
}

pub(crate) fn new_presentation_clicked(w: &mut MainWindow) -> Task<Message> {
    if let Some(existing) = w.editor.new_presentation_window_id {
        return window::gain_focus(existing);
    }
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(400.0, 260.0),
        resizable: false,
        ..Default::default()
    });
    w.editor.new_presentation_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn create_presentation(w: &mut MainWindow) -> Task<Message> {
    if !w.editor.new_presentation_name.trim().is_empty() {
        let name = w.editor.new_presentation_name.clone();
        match w.services.presentations.create(&name) {
            Ok(_) => w.load_presentations(),
            Err(e) => w.set_error(format!("Failed to create presentation: {e}")),
        }
        w.editor.new_presentation_name.clear();
    }
    if let Some(win_id) = w.editor.new_presentation_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn cancel_new_presentation(w: &mut MainWindow) -> Task<Message> {
    w.editor.new_presentation_name.clear();
    if let Some(win_id) = w.editor.new_presentation_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn new_presentation_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.editor.new_presentation_name = name;
    Task::none()
}
