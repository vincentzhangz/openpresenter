use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{Alignment, Element, Length, widget::{Row, Space, button, column, container, text}};

/// A centred modal card rendered on a dimmed overlay.
///
/// Provide a `title`, arbitrary `content`, and a list of action `buttons`
/// (typically confirm + cancel). The card and overlay are styled by
/// [`theme::dialog_card_style`] and [`theme::overlay_style`] respectively.
pub fn dialog_overlay<'a>(
    title: &'a str,
    content: Element<'a, Message>,
    actions: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let action_row = Row::with_children(actions).spacing(10);

    let card = column![
        text(title).size(18).color(theme::TEXT_PRIMARY),
        Space::new().height(14),
        content,
        Space::new().height(24),
        action_row,
    ]
    .padding(28)
    .spacing(2)
    .align_x(Alignment::Start);

    let dialog = container(card).style(theme::dialog_card_style);

    container(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(theme::overlay_style)
        .into()
}

/// Pre-built confirmation dialog with title, message body, and confirm / cancel buttons.
///
/// Set `danger: true` to render the confirm button in destructive red.
pub fn confirm_dialog<'a>(
    title: &'a str,
    message: &'a str,
    confirm_label: &'a str,
    on_confirm: Message,
    on_cancel: Message,
    danger: bool,
) -> Element<'a, Message> {
    let content = column![
        text(message).size(14).color(theme::TEXT_SECONDARY),
        Space::new().height(6),
        text("This action cannot be undone.")
            .size(12)
            .color(theme::TEXT_MUTED),
    ]
    .spacing(2)
    .into();

    let actions = vec![
        button(text(confirm_label).size(13))
            .on_press(on_confirm)
            .padding([9, 24])
            .style(if danger {
                theme::danger_button
            } else {
                theme::primary_button
            })
            .into(),
        button(text("Cancel").size(13))
            .on_press(on_cancel)
            .padding([9, 20])
            .style(theme::secondary_button)
            .into(),
    ];

    dialog_overlay(title, content, actions)
}
