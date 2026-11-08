//! An error toast.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, SharedString, div, px};

use crate::ui::theme;

/// A top-right error toast.
pub fn error_toast(
    message: impl Into<SharedString>,
    on_dismiss: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static,
) -> impl IntoElement {
    let message: SharedString = message.into();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .p(px(8.))
        .rounded(px(6.))
        .border_1()
        .border_color(theme::c(theme::DANGER_RED))
        .bg(theme::c(theme::BG_TOOLBAR))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(message),
        )
        .child(
            Button::new("toast-dismiss")
                .label("Dismiss")
                .ghost()
                .xsmall()
                .on_click(on_dismiss),
        )
}
