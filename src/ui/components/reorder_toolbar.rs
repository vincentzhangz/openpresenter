//! A reorder toolbar.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, div, px};

/// Callbacks for moving an item.
#[derive(Clone, Copy)]
pub struct ReorderControls {
    /// Move up.
    pub up: bool,
    /// Move down.
    pub down: bool,
}

/// A small up/down toolbar.
pub fn reorder_toolbar(
    _up: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
    _down: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    div().flex().flex_row().gap(px(4.))
}
