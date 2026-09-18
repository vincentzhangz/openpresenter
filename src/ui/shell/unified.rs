use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Element, Length, widget::column, widget::container, widget::row};

/// Unified multi-dock shell: top toolbar + (left rail | center | right
/// dock) + optional bottom media bin.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let navbar_state = crate::ui::navbar::NavbarState {
        ndi_active: w.presenting.ndi_output.is_some(),
        audience_active: w.output.window_id.is_some(),
        stage_active: w.presenting.stage_display_active,
        reduce_motion: w.ui.reduce_motion,
        recording_active: w.recording.manager.state == crate::recording::RecordingState::Recording,
        matrix_open: w.output.matrix_open,
        bible_active: w.shell.sidebar_tab == crate::ui::messages::SidebarTab::Bible,
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

    let content = container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::base_style);

    let base: Element<'a, Message> = if w.output.matrix_open {
        iced::widget::stack![content, crate::ui::output::matrix::view(w)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        content.into()
    };

    if w.output.settings_open {
        let backdrop = iced::widget::mouse_area(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_: &iced::Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        0.0, 0.0, 0.0, 0.75,
                    ))),
                    ..Default::default()
                }),
        )
        .on_press(Message::Output(crate::ui::output::Message::SettingsClose));

        let modal_panel = iced::widget::mouse_area(
            container(crate::ui::output::view(w))
                .width(780)
                .height(580)
                .style(theme::dark_panel_style),
        )
        .on_press(Message::Noop);

        let modal_centered = container(modal_panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        iced::widget::stack![base, backdrop, modal_centered]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        base
    }
}
