use crate::domain::Look;
use crate::output::OutputType;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{LookLayerType, Message};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Column, Row, Space, button, column, container, row, scrollable, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

/// Render the Looks & Screens Matrix modal overlay.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let looks = &w.props.manager.looks;
    let selected_id = w
        .output
        .selected_look_id
        .as_deref()
        .or(w.props.manager.active_look_id.as_deref())
        .or_else(|| looks.first().map(|l| l.id.as_str()));

    let current_look = selected_id.and_then(|id| looks.iter().find(|l| l.id == id));

    let top_bar = row![
        fa_icon_solid("sliders")
            .size(16.0_f32)
            .color(theme::ACCENT_ORANGE),
        text("LOOKS & SCREENS MATRIX")
            .size(15)
            .color(theme::TEXT_PRIMARY),
        Space::new().width(Length::Fill),
        button(
            row![
                fa_icon_solid("gear").size(12.0_f32),
                text("Configure Screens").size(12)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::Output(crate::ui::output::Message::SettingsOpen))
        .padding([6, 10])
        .style(theme::secondary_button),
        button(fa_icon_solid("xmark").size(14.0_f32))
            .on_press(Message::ToggleLooksMatrixModal)
            .padding([6, 10])
            .style(theme::ghost_button),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    // Looks selector tab strip
    let mut looks_strip = Row::new().spacing(6).align_y(Alignment::Center);
    for look in looks {
        let is_selected = Some(look.id.as_str()) == selected_id;
        let is_live = w.props.manager.active_look_id.as_deref() == Some(&look.id);

        let look_id = look.id.clone();
        let btn = button(
            row![
                if is_live {
                    fa_icon_solid("circle-check")
                        .size(11.0_f32)
                        .color(theme::LIVE_GREEN)
                } else {
                    fa_icon_solid("circle")
                        .size(9.0_f32)
                        .color(theme::TEXT_MUTED)
                },
                text(&look.name).size(12).color(if is_selected {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_MUTED
                }),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectLook(look_id))
        .padding([6, 12])
        .style(if is_selected {
            theme::primary_button
        } else {
            theme::secondary_button
        });

        looks_strip = looks_strip.push(btn);
    }

    // Add look input
    let new_look_row = row![
        text_input("New Look name…", &w.output.editing_look_name)
            .on_input(Message::LookNameInputChanged)
            .padding([5, 8])
            .width(160)
            .size(12),
        button(text("+ Add").size(11))
            .on_press_maybe(if !w.output.editing_look_name.trim().is_empty() {
                Some(Message::CreateLook(w.output.editing_look_name.clone()))
            } else {
                None
            })
            .padding([5, 10])
            .style(theme::secondary_button),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let looks_scroll = scrollable(looks_strip)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        ))
        .width(Length::Fill);

    let looks_toolbar = row![looks_scroll, new_look_row]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([4, 0]);

    // Matrix table
    let table = if let Some(look) = current_look {
        render_matrix_table(w, look)
    } else {
        container(
            text("No Looks configured. Create one above to start routing layers.")
                .size(13)
                .color(theme::TEXT_MUTED),
        )
        .padding(20)
        .into()
    };

    let footer = if let Some(look) = current_look {
        let is_live = w.props.manager.active_look_id.as_deref() == Some(&look.id);
        let look_id = look.id.clone();
        row![
            if is_live {
                text("This Look is currently LIVE on all outputs")
                    .size(12)
                    .color(theme::LIVE_GREEN)
            } else {
                text("Look is selected for editing")
                    .size(12)
                    .color(theme::TEXT_MUTED)
            },
            Space::new().width(Length::Fill),
            if !is_live {
                button(text("Make Live").size(12))
                    .on_press(Message::SelectLook(look_id.clone()))
                    .padding([6, 16])
                    .style(theme::primary_button)
            } else {
                button(text("Active").size(12))
                    .padding([6, 16])
                    .style(theme::ghost_button)
            },
            if looks.len() > 1 {
                button(text("Delete Look").size(12))
                    .on_press(Message::DeleteLook(look_id))
                    .padding([6, 12])
                    .style(theme::danger_button)
            } else {
                button(text("Default").size(12))
                    .padding([6, 12])
                    .style(theme::ghost_button)
            },
        ]
        .spacing(10)
        .align_y(Alignment::Center)
    } else {
        row![]
    };

    let dialog_box = container(
        column![top_bar, looks_toolbar, table, footer]
            .spacing(14)
            .padding(18),
    )
    .width(820)
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(theme::BG_DARK)),
        border: Border {
            color: theme::ACCENT_ORANGE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    container(dialog_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
            ..Default::default()
        })
        .into()
}

