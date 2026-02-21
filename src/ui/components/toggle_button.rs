use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Element, widget::{button, text}};

/// Binary toggle button (e.g. Bold / Italic).
///
/// Uses `primary_button` style when active and `secondary_button` when inactive.
pub fn toggle_btn<'a>(label: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(text(label).size(12).color(if active {
        iced::Color::WHITE
    } else {
        theme::TEXT_SECONDARY
    }))
    .on_press(on_press)
    .padding([5, 12])
    .style(if active {
        theme::primary_button
    } else {
        theme::secondary_button
    })
    .into()
}

/// Compact toggle for layer-property flags (e.g. visibility, lock).
///
/// Same semantics as [`toggle_btn`] but smaller padding and `tab_inactive_button` for the off state.
pub fn compact_toggle_btn<'a>(
    label: &'a str,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    button(text(label).size(11))
        .on_press(on_press)
        .style(if active {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        })
        .padding([4, 8])
        .into()
}

/// Mutually-exclusive option button (e.g. speed presets, fit modes, alignment).
///
/// Renders `primary_button` for the selected option and `secondary_button` for the rest.
pub fn option_btn<'a>(label: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(text(label).size(11))
        .on_press(on_press)
        .padding([5, 10])
        .style(if active {
            theme::primary_button
        } else {
            theme::secondary_button
        })
        .into()
}
