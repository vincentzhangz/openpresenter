//! A live badge.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, div, px};

use crate::ui::theme;

/// A small green dot with a label.
pub fn live_badge(label: &str, live: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(div().w(px(8.)).h(px(8.)).rounded_full().bg(if live {
            theme::c(theme::LIVE_GREEN)
        } else {
            theme::c(theme::TEXT_MUTED)
        }))
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(label.to_string()),
        )
}
