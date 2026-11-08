//! Section headers.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, SharedString, div, px};

use crate::ui::theme;

/// A full section header.
pub fn section_header(title: impl Into<SharedString>) -> impl IntoElement {
    let title: SharedString = title.into();
    div()
        .w_full()
        .p(px(6.))
        .bg(theme::c(theme::BG_DARK))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .child(title)
}

/// A small uppercase label.
pub fn section_label(text: impl Into<SharedString>) -> impl IntoElement {
    let text: SharedString = text.into();
    div()
        .text_size(px(10.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child(text)
}
