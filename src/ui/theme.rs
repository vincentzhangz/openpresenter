use iced::{
    Background, Border, Color,
    widget::{button, container},
};

pub const BG_DARKEST: Color = Color::from_rgb(0.102, 0.102, 0.102);
pub const BG_DARK: Color = Color::from_rgb(0.129, 0.129, 0.129);
pub const BG_PANEL: Color = Color::from_rgb(0.149, 0.149, 0.149);
pub const BG_TOOLBAR: Color = Color::from_rgb(0.176, 0.176, 0.176);
pub const BG_HOVER: Color = Color::from_rgb(0.200, 0.200, 0.200);
pub const BG_ACTIVE: Color = Color::from_rgb(0.227, 0.227, 0.227);

pub const BORDER_PANEL: Color = Color::from_rgb(0.184, 0.184, 0.184);
pub const BORDER_STRONG: Color = Color::from_rgb(0.251, 0.251, 0.251);

pub const ACCENT_BLUE: Color = Color::from_rgb(0.204, 0.471, 0.965);
pub const ACCENT_BLUE_HOVER: Color = Color::from_rgb(0.126, 0.376, 0.816);
pub const ACCENT_BLUE_ACTIVE: Color = Color::from_rgb(0.102, 0.310, 0.710);

pub const LIVE_GREEN: Color = Color::from_rgb(0.204, 0.780, 0.349);
pub const DANGER_RED: Color = Color::from_rgb(1.000, 0.271, 0.227);
pub const WARNING_AMBER: Color = Color::from_rgb(1.000, 0.624, 0.039);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.878, 0.878, 0.878);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.600, 0.600, 0.600);
pub const TEXT_MUTED: Color = Color::from_rgb(0.400, 0.400, 0.400);

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
            ACCENT_BLUE
        } else {
            BORDER_STRONG
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

pub fn primary_button(theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);
    button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => ACCENT_BLUE_HOVER,
            button::Status::Pressed => ACCENT_BLUE_ACTIVE,
            _ => ACCENT_BLUE,
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