fn render_matrix_table<'a>(w: &'a MainWindow, look: &'a Look) -> Element<'a, Message> {
    // Table Header
    let col_screen = text("SCREEN OUTPUT")
        .size(11)
        .color(theme::TEXT_MUTED)
        .width(180);
    let col_type = text("TYPE").size(11).color(theme::TEXT_MUTED).width(90);
    let col_media = text("MEDIA").size(11).color(theme::TEXT_MUTED).width(64);
    let col_slide = text("SLIDE").size(11).color(theme::TEXT_MUTED).width(64);
    let col_props = text("PROPS").size(11).color(theme::TEXT_MUTED).width(64);
    let col_msgs = text("MESSAGES").size(11).color(theme::TEXT_MUTED).width(74);
    let col_mask = text("MASK").size(11).color(theme::TEXT_MUTED).width(64);
    let col_theme = text("THEME OVERRIDE")
        .size(11)
        .color(theme::TEXT_MUTED)
        .width(Length::Fill);

    let thead = container(
        row![
            col_screen, col_type, col_media, col_slide, col_props, col_msgs, col_mask, col_theme
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([8, 12]),
    )
    .style(theme::section_header_style);

    let mut rows: Column<'a, Message> = column![].spacing(4);

    for output in w.output.manager.iter() {
        let screen_id = output.id.as_str();
        let target = look.target_or_default(screen_id);

        let type_label = match &output.output_type {
            OutputType::Window => "Window",
            OutputType::Ndi { .. } => "NDI Stream",
        };

        let look_id = look.id.clone();
        let s_id = output.id.clone();

        let media_btn = toggle_cell(
            target.media_enabled,
            Message::ToggleLookScreenLayer(look_id.clone(), s_id.clone(), LookLayerType::Media),
        );
        let slide_btn = toggle_cell(
            target.slide_enabled,
            Message::ToggleLookScreenLayer(look_id.clone(), s_id.clone(), LookLayerType::Slide),
        );
        let props_btn = toggle_cell(
            target.props_enabled,
            Message::ToggleLookScreenLayer(look_id.clone(), s_id.clone(), LookLayerType::Props),
        );
        let msgs_btn = toggle_cell(
            target.messages_enabled,
            Message::ToggleLookScreenLayer(look_id.clone(), s_id.clone(), LookLayerType::Messages),
        );
        let mask_btn = toggle_cell(
            target.mask_enabled,
            Message::ToggleLookScreenLayer(look_id.clone(), s_id.clone(), LookLayerType::Mask),
        );

        let theme_label = target
            .theme_id
            .as_deref()
            .and_then(|id| w.theme_state.list.iter().find(|t| t.id == id))
            .map(|t| t.name.as_str())
            .unwrap_or("(Original Slide Style)");

        let theme_btn = button(
            row![
                text(theme_label)
                    .size(11)
                    .color(if target.theme_id.is_some() {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TEXT_MUTED
                    }),
                Space::new().width(Length::Fill),
                fa_icon_solid("chevron-down")
                    .size(9.0_f32)
                    .color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        )
        .on_press({
            let next_theme = cycle_theme(target.theme_id.as_deref(), &w.theme_state.list);
            Message::SetLookScreenTheme(look_id, s_id, next_theme)
        })
        .padding([4, 8])
        .style(theme::secondary_button)
        .width(Length::Fill);

        let row_item = container(
            row![
                text(&output.label)
                    .size(12)
                    .color(theme::TEXT_PRIMARY)
                    .width(180),
                text(type_label)
                    .size(11)
                    .color(theme::TEXT_SECONDARY)
                    .width(90),
                container(media_btn).width(64).center_x(64),
                container(slide_btn).width(64).center_x(64),
                container(props_btn).width(64).center_x(64),
                container(msgs_btn).width(74).center_x(74),
                container(mask_btn).width(64).center_x(64),
                theme_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([6, 12]),
        )
        .style(theme::dark_panel_style);

        rows = rows.push(row_item);
    }

    container(column![thead, scrollable(rows).height(240)].spacing(4))
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_DARKEST)),
            border: Border {
                color: theme::BORDER_PANEL,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn toggle_cell<'a>(active: bool, on_press: Message) -> Element<'a, Message> {
    button(
        fa_icon_solid(if active { "check" } else { "xmark" })
            .size(11.0_f32)
            .color(if active {
                theme::LIVE_GREEN
            } else {
                theme::TEXT_MUTED
            }),
    )
    .on_press(on_press)
    .padding([4, 10])
    .style(if active {
        theme::primary_button
    } else {
        theme::ghost_button
    })
    .into()
}

fn cycle_theme(current_id: Option<&str>, themes: &[crate::domain::SlideTheme]) -> Option<String> {
    if themes.is_empty() {
        return None;
    }
    match current_id {
        None => themes.first().map(|t| t.id.clone()),
        Some(id) => {
            if let Some(idx) = themes.iter().position(|t| t.id == id) {
                if idx + 1 < themes.len() {
                    Some(themes[idx + 1].id.clone())
                } else {
                    None
                }
            } else {
                themes.first().map(|t| t.id.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SlideTheme;

    #[test]
    fn cycle_theme_behavior() {
        let mut t1 = SlideTheme::new(
            "Theme 1".into(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        t1.id = "t1".into();
        let mut t2 = SlideTheme::new(
            "Theme 2".into(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        t2.id = "t2".into();
        let themes = vec![t1, t2];

        // None -> first theme
        assert_eq!(cycle_theme(None, &themes), Some("t1".into()));
        // First theme -> second theme
        assert_eq!(cycle_theme(Some("t1"), &themes), Some("t2".into()));
        // Last theme -> None
        assert_eq!(cycle_theme(Some("t2"), &themes), None);
        // Unknown id -> first theme
        assert_eq!(cycle_theme(Some("unknown"), &themes), Some("t1".into()));
        // Empty themes -> None
        assert_eq!(cycle_theme(None, &[]), None);
    }
}
