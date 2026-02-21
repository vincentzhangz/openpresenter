use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub fn start(w: &mut MainWindow) -> Task<Message> {
    if let Err(e) = w.recording_manager.start() {
        eprintln!("[recording] Failed to start: {e}");
    }
    Task::none()
}

pub fn stop(w: &mut MainWindow) -> Task<Message> {
    w.recording_manager.stop();
    Task::none()
}

pub fn path_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.recording_manager.output_path = s;
    Task::none()
}

pub fn fps_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    if let Ok(fps) = s.parse::<u32>() {
        w.recording_manager.fps = fps.clamp(1, 120);
    }
    Task::none()
}

pub fn toggle_panel(w: &mut MainWindow) -> Task<Message> {
    w.recording_panel_open = !w.recording_panel_open;
    Task::none()
}
