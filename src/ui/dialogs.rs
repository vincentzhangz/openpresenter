use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

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
            container(text(key).size(11).color(theme::TEXT_SECONDARY)).width(150),
            text(desc).size(11).color(theme::TEXT_MUTED),
        ]
        .spacing(8)
        .into()
    };

    let card = column![
        row![
            fa_icon_solid("keyboard")
                .size(16.0_f32)
                .color(theme::ACCENT_ORANGE),
            Space::new().width(8),
            text("Keyboard Shortcuts & Hotkeys")
                .size(16)
                .color(theme::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
        Space::new().height(12),
        text("SHOW & LIVE NAVIGATION")
            .size(10)
            .color(theme::TEXT_MUTED),
        Space::new().height(4),
        shortcut("→ / ↓ / Space / PgDn", "Advance to next slide"),
        shortcut("← / ↑ / PgUp", "Return to previous slide"),
        shortcut("1 … 9", "Jump directly to slide 1–9"),
        shortcut("V", "Jump to next Verse group"),
        shortcut("C", "Jump to next Chorus group"),
        shortcut("Home / End", "First / Last slide"),
        Space::new().height(10),
        text("CLEAR CONTROLS").size(10).color(theme::TEXT_MUTED),
        Space::new().height(4),
        shortcut("F1 / ⌘1 / Ctrl+1", "Clear All (Screen & Audio)"),
        shortcut("F2 / ⌘2 / Ctrl+2", "Clear Slide Text"),
        shortcut("F3 / ⌘3 / Ctrl+3", "Clear Media / Background"),
        shortcut("F4 / ⌘4 / Ctrl+4", "Clear Props & Overlays"),
        shortcut("F5 / ⌘5 / Ctrl+5", "Clear Audio playback"),
        shortcut("F6 / ⌘6 / Ctrl+6", "Clear Messages & Alerts"),
        shortcut("B", "Toggle Black screen"),
        Space::new().height(10),
        text("APP & SYSTEM").size(10).color(theme::TEXT_MUTED),
        Space::new().height(4),
        shortcut("⌘, / Ctrl+,", "Open Preferences / Configuration"),
        shortcut("?", "Toggle this Shortcuts overlay"),
        shortcut("⌘Z / Ctrl+Z", "Undo"),
        shortcut("⌘⇧Z / Ctrl+Y", "Redo"),
        shortcut("Escape", "Dismiss modal / end text edit"),
        Space::new().height(16),
        button(text("Close").size(12))
            .on_press(Message::ToggleShortcutsOverlay)
            .padding([7, 24])
            .style(theme::secondary_button),
    ]
    .padding(24)
    .spacing(3)
    .align_x(Alignment::Start);

    let dialog = container(card).width(440).style(theme::dialog_card_style);

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
