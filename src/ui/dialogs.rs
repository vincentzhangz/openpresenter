use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text, text_input},
};

pub fn new_presentation_dialog<'a>(name: &'a str) -> Element<'a, Message> {
    let card = column![
        text("New Presentation").size(18).color(theme::TEXT_PRIMARY),
        Space::new().height(16),
        text("Name:").size(13).color(theme::TEXT_SECONDARY),
        Space::new().height(6),
        text_input("Presentation name…", name)
            .on_input(Message::NewPresentationNameChanged)
            .on_submit(Message::CreatePresentation)
            .padding([10, 12])
            .size(14)
            .width(320),
        Space::new().height(20),
        row![
            button(text("Create").size(13))
                .on_press(Message::CreatePresentation)
                .padding([9, 28])
                .style(theme::primary_button),
            button(text("Cancel").size(13))
                .on_press(Message::CancelNewPresentation)
                .padding([9, 20])
                .style(theme::secondary_button),
        ]
        .spacing(10),
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

pub fn shortcuts_overlay<'a>() -> Element<'a, Message> {
    let shortcut = |key: &'static str, desc: &'static str| -> Element<'a, Message> {
        row![
            container(text(key).size(11).color(theme::TEXT_SECONDARY)).width(140),
            text(desc).size(11).color(theme::TEXT_MUTED),
        ]
        .spacing(8)
        .into()
    };

    let card = column![
        text("Keyboard Shortcuts")
            .size(16)
            .color(theme::TEXT_PRIMARY),
        Space::new().height(14),
        text("EDITOR").size(10).color(theme::TEXT_MUTED),
        Space::new().height(4),
        shortcut("?", "Toggle this overlay"),
        shortcut("⌘Z / Ctrl+Z", "Undo"),
        shortcut("⌘⇧Z / Ctrl+Y", "Redo"),
        Space::new().height(10),
        text("PRESENTER").size(10).color(theme::TEXT_MUTED),
        Space::new().height(4),
        shortcut("→ or Space", "Next slide"),
        shortcut("←", "Previous slide"),
        shortcut("Escape", "Return to library"),
        Space::new().height(20),
        button(text("Close").size(13))
            .on_press(Message::ToggleShortcutsOverlay)
            .padding([8, 24])
            .style(theme::secondary_button),
    ]
    .padding(28)
    .spacing(4)
    .align_x(Alignment::Start);

    let dialog = container(card).width(380).style(theme::dialog_card_style);

    container(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(theme::overlay_style)
        .into()
}

pub fn confirm_delete_dialog<'a>(
    kind: &'a str,
    item_name: &'a str,
    confirm: Message,
    cancel: Message,
) -> Element<'a, Message> {
    let title = format!("Delete {kind}");
    let card = column![
        text(title).size(18).color(theme::TEXT_PRIMARY),
        Space::new().height(14),
        text(format!("Delete \"{}\"?", item_name))
            .size(14)
            .color(theme::TEXT_SECONDARY),
        Space::new().height(6),
        text("This action cannot be undone.")
            .size(12)
            .color(theme::TEXT_MUTED),
        Space::new().height(24),
        row![
            button(text("Delete").size(13))
                .on_press(confirm)
                .padding([9, 24])
                .style(theme::danger_button),
            button(text("Cancel").size(13))
                .on_press(cancel)
                .padding([9, 20])
                .style(theme::secondary_button),
        ]
        .spacing(10),
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

pub fn delete_confirmation_dialog<'a>(presentation_name: &'a str) -> Element<'a, Message> {
    confirm_delete_dialog(
        "Presentation",
        presentation_name,
        Message::ConfirmDeletePresentation,
        Message::CancelDelete,
    )
}
