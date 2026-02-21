use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Color, Element, Length,
    widget::{Space, button, row, slider, text},
};

/// A single R / G / B / A channel slider row.
///
/// Renders a short channel label (14 px wide), a full-width slider, and a
/// numeric readout (28 px wide). All text uses `size(11)` and [`theme::TEXT_MUTED`].
pub fn color_channel_slider<'a>(
    label: &'a str,
    value: u8,
    on_change: impl Fn(u8) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(label).size(11).color(theme::TEXT_MUTED).width(14),
        slider(0.0..=255.0, value as f32, move |v| on_change(v as u8))
            .step(1.0)
            .width(Length::Fill),
        text(format!("{value}"))
            .size(11)
            .color(theme::TEXT_MUTED)
            .width(28),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

/// A 28 × 28 clickable colour swatch styled via [`theme::swatch_button`].
///
/// Pass `selected: true` to draw the selection indicator around the swatch.
pub fn color_swatch_btn<'a>(color: Color, selected: bool, msg: Message) -> Element<'a, Message> {
    button(Space::new().width(28).height(28))
        .on_press(msg)
        .padding(0)
        .style(crate::ui::theme::swatch_button(color, selected))
        .into()
}
