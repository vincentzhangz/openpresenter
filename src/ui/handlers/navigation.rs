use crate::ui::handlers::video;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, Message, ViewMode};
use iced::{Size, Task, window};

pub(crate) fn switch_mode(w: &mut MainWindow, mode: ViewMode) -> Task<Message> {
    w.current_mode = mode;
    if mode == ViewMode::Show {
        w.editing_presentation = None;
        w.selected_slide_index = None;
        let idx = w.presenting_slide_index;
        video::on_presenter_slide_changed(w, idx);
    } else {
        w.presenting_presentation = None;
        w.presenting_slide_index = 0;
        video::stop_video(w);
    }
    Task::none()
}

pub(crate) fn switch_inspector_tab(w: &mut MainWindow, tab: InspectorTab) -> Task<Message> {
    w.inspector_tab = tab;
    if tab == InspectorTab::Theme && w.themes.is_empty() {
        w.load_themes();
    }
    Task::none()
}

pub(crate) fn back_to_list(w: &mut MainWindow) -> Task<Message> {
    w.editing_presentation = None;
    w.presenting_presentation = None;
    w.selected_slide_index = None;
    w.presenting_slide_index = 0;
    w.presenting_transition = None;
    w.editing_slide_text.clear();
    w.editing_slide_font_size = String::from("72");
    video::stop_video(w);
    Task::none()
}

pub(crate) fn quit(w: &mut MainWindow) -> Task<Message> {
    if let Some(ndi) = w.ndi_output.take() {
        ndi.stop();
    }
    iced::exit()
}

pub(crate) fn search_query_changed(w: &mut MainWindow, q: String) -> Task<Message> {
    w.search_query = q;
    Task::none()
}

pub(crate) fn new_presentation_clicked(w: &mut MainWindow) -> Task<Message> {
    if let Some(existing) = w.new_presentation_window_id {
        return window::gain_focus(existing);
    }
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(400.0, 260.0),
        resizable: false,
        ..Default::default()
    });
    w.new_presentation_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn create_presentation(w: &mut MainWindow) -> Task<Message> {
    if !w.new_presentation_name.trim().is_empty() {
        let name = w.new_presentation_name.clone();
        match w.repo.create_presentation(&name) {
            Ok(_) => w.load_presentations(),
            Err(e) => eprintln!("Failed to create presentation: {e}"),
        }
        w.new_presentation_name.clear();
    }
    if let Some(win_id) = w.new_presentation_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn cancel_new_presentation(w: &mut MainWindow) -> Task<Message> {
    w.new_presentation_name.clear();
    if let Some(win_id) = w.new_presentation_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn new_presentation_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.new_presentation_name = name;
    Task::none()
}
