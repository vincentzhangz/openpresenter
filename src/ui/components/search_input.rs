use crate::ui::messages::Message;
use iced::{
    Element,
    widget::{container, text_input},
};

/// Padded search text-input with consistent inner and outer spacing.
///
/// The outer container adds `top: 6 / right: 10 / bottom: 2 / left: 10` padding
/// so the field aligns with adjacent panel content.
pub fn search_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    container(
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([7, 10])
            .size(13),
    )
    .padding(iced::Padding {
        top: 6.0,
        right: 10.0,
        bottom: 2.0,
        left: 10.0,
    })
    .into()
}
