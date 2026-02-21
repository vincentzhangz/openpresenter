use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Element, Length,
    widget::{TextInput, column, text},
};

/// Stacked label + text-input column used in settings forms and property panels.
///
/// The `input` element is given `Fill` width and `[5, 8]` padding automatically.
pub fn field_col<'a>(label: &'a str, input: TextInput<'a, Message>) -> Element<'a, Message> {
    column![
        text(label).size(11).color(theme::TEXT_MUTED),
        input.padding([5u16, 8u16]).size(13).width(Length::Fill),
    ]
    .spacing(3)
    .width(Length::Fill)
    .into()
}
