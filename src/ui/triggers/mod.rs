use crate::triggers::Action;
use crate::triggers::automation::Macro;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::{
    Element, Length, Task,
    widget::{Space, button, column, row, text, text_input, toggler},
};
use iced_font_awesome::fa_icon_solid;

/// Messages owned by the Triggers feature module (see `AGENTS.md`).
///
/// `ToggleTriggersPanel` (global panel visibility) and `TriggerFired`
/// (external trigger events injected via subscription) stay as root variants.
#[derive(Debug, Clone)]
pub enum Message {
    HttpStart,
    HttpStop,
    HttpPortChanged(String),
    OscStart,
    OscStop,
    OscPortChanged(String),
    MacroAdd,
    MacroRemove(String),
    MacroRun(String),
    MacroStop(String),
    MacroToggleLoop(String),
    MacroNameChanged(String),
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Triggers(msg)
}

/// Render the triggers panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    triggers_panel(
        w.triggers.manager.http_running,
        &w.triggers.http_port_str,
        w.triggers.manager.osc_running,
        &w.triggers.osc_port_str,
        &w.triggers.manager.macros,
        &w.triggers.new_macro_name,
        &w.triggers.macro_running_ids,
    )
}

/// Dispatch a triggers message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::HttpStart => http_start(w),
        Message::HttpStop => http_stop(w),
        Message::HttpPortChanged(s) => http_port_changed(w, s),
        Message::OscStart => osc_start(w),
        Message::OscStop => osc_stop(w),
        Message::OscPortChanged(s) => osc_port_changed(w, s),
        Message::MacroAdd => macro_add(w),
        Message::MacroRemove(id) => macro_remove(w, id),
        Message::MacroRun(id) => macro_run(w, id),
        Message::MacroStop(id) => macro_stop(w, id),
        Message::MacroToggleLoop(id) => macro_toggle_loop(w, id),
        Message::MacroNameChanged(s) => macro_name_changed(w, s),
    }
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.triggers.panel_open = !w.triggers.panel_open;
    Task::none()
}

pub(crate) fn http_start(w: &mut MainWindow) -> Task<RootMessage> {
    if let Ok(port) = w.triggers.http_port_str.parse::<u16>() {
        w.triggers.manager.http_port = port;
    }
    w.triggers.manager.start_http();
    Task::none()
}

pub(crate) fn http_stop(w: &mut MainWindow) -> Task<RootMessage> {
    w.triggers.manager.stop_http();
    Task::none()
}

pub(crate) fn http_port_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.triggers.http_port_str = s;
    Task::none()
}

pub(crate) fn osc_start(w: &mut MainWindow) -> Task<RootMessage> {
    if let Ok(port) = w.triggers.osc_port_str.parse::<u16>() {
        w.triggers.manager.osc_port = port;
    }
    w.triggers.manager.start_osc();
    Task::none()
}

pub(crate) fn osc_stop(w: &mut MainWindow) -> Task<RootMessage> {
    w.triggers.manager.stop_osc();
    Task::none()
}

pub(crate) fn osc_port_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.triggers.osc_port_str = s;
    Task::none()
}

pub(crate) fn apply_action(w: &mut MainWindow, action: Action) -> Task<RootMessage> {
    match action {
        Action::NextSlide => crate::ui::presenter::next_slide(w),
        Action::PrevSlide => crate::ui::presenter::prev_slide(w),
        Action::GotoSlide(idx) => crate::ui::presenter::activate_slide(w, idx, true),
        Action::BlackScreen(on) => {
            w.output.black_screen = on;
            Task::none()
        }
        Action::ClearOutput => {
            w.output.black_screen = true;
            Task::none()
        }
        Action::TriggerProp(id) => {
            w.props.manager.toggle_prop(&id);
            Task::none()
        }
        Action::StartTimer => {
            if !w.presenting.timer_running {
                w.presenting.timer_running = true;
                w.presenting.timer_start_epoch = w.presenting.clock_secs;
            }
            Task::none()
        }
        Action::StopTimer => {
            w.presenting.timer_running = false;
            Task::none()
        }
        Action::ResetTimer => {
            w.presenting.timer_secs = 0;
            w.presenting.timer_running = false;
            Task::none()
        }
    }
}

pub(crate) fn trigger_fired(w: &mut MainWindow, action: Action) -> Task<RootMessage> {
    apply_action(w, action)
}

pub(crate) fn macro_add(w: &mut MainWindow) -> Task<RootMessage> {
    if w.triggers.new_macro_name.trim().is_empty() {
        return Task::none();
    }
    let name = w.triggers.new_macro_name.trim().to_string();
    let m = Macro::new(name);
    w.triggers.manager.macros.push(m);
    w.triggers.new_macro_name.clear();
    Task::none()
}

