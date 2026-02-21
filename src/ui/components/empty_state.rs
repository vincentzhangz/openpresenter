use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Element, Length,
    widget::{container, text},
};

/// Centred muted placeholder shown when a list or panel has no content.
pub fn empty_state<'a>(message: &'a str) -> Element<'a, Message> {
    container(text(message).size(13).color(theme::TEXT_MUTED))
        .padding([16, 14])
        .center_x(Length::Fill)
        .into()
}
