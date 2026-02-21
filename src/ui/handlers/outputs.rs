use crate::output::{NamedOutput, OutputContentRoute};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub fn open_settings(w: &mut MainWindow) -> Task<Message> {
    w.output_settings_open = true;
    Task::none()
}

pub fn close_settings(w: &mut MainWindow) -> Task<Message> {
    w.output_settings_open = false;
    Task::none()
}

pub fn add_window(w: &mut MainWindow) -> Task<Message> {
    let label = w.new_output_label.trim().to_string();
    if label.is_empty() {
        return Task::none();
    }
    let id = label.to_lowercase().replace(' ', "_");
    w.output_manager.add(NamedOutput::new_window(id, label));
    w.new_output_label.clear();
    Task::none()
}

pub fn add_ndi(w: &mut MainWindow) -> Task<Message> {
    let label = w.new_output_label.trim().to_string();
    if label.is_empty() {
        return Task::none();
    }
    let stream_name = if w.new_output_ndi_name.trim().is_empty() {
        label.clone()
    } else {
        w.new_output_ndi_name.trim().to_string()
    };
    let id = label.to_lowercase().replace(' ', "_") + "_ndi";
    w.output_manager
        .add(NamedOutput::new_ndi(id, label, stream_name));
    w.new_output_label.clear();
    w.new_output_ndi_name.clear();
    Task::none()
}

pub fn remove(w: &mut MainWindow, id: String) -> Task<Message> {
    if id == "main" {
        return Task::none();
    }
    w.output_manager.remove(&id);
    Task::none()
}

pub fn set_active(w: &mut MainWindow, id: String, active: bool) -> Task<Message> {
    w.output_manager.set_active(&id, active);
    Task::none()
}

pub fn cycle_content(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(output) = w.output_manager.get(&id) {
        let next = match &output.content {
            OutputContentRoute::LiveSlide => OutputContentRoute::Stage,
            OutputContentRoute::Stage => OutputContentRoute::Blank,
            OutputContentRoute::Blank => OutputContentRoute::LiveSlide,
            OutputContentRoute::Mirror { .. } => OutputContentRoute::LiveSlide,
        };
        w.output_manager.set_content(&id, next);
    }
    Task::none()
}

pub fn set_resolution(w: &mut MainWindow, id: String, width: u32, height: u32) -> Task<Message> {
    w.output_manager.set_resolution(&id, width, height);
    Task::none()
}

pub fn new_label_changed(w: &mut MainWindow, label: String) -> Task<Message> {
    w.new_output_label = label;
    Task::none()
}

pub fn new_ndi_name_changed(w: &mut MainWindow, name: String) -> Task<Message> {
    w.new_output_ndi_name = name;
    Task::none()
}
