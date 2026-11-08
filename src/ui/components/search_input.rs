//! A search input.

use gpui_kit::component::input::Input;
use gpui_kit::{Entity, IntoElement, SharedString};

/// The global search id.
pub fn global_search_id() -> &'static str {
    "global-search"
}

/// A search field.
pub fn search_input(entity: &Entity<gpui_kit::component::input::InputState>) -> impl IntoElement {
    Input::new(entity)
}

/// A search field with an id.
pub fn search_input_with_id(
    _id: impl Into<SharedString>,
    entity: &Entity<gpui_kit::component::input::InputState>,
) -> impl IntoElement {
    Input::new(entity)
}
