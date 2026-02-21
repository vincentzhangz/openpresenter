use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Background, Element, Length,
    widget::{Space, container},
};

/// A 1 px horizontal rule styled with [`theme::BORDER_PANEL`].
///
/// Drop between sections whenever a visual separation is needed without a full header.
pub fn divider<'a>() -> Element<'a, Message> {
    container(Space::new().height(1))
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BORDER_PANEL)),
            ..Default::default()
        })
        .into()
}
