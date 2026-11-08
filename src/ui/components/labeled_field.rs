//! A labeled field column.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, SharedString, div, px};

use crate::ui::theme;

/// A label above a field.
pub fn field_col(label: impl Into<SharedString>, field: impl IntoElement) -> impl IntoElement {
    let label: SharedString = label.into();
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(label),
        )
        .child(field.into_any_element())
}
