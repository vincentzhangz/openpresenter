use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element,
    widget::{container, row, text},
};

/// Small green status badge indicating an active or live state (e.g. "NDI", "LIVE").
pub fn live_badge<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        row![text(label).size(11).color(theme::LIVE_GREEN)]
            .spacing(4)
            .align_y(Alignment::Center)
            .padding([3, 8]),
    )
    .style(|_t: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba(
            0.204, 0.780, 0.349, 0.12,
        ))),
        border: iced::Border {
            color: iced::Color::from_rgba(0.204, 0.780, 0.349, 0.40),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
