//! A tab button and the bar that holds a row of them.

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, IntoElement, SharedString, div, px};

use crate::ui::theme;

/// One tab in a tab bar.
pub fn tab_btn(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> Button {
    use gpui_kit::component::Sizable;
    let mut button = Button::new(id).label(label).xsmall().px(px(4.)).py(px(3.));
    if active {
        button = button.primary();
    } else {
        button = button.ghost();
    }
    button.on_click(on_click)
}

/// A row of tabs.
pub fn tab_bar(tabs: Vec<AnyElement>) -> impl IntoElement {
    let mut row = div()
        .flex()
        .flex_row()
        .gap(px(1.))
        .p(px(2.))
        .bg(theme::c(theme::BG_PANEL));
    for tab in tabs {
        row = row.child(tab);
    }
    row
}
