use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Alignment, Element, Length, widget::{button, container, row, text}};

/// Full-width ghost button with a leading `+` accent and a top-border separator.
///
/// Use at the bottom of list panels to let users add a new item.
pub fn add_button<'a>(label: &'a str, on_press: Message) -> Element<'a, Message> {
    container(
        button(
            row![
                text("+").size(16).color(theme::ACCENT_BLUE),
                text(label).size(12).color(theme::TEXT_PRIMARY),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([0, 4]),
        )
        .on_press(on_press)
        .width(Length::Fill)
        .style(theme::ghost_button),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        border: iced::Border {
            color: theme::BORDER_PANEL,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}
