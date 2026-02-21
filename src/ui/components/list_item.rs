use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Element, Length, widget::button};

/// Full-width selectable list row.
///
/// Renders with `primary_button` style when `selected`, `ghost_button` otherwise.
pub fn list_item<'a>(
    content: impl Into<Element<'a, Message>>,
    on_press: Message,
    selected: bool,
) -> Element<'a, Message> {
    button(content)
        .on_press(on_press)
        .width(Length::Fill)
        .padding([8, 12])
        .style(if selected {
            theme::primary_button
        } else {
            theme::ghost_button
        })
        .into()
}
