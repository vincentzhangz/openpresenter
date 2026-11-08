//! A hairline divider.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, div, px};

use crate::ui::theme;

/// A horizontal rule.
pub fn divider() -> impl IntoElement {
    div().h(px(1.)).w_full().bg(theme::c(theme::BORDER_PANEL))
}
