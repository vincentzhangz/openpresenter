use crate::ui::messages::Message;
use iced::{
    Element,
    widget::{Id, container, text_input},
};

/// Stable [`Id`] of the global search field rendered at the top of the left
/// rail. Used by `Message::FocusSearch` to move keyboard focus into it.
pub fn global_search_id() -> Id {
    Id::new("global-search")
}

/// Padded search text-input with consistent inner and outer spacing.
///
/// The outer container adds `top: 6 / right: 10 / bottom: 2 / left: 10` padding
/// so the field aligns with adjacent panel content.
pub fn search_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    build_search_input(placeholder, value, on_input, None)
}

/// Same as [`search_input`], but with an explicit [`Id`] so the field can be
/// focused programmatically via a widget operation.
pub fn search_input_with_id<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    id: Id,
) -> Element<'a, Message> {
    build_search_input(placeholder, value, on_input, Some(id))
}

fn build_search_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    id: Option<Id>,
) -> Element<'a, Message> {
    let mut input = text_input(placeholder, value).on_input(on_input);
    if let Some(id) = id {
        input = input.id(id);
    }
    container(input.padding([7, 10]).size(13))
        .padding(iced::Padding {
            top: 6.0,
            right: 10.0,
            bottom: 2.0,
            left: 10.0,
        })
        .into()
}
