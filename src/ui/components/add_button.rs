//! An add button.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::{IntoElement, SharedString};

/// A small add button.
pub fn add_button(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    Button::new(id).label(label).xsmall().on_click(on_click)
}
