//! A single colour-channel slider row and a colour swatch.

use gpui_kit::component::slider::Slider;
use gpui_kit::prelude::*;
use gpui_kit::{Entity, Hsla, IntoElement, SharedString, div, px};

use crate::ui::theme;

/// One R / G / B / A channel: label, slider, and readout.
pub fn color_channel_slider(
    label: impl Into<SharedString>,
    value: u8,
    slider: &Entity<gpui_kit::component::slider::SliderState>,
) -> impl IntoElement {
    let label: SharedString = label.into();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .w(px(14.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(label),
        )
        .child(Slider::new(slider))
        .child(
            div()
                .w(px(28.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("{value}")),
        )
}

/// A clickable colour swatch.
pub fn color_swatch_btn(
    id: impl Into<gpui_kit::ElementId>,
    color: Hsla,
    selected: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    let border = if selected {
        theme::c(0xffffff)
    } else {
        theme::c(theme::BORDER_PANEL)
    };
    div()
        .id(id)
        .w(px(28.))
        .h(px(28.))
        .rounded(px(4.))
        .border_1()
        .border_color(border)
        .bg(color)
        .on_click(on_click)
}