pub(crate) fn macro_remove(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    if let Some(h) = w.triggers.macro_running_handles.remove(&id) {
        h.abort();
        w.triggers.macro_running_ids.remove(&id);
    }
    w.triggers.manager.macros.retain(|m| m.id != id);
    Task::none()
}

pub(crate) fn macro_run(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    let tx = w.triggers.manager.sender();
    if let Some(m) = w.triggers.manager.macros.iter().find(|m| m.id == id) {
        let handle = m.spawn(tx);
        w.triggers.macro_running_handles.insert(id.clone(), handle);
        w.triggers.macro_running_ids.insert(id);
    }
    Task::none()
}

pub(crate) fn macro_stop(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    if let Some(h) = w.triggers.macro_running_handles.remove(&id) {
        h.abort();
        w.triggers.macro_running_ids.remove(&id);
    }
    Task::none()
}

pub(crate) fn macro_toggle_loop(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id) {
        m.looping = !m.looping;
    }
    Task::none()
}

pub(crate) fn macro_name_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.triggers.new_macro_name = s;
    Task::none()
}

pub fn triggers_panel<'a>(
    http_running: bool,
    http_port: &'a str,
    osc_running: bool,
    osc_port: &'a str,
    macros: &'a [Macro],
    new_macro_name: &'a str,
    running_ids: &'a std::collections::HashSet<String>,
) -> Element<'a, RootMessage> {
    let header = row![
        text("Triggers & Automation").size(16.0),
        Space::new().width(Length::Fill),
        button(fa_icon_solid("xmark").size(13.0_f32)).on_press(RootMessage::ToggleTriggersPanel),
    ]
    .spacing(8);

    let http_label = if http_running {
        format!("HTTP server running on :{http_port}")
    } else {
        format!("HTTP server (port {http_port})")
    };
    let http_start_stop = if http_running {
        button("Stop").on_press(wrap(Message::HttpStop))
    } else {
        button("Start").on_press(wrap(Message::HttpStart))
    };
    let http_section = row![
        text("HTTP REST").size(13.0),
        Space::new().width(Length::Fill),
        text_input("Port", http_port)
            .on_input(move |v| wrap(Message::HttpPortChanged(v)))
            .width(70),
        http_start_stop,
        text(http_label).size(11.0),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let osc_label = if osc_running {
        format!("OSC listening on :{osc_port}")
    } else {
        format!("OSC listener (port {osc_port})")
    };
    let osc_start_stop = if osc_running {
        button("Stop").on_press(wrap(Message::OscStop))
    } else {
        button("Start").on_press(wrap(Message::OscStart))
    };
    let osc_section = row![
        text("OSC").size(13.0),
        Space::new().width(Length::Fill),
        text_input("Port", osc_port)
            .on_input(move |v| wrap(Message::OscPortChanged(v)))
            .width(70),
        osc_start_stop,
        text(osc_label).size(11.0),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let macro_title = text("Automation macros").size(13.0);

    let macro_list: Element<'a, RootMessage> = if macros.is_empty() {
        text("No macros defined.").size(11.0).into()
    } else {
        let rows: Vec<Element<'a, RootMessage>> = macros
            .iter()
            .map(|m| {
                let is_running = running_ids.contains(&m.id);
                let run_btn = if is_running {
                    button("Stop").on_press(wrap(Message::MacroStop(m.id.clone())))
                } else {
                    button("Run").on_press(wrap(Message::MacroRun(m.id.clone())))
                };
                let loop_toggle = toggler(m.looping)
                    .on_toggle(move |_v| wrap(Message::MacroToggleLoop(m.id.clone())));
                row![
                    text(&m.name).size(12.0),
                    Space::new().width(Length::Fill),
                    loop_toggle,
                    text("loop").size(11.0),
                    run_btn,
                    button(fa_icon_solid("xmark").size(13.0_f32))
                        .on_press(wrap(Message::MacroRemove(m.id.clone()))),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .into()
            })
            .collect();
        column(rows).spacing(4).into()
    };

    let new_macro_row = row![
        text_input("New macro name…", new_macro_name)
            .on_input(move |v| wrap(Message::MacroNameChanged(v)))
            .width(Length::Fill),
        button("+ Macro").on_press(wrap(Message::MacroAdd)),
    ]
    .spacing(6);

    column![
        header,
        http_section,
        osc_section,
        macro_title,
        macro_list,
        new_macro_row,
    ]
    .spacing(10)
    .padding(12)
    .width(Length::Fill)
    .into()
}
