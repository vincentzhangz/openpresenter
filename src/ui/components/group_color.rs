//! Group colours.

use gpui_kit::prelude::*;
use gpui_kit::{IntoElement, div, px};

use crate::ui::theme;

/// Colour palette used to distinguish slide group labels.
pub const GROUP_COLORS: [(u8, u8, u8); 8] = [
    (52, 120, 246),
    (255, 149, 0),
    (52, 199, 89),
    (255, 59, 48),
    (175, 82, 222),
    (255, 204, 0),
    (0, 199, 190),
    (162, 132, 94),
];

/// The colour of a group label, chosen deterministically.
pub fn group_option_color(name: &str) -> gpui_kit::Hsla {
    let (r, g, b) = {
        let hash: usize = name.bytes().fold(0usize, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(usize::from(b))
        });
        GROUP_COLORS[hash % GROUP_COLORS.len()]
    };
    let rgb = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
    theme::c(rgb)
}

/// The colour as 0xRRGGBB.
pub fn group_option_rgb(name: &str) -> u32 {
    let (r, g, b) = {
        let hash: usize = name.bytes().fold(0usize, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(usize::from(b))
        });
        GROUP_COLORS[hash % GROUP_COLORS.len()]
    };
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// A coloured dot with the group name.
pub fn group_color(_name: &str) -> impl IntoElement {
    div()
        .w(px(8.))
        .h(px(8.))
        .rounded_full()
        .bg(theme::c(theme::ACCENT_ORANGE))
}

/// A row with the group name and its colour dot.
pub fn group_label_widget(name: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .w(px(8.))
                .h(px(8.))
                .rounded_full()
                .bg(group_option_color(name)),
        )
        .child(name.to_string())
}
