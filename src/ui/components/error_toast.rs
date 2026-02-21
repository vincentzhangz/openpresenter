use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Alignment, Element, Length, widget::{button, container, row, text}};

/// Dismissible full-width error banner rendered above all other UI layers.
///
/// Emits `on_dismiss` when the user clicks the close button.
pub fn error_toast<'a>(message: &'a str, on_dismiss: Message) -> Element<'a, Message> {
    container(
        row![
            text(message)
                .size(13)
                .color(theme::TEXT_PRIMARY)
                .width(Length::Fill),
            button(text("✕").size(13))
                .on_press(on_dismiss)
                .padding([4, 8])
                .style(theme::ghost_button),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([10, 14]),
    )
    .width(Length::Fill)
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(0.7, 0.15, 0.1))),
        ..Default::default()
    })
    .into()
}
