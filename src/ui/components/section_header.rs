use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Alignment, Element, Length, widget::{Space, container, row, text}};

/// Compact 10 px muted inline label. Use between property rows in inspector panels.
pub fn section_label<'a>(label: &'a str) -> Element<'a, Message> {
    text(label).size(10).color(theme::TEXT_MUTED).into()
}

/// Full-width dark-background header bar.
///
/// Use above a group of list items or between major sections in sidebar panels.
pub fn section_header<'a>(label: &'a str) -> Element<'a, Message> {
    container(
        row![
            text(label).size(10).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
        ]
        .padding([10, 14])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::section_header_style)
    .into()
}
