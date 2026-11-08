//! A full-width selectable list row.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, SharedString, div, px};

use crate::ui::theme;

/// One list row.
pub fn list_item(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
    div()
        .id(id)
        .w_full()
        .p(px(6.))
        .rounded(px(4.))
        .bg(if selected {
            theme::c(theme::BG_ACTIVE)
        } else {
            theme::c(theme::TRANSPARENT)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .on_click(on_click)
        .child(label)
}
