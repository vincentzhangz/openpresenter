//! Small toggle and option buttons.

use gpui_kit::SharedString;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};

/// A binary toggle button.
pub fn toggle_btn(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> Button {
    let mut button = Button::new(id).label(label);
    if active {
        button = button.primary();
    } else {
        button = button.secondary();
    }
    button.on_click(on_click)
}

/// A compact toggle for flags.
pub fn compact_toggle_btn(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> Button {
    let mut button = Button::new(id).label(label).xsmall();
    if active {
        button = button.primary();
    } else {
        button = button.ghost();
    }
    button.on_click(on_click)
}

/// A mutually-exclusive option button.
pub fn option_btn(
    id: impl Into<gpui_kit::ElementId>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> Button {
    let mut button = Button::new(id).label(label).xsmall();
    if active {
        button = button.primary();
    } else {
        button = button.ghost();
    }
    button.on_click(on_click)
}
