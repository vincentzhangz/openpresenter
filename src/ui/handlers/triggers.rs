use crate::triggers::{TriggerAction, automation::Macro};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub fn toggle_panel(w: &mut MainWindow) -> Task<Message> {
    w.triggers_panel_open = !w.triggers_panel_open;
    Task::none()
}

pub fn http_start(w: &mut MainWindow) -> Task<Message> {
    if let Ok(port) = w.trigger_http_port_str.parse::<u16>() {
        w.trigger_manager.http_port = port;
    }
    w.trigger_manager.start_http();
    Task::none()
}

pub fn http_stop(w: &mut MainWindow) -> Task<Message> {
    w.trigger_manager.stop_http();
    Task::none()
}

pub fn http_port_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.trigger_http_port_str = s;
    Task::none()
}

pub fn osc_start(w: &mut MainWindow) -> Task<Message> {
    if let Ok(port) = w.trigger_osc_port_str.parse::<u16>() {
        w.trigger_manager.osc_port = port;
    }
    w.trigger_manager.start_osc();
    Task::none()
}

pub fn osc_stop(w: &mut MainWindow) -> Task<Message> {
    w.trigger_manager.stop_osc();
    Task::none()
}

pub fn osc_port_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.trigger_osc_port_str = s;
    Task::none()
}

pub fn trigger_fired(w: &mut MainWindow, action: TriggerAction) -> Task<Message> {
    match action {
        TriggerAction::NextSlide => crate::ui::handlers::presenter::next_slide(w),
        TriggerAction::PrevSlide => crate::ui::handlers::presenter::prev_slide(w),
        TriggerAction::GotoSlide(idx) => {
            w.presenting_slide_index = idx;
            Task::none()
        }
        TriggerAction::BlackScreen(on) => {
            w.output_black_screen = on;
            Task::none()
        }
        TriggerAction::ClearOutput => {
            w.output_black_screen = true;
            Task::none()
        }
        TriggerAction::TriggerProp(id) => {
            w.prop_manager.toggle_prop(&id);
            Task::none()
        }
        TriggerAction::StartTimer => {
            if !w.timer_running {
                w.timer_running = true;
                w.timer_start_epoch = w.clock_secs;
            }
            Task::none()
        }
        TriggerAction::StopTimer => {
            w.timer_running = false;
            Task::none()
        }
        TriggerAction::ResetTimer => {
            w.timer_secs = 0;
            w.timer_running = false;
            Task::none()
        }
    }
}

pub fn macro_add(w: &mut MainWindow) -> Task<Message> {
    if w.new_macro_name.trim().is_empty() {
        return Task::none();
    }
    let name = w.new_macro_name.trim().to_string();
    let m = Macro::new(name);
    w.trigger_manager.macros.push(m);
    w.new_macro_name.clear();
    Task::none()
}

pub fn macro_remove(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(h) = w.macro_running_handles.remove(&id) {
        h.abort();
        w.macro_running_ids.remove(&id);
    }
    w.trigger_manager.macros.retain(|m| m.id != id);
    Task::none()
}

pub fn macro_run(w: &mut MainWindow, id: String) -> Task<Message> {
    let tx = w.trigger_manager.sender();
    if let Some(m) = w.trigger_manager.macros.iter().find(|m| m.id == id) {
        let handle = m.spawn(tx);
        w.macro_running_handles.insert(id.clone(), handle);
        w.macro_running_ids.insert(id);
    }
    Task::none()
}

pub fn macro_stop(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(h) = w.macro_running_handles.remove(&id) {
        h.abort();
        w.macro_running_ids.remove(&id);
    }
    Task::none()
}

pub fn macro_toggle_loop(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(m) = w.trigger_manager.macros.iter_mut().find(|m| m.id == id) {
        m.looping = !m.looping;
    }
    Task::none()
}

pub fn macro_name_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.new_macro_name = s;
    Task::none()
}
