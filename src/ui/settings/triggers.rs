use crate::triggers::automation::Macro;
use crate::ui::messages::Message;
use iced_font_awesome::fa_icon_solid;
use iced::{
    Element, Length,
    widget::{Space, button, column, row, text, text_input, toggler},
};

pub fn triggers_panel<'a>(
    http_running: bool,
    http_port: &'a str,
    osc_running: bool,
    osc_port: &'a str,
    macros: &'a [Macro],
    new_macro_name: &'a str,
    running_ids: &'a std::collections::HashSet<String>,
) -> Element<'a, Message> {
    let header = row![
        text("Triggers & Automation").size(16.0),
        Space::new().width(Length::Fill),
        button(fa_icon_solid("xmark").size(13.0)).on_press(Message::ToggleTriggersPanel),
    ]
    .spacing(8);

    let http_label = if http_running {
        format!("HTTP server running on :{http_port}")
    } else {
        format!("HTTP server (port {http_port})")
    };
    let http_start_stop = if http_running {
        button("Stop").on_press(Message::TriggerHttpStop)
    } else {
        button("Start").on_press(Message::TriggerHttpStart)
    };
    let http_section = row![
        text("HTTP REST").size(13.0),
        Space::new().width(Length::Fill),
        text_input("Port", http_port)
            .on_input(Message::TriggerHttpPortChanged)
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
        button("Stop").on_press(Message::TriggerOscStop)
    } else {
        button("Start").on_press(Message::TriggerOscStart)
    };
    let osc_section = row![
        text("OSC").size(13.0),
        Space::new().width(Length::Fill),
        text_input("Port", osc_port)
            .on_input(Message::TriggerOscPortChanged)
            .width(70),
        osc_start_stop,
        text(osc_label).size(11.0),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let macro_title = text("Automation macros").size(13.0);

    let macro_list: Element<'a, Message> = if macros.is_empty() {
        text("No macros defined.").size(11.0).into()
    } else {
        let rows: Vec<Element<'a, Message>> = macros
            .iter()
            .map(|m| {
                let is_running = running_ids.contains(&m.id);
                let run_btn = if is_running {
                    button("Stop").on_press(Message::MacroStop(m.id.clone()))
                } else {
                    button("Run").on_press(Message::MacroRun(m.id.clone()))
                };
                let loop_toggle =
                    toggler(m.looping).on_toggle(move |_v| Message::MacroToggleLoop(m.id.clone()));
                row![
                    text(&m.name).size(12.0),
                    Space::new().width(Length::Fill),
                    loop_toggle,
                    text("loop").size(11.0),
                    run_btn,
                    button(fa_icon_solid("xmark").size(13.0)).on_press(Message::MacroRemove(m.id.clone())),
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
            .on_input(Message::MacroNameChanged)
            .width(Length::Fill),
        button("+ Macro").on_press(Message::MacroAdd),
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
