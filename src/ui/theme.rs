//! Dark charcoal palette, exposed as GPUI colors.
//!
//! The iced-era colors are kept as hex constants so every panel reads
//! the same value; `c()` converts one to a GPUI `Hsla`.

use gpui_kit::{App, Hsla, rgb};

/// Darkest background (~#181818).
pub const BG_DARKEST: u32 = 0x181818;
/// Dark background (~#1e1e1e).
pub const BG_DARK: u32 = 0x1e1e1e;
/// Panel background (~#232323).
pub const BG_PANEL: u32 = 0x232323;
/// Toolbar background (~#272727).
pub const BG_TOOLBAR: u32 = 0x272727;
/// Hover background (~#333333).
pub const BG_HOVER: u32 = 0x333333;
/// Active background (~#3c3c3c).
pub const BG_ACTIVE: u32 = 0x3c3c3c;

/// Panel border (~#2e2e2e).
pub const BORDER_PANEL: u32 = 0x2e2e2e;
/// Hairline separator (~#1c1c1c).
pub const BORDER_STRONG: u32 = 0x1c1c1c;

/// Electric orange accent.
pub const ACCENT_ORANGE: u32 = 0xf03708;
/// Orange on hover.
pub const ACCENT_ORANGE_HOVER: u32 = 0xff6b33;
/// Orange while pressed.
pub const ACCENT_ORANGE_ACTIVE: u32 = 0xcc2e06;
/// Legacy blue name, kept for existing call sites.
pub const ACCENT_BLUE: u32 = 0xf03708;
/// Legacy blue on hover.
pub const ACCENT_BLUE_HOVER: u32 = 0xff6b33;
/// Legacy blue while pressed.
pub const ACCENT_BLUE_ACTIVE: u32 = 0xcc2e06;

/// Live indicator green.
pub const LIVE_GREEN: u32 = 0x34c759;
/// Danger red.
pub const DANGER_RED: u32 = 0xff453a;
/// Warning amber.
pub const WARNING_AMBER: u32 = 0xff9f0a;
/// Link yellow.
pub const LINK_YELLOW: u32 = 0xffd60a;

/// Primary text.
pub const TEXT_PRIMARY: u32 = 0xe2e2e2;
/// Secondary text.
pub const TEXT_SECONDARY: u32 = 0x9e9e9e;
/// Muted text.
pub const TEXT_MUTED: u32 = 0x6b6b6b;

/// Transparent.
pub const TRANSPARENT: u32 = 0x00000000;

/// Convert a hex color to a GPUI color.
pub fn c(hex: u32) -> Hsla {
    if hex == TRANSPARENT {
        return gpui_kit::transparent_black();
    }
    // Include an opaque alpha channel.
    rgb(hex | 0xff00_0000).into()
}

/// Install the app theme: Dark mode with the charcoal/orange palette.
///
/// Panels read colors via `c()` directly, but gpui-kit controls
/// (`Button`, `Input`, `Slider`) read the global `Theme` — without this
/// they paint in the default light theme (the white pills). This sets
/// the tokens those controls use.
pub fn install(cx: &mut App) {
    use gpui_kit::component::{Theme, ThemeMode};
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.background = c(BG_DARKEST);
        theme.foreground = c(TEXT_PRIMARY);
        theme.border = c(BORDER_PANEL);
        theme.accent = c(ACCENT_ORANGE);
        theme.accent_foreground = c(0xffffff);
        theme.primary = c(ACCENT_ORANGE);
        theme.primary_hover = c(ACCENT_ORANGE_HOVER);
        theme.primary_active = c(ACCENT_ORANGE_ACTIVE);
        theme.primary_foreground = c(0xffffff);
        theme.button_primary = c(ACCENT_ORANGE);
        theme.button_primary_hover = c(ACCENT_ORANGE_HOVER);
        theme.button_primary_active = c(ACCENT_ORANGE_ACTIVE);
        theme.button_primary_foreground = c(0xffffff);
        theme.secondary = c(BG_HOVER);
        theme.secondary_hover = c(BG_ACTIVE);
        theme.secondary_active = c(BG_ACTIVE);
        theme.button_secondary = c(BG_HOVER);
        theme.button_secondary_hover = c(BG_ACTIVE);
        theme.button_secondary_foreground = c(TEXT_PRIMARY);
        theme.button = c(BG_HOVER);
        theme.button_hover = c(BG_ACTIVE);
        theme.button_active = c(BG_ACTIVE);
        theme.button_foreground = c(TEXT_PRIMARY);
        theme.input = c(BG_PANEL);
        theme.muted = c(BG_PANEL);
        theme.popover = c(BG_PANEL);
        theme.colors.list = c(BG_PANEL);
        theme.list_hover = c(BG_HOVER);
        theme.list_active = c(BG_ACTIVE);
        theme.tab = c(BG_PANEL);
        theme.tab_active = c(ACCENT_ORANGE);
        theme.tab_foreground = c(TEXT_SECONDARY);
        theme.tab_active_foreground = c(0xffffff);
        theme.tab_bar = c(BG_PANEL);
        theme.slider_bar = c(ACCENT_ORANGE);
        theme.slider_thumb = c(0xffffff);
        theme.success = c(LIVE_GREEN);
        theme.danger = c(DANGER_RED);
        theme.warning = c(WARNING_AMBER);
        theme.link = c(LINK_YELLOW);
        theme.selection = c(ACCENT_ORANGE).opacity(0.35);
        theme.sidebar = c(BG_DARK);
        theme.title_bar = c(BG_TOOLBAR);
    });
}
