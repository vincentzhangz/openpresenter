//! An empty state.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, SharedString, div, px};

use crate::ui::theme;

/// Centered muted text for an empty list.
pub fn empty_state(text: impl Into<SharedString>) -> impl IntoElement {
    let text: SharedString = text.into();
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .p(px(16.))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child(text)
}
