use iced::{
    Background, Border, Color,
    widget::{button, container},
};

// ProPresenter-style charcoal palette.
pub const BG_DARKEST: Color = Color::from_rgb(0.094, 0.094, 0.094); // ~#181818
pub const BG_DARK: Color = Color::from_rgb(0.118, 0.118, 0.118); // ~#1e1e1e
pub const BG_PANEL: Color = Color::from_rgb(0.137, 0.137, 0.137); // ~#232323
pub const BG_TOOLBAR: Color = Color::from_rgb(0.153, 0.153, 0.153); // ~#272727
pub const BG_HOVER: Color = Color::from_rgb(0.200, 0.200, 0.200);
pub const BG_ACTIVE: Color = Color::from_rgb(0.235, 0.235, 0.235);

pub const BORDER_PANEL: Color = Color::from_rgb(0.180, 0.180, 0.180);
pub const BORDER_STRONG: Color = Color::from_rgb(0.110, 0.110, 0.110); // hairline separators

// ProPresenter selection accent (electric orange).
pub const ACCENT_ORANGE: Color = Color::from_rgb(0.941, 0.216, 0.031); // ~#f03708
pub const ACCENT_ORANGE_HOVER: Color = Color::from_rgb(1.000, 0.420, 0.200);
pub const ACCENT_ORANGE_ACTIVE: Color = Color::from_rgb(0.800, 0.180, 0.020);
// Legacy blue kept for compatibility with existing call sites.
pub const ACCENT_BLUE: Color = ACCENT_ORANGE;
pub const ACCENT_BLUE_HOVER: Color = ACCENT_ORANGE_HOVER;
pub const ACCENT_BLUE_ACTIVE: Color = ACCENT_ORANGE_ACTIVE;

pub const LIVE_GREEN: Color = Color::from_rgb(0.204, 0.780, 0.349);
pub const DANGER_RED: Color = Color::from_rgb(1.000, 0.271, 0.227);
pub const WARNING_AMBER: Color = Color::from_rgb(1.000, 0.624, 0.039);
pub const LINK_YELLOW: Color = Color::from_rgb(1.000, 0.835, 0.000);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.886, 0.886, 0.886);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.620, 0.620, 0.620);
pub const TEXT_MUTED: Color = Color::from_rgb(0.420, 0.420, 0.420);

pub const OVERLAY: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.78);

pub const TRANSPARENT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.0);

pub fn base_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARKEST)),
        ..Default::default()
    }
}

pub fn panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_PANEL)),
        border: Border {
            color: BORDER_PANEL,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn dark_panel_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARK)),
        border: Border {
            color: BORDER_PANEL,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn toolbar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_TOOLBAR)),
        border: Border {
            color: BORDER_STRONG,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn navbar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARKEST)),
        border: Border {
            color: BORDER_STRONG,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn canvas_bg_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.067, 0.067, 0.067))),
        ..Default::default()
    }
}

pub fn slide_thumbnail_style(
    selected: bool,
    live: bool,
) -> impl Fn(&iced::Theme) -> container::Style {
    move |_theme: &iced::Theme| {
        let border_color = if live {
            LIVE_GREEN
        } else if selected {
            ACCENT_ORANGE
        } else {
            BORDER_PANEL
        };
        container::Style {
            border: Border {
                color: border_color,
                width: if selected || live { 2.0 } else { 1.0 },
                radius: 3.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn tab_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_PANEL)),
        border: Border {
            color: BORDER_STRONG,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn overlay_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(OVERLAY)),
        ..Default::default()
    }
}

pub fn dialog_card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_TOOLBAR)),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn drag_handle_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
            width: 1.0,
            radius: 2.0.into(),
        },
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
        ..Default::default()
    }
}

pub fn section_header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARK)),
        border: Border {
            color: BORDER_STRONG,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// Compact uppercase section title used as a ProPresenter-style rail header.
pub fn section_title_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARKEST)),
        border: Border {
            color: BORDER_STRONG,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn primary_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => ACCENT_ORANGE_HOVER,
            button::Status::Pressed => ACCENT_ORANGE_ACTIVE,
            _ => ACCENT_ORANGE,
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 5.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        ..base
    }
}

/// Compact square icon button used in the ProPresenter-style top toolbar (~30px).
pub fn toolbar_icon_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let mut s = ghost_button(theme, status);
    s.border = Border {
        radius: 4.0.into(),
        width: 1.0,
        color: match status {
            button::Status::Hovered => BORDER_PANEL,
            _ => TRANSPARENT,
        },
    };
    s
}

pub fn secondary_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::secondary(theme, status);
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => BG_ACTIVE,
            button::Status::Pressed => BG_HOVER,
            _ => BG_HOVER,
        })),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 5.0.into(),
        },
        ..base
    }
}

pub fn danger_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::danger(theme, status);
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color::from_rgb(0.85, 0.2, 0.15),
            button::Status::Pressed => Color::from_rgb(0.7, 0.15, 0.1),
            _ => DANGER_RED,
        })),
        text_color: Color::WHITE,
        border: Border {
            radius: 5.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        ..base
    }
}

pub fn ghost_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => BG_HOVER,
            button::Status::Pressed => BG_ACTIVE,
            _ => TRANSPARENT,
        })),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

pub fn tab_active_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    primary_button(theme, status)
}

pub fn tab_inactive_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => BG_ACTIVE,
            _ => TRANSPARENT,
        })),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 5.0.into(),
            width: 0.0,
            color: TRANSPARENT,
        },
        ..ghost_button(theme, status)
    }
}

pub fn swatch_button(
    color: Color,
    selected: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme: &iced::Theme, status| button::Style {
        background: Some(Background::Color(color)),
        border: Border {
            color: if selected {
                Color::WHITE
            } else if matches!(status, button::Status::Hovered) {
                Color::from_rgba(1.0, 1.0, 1.0, 0.6)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.25)
            },
            width: if selected { 2.0 } else { 1.0 },
            radius: 4.0.into(),
        },
        text_color: TRANSPARENT,
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

pub fn invisible_button(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: TRANSPARENT,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: false,
    }
}
