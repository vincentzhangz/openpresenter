use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element,
    widget::{Space, button, row, text},
};

/// Data required to render a [`reorder_toolbar`] for a single list item.
pub struct ReorderControls {
    /// Zero-based index of the item within the list.
    pub index: usize,
    /// Total number of items — used to hide the up/down buttons at the boundaries.
    pub total: usize,
    /// Message emitted when the move-up button is pressed.
    pub on_up: Message,
    /// Message emitted when the move-down button is pressed.
    pub on_down: Message,
    /// Message emitted when the delete button is pressed.
    pub on_delete: Message,
    /// Optional duplicate action. When `Some`, a `⧉` button is rendered.
    pub on_duplicate: Option<Message>,
}

/// Compact up / down / delete row for reorderable list items.
///
/// Up or down arrows are replaced by invisible spacers at the list boundaries.
pub fn reorder_toolbar<'a>(controls: ReorderControls) -> Element<'a, Message> {
    let up_el: Element<'a, Message> = if controls.index > 0 {
        button(text("^").size(11))
            .on_press(controls.on_up)
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let down_el: Element<'a, Message> = if controls.index + 1 < controls.total {
        button(text("v").size(11))
            .on_press(controls.on_down)
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let del = button(text("x").size(11))
        .on_press(controls.on_delete)
        .style(theme::danger_button)
        .padding([4u16, 8u16]);

    if let Some(dup_msg) = controls.on_duplicate {
        let dup = button(text("⧉").size(11))
            .on_press(dup_msg)
            .style(theme::secondary_button)
            .padding([4u16, 8u16]);
        row![up_el, down_el, dup, del]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
    } else {
        row![up_el, down_el, del]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
    }
}
