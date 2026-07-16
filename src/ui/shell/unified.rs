use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Element, Length, widget::column, widget::container, widget::row};

/// ProPresenter-style unified shell: top toolbar + (left rail | center | right
/// dock) + optional bottom media bin.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let navbar_state = crate::ui::navbar::NavbarState {
        ndi_active: w.presenting.ndi_output.is_some(),
        stage_active: w.presenting.stage_display_active,
        reduce_motion: w.ui.reduce_motion,
        recording_active: w.recording.manager.state == crate::recording::RecordingState::Recording,
    };

    let nav = crate::ui::navbar::navbar(&w.shell, navbar_state);

    let left = crate::ui::shell::left_rail::view(w);
    let center = crate::ui::shell::center::view(w);
    let right = crate::ui::shell::right_dock::view(w);

    let middle = row![left, center, right]
        .width(Length::Fill)
        .height(Length::Fill);

    let mut body = column![nav, middle];

    if w.shell.media_bin_open {
        body = body.push(crate::ui::shell::media_bin::view(w));
    }

    if let Some(ref err) = w.ui.error_message {
        body = body.push(crate::ui::components::error_toast(
            err,
            Message::DismissError,
        ));
    }

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::base_style)
        .into()
}
