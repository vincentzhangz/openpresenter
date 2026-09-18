use crate::domain::{Presentation, ShapeType, TextAlignment};
use crate::ui::editor::canvas;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, ViewMode};
use crate::ui::shell;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, slider, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

const RIGHT_W: f32 = 320.0;

/// Central column of the unified workspace shell.
/// * Edit mode: large slide canvas (no rulers) + bottom object strip.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    if w.presenting.stage_display_active {
        return crate::ui::stage::view(w);
    }
    if w.shell.current_mode == ViewMode::Edit {
        return edit_center(w);
    }
    match w.shell.sidebar_tab {
        crate::ui::messages::SidebarTab::Playlists => crate::ui::playlist::editor_view(w),
        crate::ui::messages::SidebarTab::Songs => crate::ui::songs::editor_view(w),
        crate::ui::messages::SidebarTab::Bible => crate::ui::bible::browser_view(w),
        crate::ui::messages::SidebarTab::Presentations
        | crate::ui::messages::SidebarTab::Library => show_center(w),
    }
}

fn show_center<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let pres = w.presenting.presentation.as_ref();
    let header = presentation_header(w, pres);

    let grid = shell::show::slides_workspace(
        pres,
        w.presenting.slide_index,
        w.presenting.slide_context_index,
        w.presenting.slide_context_pos,
        w.presenting.group_submenu,
        w.presenting.cue_submenu,
        w.presenting.transition_submenu,
        w.presenting.slide_grid_cols,
        w.presenting.slide_view_mode,
    );

    let bottom = bottom_bar(w);

    let col = column![header, grid, bottom]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::base_style)
        .into()
}

fn presentation_header<'a>(
    w: &'a MainWindow,
    pres: Option<&'a Presentation>,
) -> Element<'a, Message> {
    let name = pres
        .map(|p| p.name.as_str())
        .unwrap_or("No Presentation Selected");
    let slide_count = pres.map(|p| p.slides.len()).unwrap_or(0);
    let group = pres
        .and_then(|p| p.slides.get(w.presenting.slide_index))
        .and_then(|s| s.group.as_deref())
        .unwrap_or("—");

    let display_name = crate::ui::components::truncate(name, 36);

    let mut header_row = row![
        fa_icon_solid("desktop")
            .size(13.0_f32)
            .color(if pres.is_some() {
                theme::LIVE_GREEN
            } else {
                theme::TEXT_MUTED
            }),
        text(display_name).size(13).color(theme::TEXT_PRIMARY),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if slide_count > 0 {
        header_row = header_row.push(
            container(
                text(format!("{slide_count} slides"))
                    .size(10)
                    .color(theme::TEXT_MUTED),
            )
            .padding([2, 6])
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }

    if group != "—" {
        header_row = header_row.push(
            container(text(group).size(10).color(theme::TEXT_SECONDARY))
                .padding([2, 8])
                .style(|_: &iced::Theme| iced::widget::container::Style {
                    background: Some(Background::Color(Color::from_rgba(
                        0.941, 0.216, 0.031, 0.16,
                    ))),
                    border: Border {
                        color: theme::ACCENT_ORANGE,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                }),
        );
    }

    header_row = header_row.push(Space::new().width(Length::Fill));

    header_row = header_row
        .push(small_icon_btn(
            "magnifying-glass",
            "Find",
            Some(Message::FocusSearch),
        ))
        .push(small_icon_btn(
            "clock",
            "Timer",
            Some(Message::from(crate::ui::stage::Message::ToggleTimer)),
        ))
        .push(small_icon_btn(
            "layer-group",
            "Layers",
            Some(Message::SwitchInspectorTab(
                crate::ui::messages::InspectorTab::Layers,
            )),
        ));

    container(header_row)
        .width(Length::Fill)
        .padding([8, 12])
        .style(theme::section_header_style)
        .into()
}

fn bottom_bar<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let cur_trans = w.presenting.global_transition;
    let is_cut = matches!(cur_trans, crate::domain::Transition::Cut);
    let is_fade_300 = matches!(
        cur_trans,
        crate::domain::Transition::Fade { duration_ms: 300 }
    );
    let is_fade_500 = matches!(
        cur_trans,
        crate::domain::Transition::Fade { duration_ms: 500 }
    );
    let is_fade_1000 = matches!(
        cur_trans,
        crate::domain::Transition::Fade { duration_ms: 1000 }
    );

    let transition_controls = row![
        text("Transition").size(11).color(theme::TEXT_MUTED),
        Space::new().width(4),
        transition_btn(
            "Cut",
            is_cut,
            Message::SetGlobalTransition(crate::domain::Transition::Cut)
        ),
        transition_btn(
            "0.3s",
            is_fade_300,
            Message::SetGlobalTransition(crate::domain::Transition::Fade { duration_ms: 300 })
        ),
        transition_btn(
            "0.5s",
            is_fade_500,
            Message::SetGlobalTransition(crate::domain::Transition::Fade { duration_ms: 500 })
        ),
        transition_btn(
            "1.0s",
            is_fade_1000,
            Message::SetGlobalTransition(crate::domain::Transition::Fade { duration_ms: 1000 })
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let cur_cols = w.presenting.slide_grid_cols.clamp(2, 6);
    let zoom_val = (8 - cur_cols) as f32; // 2 cols = zoom 6, 6 cols = zoom 2
    let zoom_slider = slider(2.0..=6.0, zoom_val, move |v: f32| {
        let cols = 8 - (v.round() as usize).clamp(2, 6);
        Message::SetSlideGridCols(cols)
    })
    .step(1.0_f32)
    .width(90);

    let is_grid = w.presenting.slide_view_mode == crate::ui::messages::SlideViewMode::Grid;
    let is_table = w.presenting.slide_view_mode == crate::ui::messages::SlideViewMode::Table;

    let view_mode_toggles = row![
        button(
            fa_icon_solid("table-cells")
                .size(11.0_f32)
                .color(if is_grid {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_MUTED
                })
        )
        .on_press(Message::SetSlideViewMode(
            crate::ui::messages::SlideViewMode::Grid
        ))
        .padding([4, 6])
        .style(if is_grid {
            theme::primary_button
        } else {
            theme::ghost_button
        }),
        button(fa_icon_solid("list-ul").size(11.0_f32).color(if is_table {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        }))
        .on_press(Message::SetSlideViewMode(
            crate::ui::messages::SlideViewMode::Table
        ))
        .padding([4, 6])
        .style(if is_table {
            theme::primary_button
        } else {
            theme::ghost_button
        }),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    let zoom_controls = row![
        view_mode_toggles,
        Space::new().width(4),
        zoom_slider,
        Space::new().width(4),
        text(format!("{cur_cols} cols"))
            .size(10)
            .color(theme::TEXT_SECONDARY),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let right_controls = row![
        zoom_controls,
        Space::new().width(6),
        small_icon_btn(
            "gear",
            "Outputs",
            Some(Message::Output(crate::ui::output::Message::SettingsOpen)),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let row_el = row![
        transition_controls,
        Space::new().width(Length::Fill),
        right_controls,
    ]
    .align_y(Alignment::Center)
    .padding([4, 12])
    .spacing(8);

    container(row_el)
        .width(Length::Fill)
        .style(theme::section_header_style)
        .into()
}

fn transition_btn(
    label: &'static str,
    active: bool,
    on_press: Message,
) -> Element<'static, Message> {
    button(text(label).size(10).color(if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    }))
    .on_press(on_press)
    .padding([3, 6])
    .style(move |_t: &iced::Theme, _s| iced::widget::button::Style {
        background: Some(Background::Color(if active {
            Color::from_rgba(0.204, 0.471, 0.965, 0.22)
        } else {
            theme::TRANSPARENT
        })),
        border: Border {
            color: if active {
                theme::ACCENT_BLUE
            } else {
                theme::BORDER_PANEL
            },
            width: 1.0,
            radius: 3.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn edit_center<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let editing = match w.editor.editing.as_ref() {
        Some(p) => p,
        None => {
            return show_center(w);
        }
    };
    let selected = w.editor.selected_slide_index;
    let slide = selected.and_then(|i| editing.slides.get(i));

    let (is_bold, is_italic, is_shadow, is_outline, font_size) = {
        let sel_layer = w.layer.selected_index;
        let mut b = false;
        let mut it = false;
        let mut sh = false;
        let mut out = false;
        let mut fs = 64.0_f32;

        if let Some(s) = slide {
            if let Some(idx) = sel_layer
                && let Some(layer) = s.layers.get(idx)
                && let crate::domain::ObjectContent::Text { style, .. } = &layer.content
            {
                b = style.bold;
                it = style.italic;
                sh = style.shadow;
                out = style.outline;
                fs = style.font_size;
            } else if let crate::domain::SlideContent::Text { style, .. } = &s.content {
                b = style.bold;
                it = style.italic;
                sh = style.shadow;
                out = style.outline;
                fs = style.font_size;
            } else if let Some(layer) = s
                .layers
                .iter()
                .find(|l| matches!(l.content, crate::domain::ObjectContent::Text { .. }))
                && let crate::domain::ObjectContent::Text { style, .. } = &layer.content
            {
                b = style.bold;
                it = style.italic;
                sh = style.shadow;
                out = style.outline;
                fs = style.font_size;
            }
        }
        (b, it, sh, out, fs)
    };

    let insert_tools = row![
        ribbon_btn(
            "font",
            "Text",
            false,
            Message::from(crate::ui::layers::Message::AddTextLayer),
        ),
        ribbon_btn(
            "square",
            "Rect",
            false,
            Message::from(crate::ui::layers::Message::AddShapeLayer(
                ShapeType::Rectangle,
            )),
        ),
        ribbon_btn(
            "circle",
            "Circle",
            false,
            Message::from(crate::ui::layers::Message::AddShapeLayer(
                ShapeType::Ellipse,
            )),
        ),
        ribbon_btn(
            "minus",
            "Line",
            false,
            Message::from(crate::ui::layers::Message::AddShapeLayer(ShapeType::Line,)),
        ),
        ribbon_btn(
            "image",
            "Media",
            false,
            Message::from(crate::ui::slides::Message::PickImageFile),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let format_tools = row![
        button(
            fa_icon_solid("minus")
                .size(9.0_f32)
                .color(theme::TEXT_MUTED)
        )
        .on_press(Message::from(
            crate::ui::slides::Message::SlideFontSizeStep(-4.0),
        ))
        .padding([3, 6])
        .style(theme::ghost_button),
        container(
            text(format!("{:.0}", font_size))
                .size(11)
                .color(theme::TEXT_PRIMARY),
        )
        .padding([2, 4]),
        button(fa_icon_solid("plus").size(9.0_f32).color(theme::TEXT_MUTED))
            .on_press(Message::from(
                crate::ui::slides::Message::SlideFontSizeStep(4.0),
            ))
            .padding([3, 6])
            .style(theme::ghost_button),
        Space::new().width(4),
        ribbon_btn(
            "bold",
            "",
            is_bold,
            Message::from(crate::ui::slides::Message::SlideBoldToggled(!is_bold)),
        ),
        ribbon_btn(
            "italic",
            "",
            is_italic,
            Message::from(crate::ui::slides::Message::SlideItalicToggled(!is_italic)),
        ),
        ribbon_btn(
            "sun",
            "",
            is_shadow,
            Message::from(crate::ui::slides::Message::SlideShadowToggled(!is_shadow)),
        ),
        ribbon_btn(
            "border-all",
            "",
            is_outline,
            Message::from(crate::ui::slides::Message::SlideOutlineToggled(!is_outline)),
        ),
        Space::new().width(4),
        ribbon_btn(
            "align-left",
            "",
            false,
            Message::from(crate::ui::slides::Message::SlideAlignmentChanged(
                TextAlignment::Left,
            )),
        ),
        ribbon_btn(
            "align-center",
            "",
            false,
            Message::from(crate::ui::slides::Message::SlideAlignmentChanged(
                TextAlignment::Center,
            )),
        ),
        ribbon_btn(
            "align-right",
            "",
            false,
            Message::from(crate::ui::slides::Message::SlideAlignmentChanged(
                TextAlignment::Right,
            )),
        ),
        Space::new().width(4),
        ribbon_color_chip(255, 255, 255),
        ribbon_color_chip(247, 209, 84),
        ribbon_color_chip(231, 76, 60),
        ribbon_color_chip(0, 210, 211),
        ribbon_color_chip(46, 204, 113),
        button(
            fa_icon_solid("palette")
                .size(10.0_f32)
                .color(theme::TEXT_MUTED)
        )
        .on_press(Message::from(
            crate::ui::slides::Message::SlideTextColorCycle
        ))
        .padding([3, 5])
        .style(theme::ghost_button),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    let arrange_tools = row![
        ribbon_btn(
            "arrow-up",
            "",
            false,
            Message::from(crate::ui::layers::Message::MoveSelectedLayerUp),
        ),
        ribbon_btn(
            "arrow-down",
            "",
            false,
            Message::from(crate::ui::layers::Message::MoveSelectedLayerDown),
        ),
        button(
            fa_icon_solid("trash")
                .size(10.0_f32)
                .color(theme::DANGER_RED)
        )
        .on_press(Message::from(
            crate::ui::layers::Message::DeleteSelectedLayer
        ))
        .padding([4, 6])
        .style(theme::ghost_button),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    let history_tools = row![
        button(
            fa_icon_solid("rotate-left")
                .size(10.0_f32)
                .color(theme::TEXT_SECONDARY)
        )
        .on_press(Message::Undo)
        .padding([4, 6])
        .style(theme::ghost_button),
        button(
            fa_icon_solid("rotate-right")
                .size(10.0_f32)
                .color(theme::TEXT_SECONDARY)
        )
        .on_press(Message::Redo)
        .padding([4, 6])
        .style(theme::ghost_button),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    let pres_actions = row![
        text_input("Presentation name", &editing.name)
            .on_input(Message::RenamePresentationChanged)
            .padding([4, 8])
            .size(12)
            .width(160),
        button(text("Rename").size(10).color(theme::TEXT_SECONDARY))
            .on_press(Message::RenamePresentation)
            .padding([4, 8])
            .style(theme::secondary_button),
        button(fa_icon_solid("trash").size(10.0_f32))
            .on_press(Message::DeletePresentationClicked(editing.id.clone()))
            .padding([4, 8])
            .style(theme::danger_button),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let ribbon = container(
        row![
            insert_tools,
            ribbon_divider(),
            format_tools,
            ribbon_divider(),
            arrange_tools,
            ribbon_divider(),
            history_tools,
            Space::new().width(Length::Fill),
            pres_actions,
        ]
        .spacing(8)
        .padding([4, 10])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::toolbar_style);

    let horizontal_ruler = {
        let mut ticks = row![Space::new().width(24)]
            .spacing(0)
            .align_y(Alignment::Center);
        for mark in (0..=1920).step_by(200) {
            ticks = ticks.push(
                container(text(format!("{mark}")).size(8).color(theme::TEXT_MUTED))
                    .width(75)
                    .center_x(Length::Fill),
            );
        }
        container(ticks)
            .height(14)
            .width(Length::Fill)
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.09, 0.11))),
                border: Border {
                    color: theme::BORDER_PANEL,
                    width: 0.5,
                    ..Default::default()
                },
                ..Default::default()
            })
    };

    let vertical_ruler = {
        let mut ticks = column![];
        for mark in (0..=1080).step_by(200) {
            ticks = ticks.push(
                container(text(format!("{mark}")).size(8).color(theme::TEXT_MUTED))
                    .height(48)
                    .center_y(Length::Fill),
            );
        }
        container(ticks)
            .width(24)
            .height(Length::Fill)
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.09, 0.11))),
                border: Border {
                    color: theme::BORDER_PANEL,
                    width: 0.5,
                    ..Default::default()
                },
                ..Default::default()
            })
    };

    let stage = container(canvas::canvas_panel(
        slide,
        w.video.frame.as_ref(),
        w.layer.selected_index,
        w.editor.inline_editing,
        &w.editor.editing_slide_text,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([16, 16])
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb(0.12, 0.13, 0.16))),
        border: Border {
            color: theme::BORDER_STRONG,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    let canvas_area = row![vertical_ruler, stage]
        .width(Length::Fill)
        .height(Length::Fill);

    let workspace = column![horizontal_ruler, canvas_area]
        .width(Length::Fill)
        .height(Length::Fill);

    let col = column![ribbon, workspace]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn ribbon_btn<'a>(
    icon: &'static str,
    label: &'static str,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let text_col = if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    let icon_col = if active {
        theme::ACCENT_ORANGE
    } else {
        theme::TEXT_MUTED
    };

    let content = if label.is_empty() {
        row![fa_icon_solid(icon).size(11.0_f32).color(icon_col)].align_y(Alignment::Center)
    } else {
        row![
            fa_icon_solid(icon).size(10.0_f32).color(icon_col),
            text(label).size(11).color(text_col),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
    };

    button(content)
        .on_press(on_press)
        .padding([4, 6])
        .style(move |_: &iced::Theme, status| iced::widget::button::Style {
            background: Some(Background::Color(if active {
                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            })),
            border: Border {
                color: if active {
                    theme::ACCENT_ORANGE
                } else {
                    theme::BORDER_PANEL
                },
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn ribbon_divider<'a>() -> Element<'a, Message> {
    container(Space::new().width(1).height(18))
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))),
            ..Default::default()
        })
        .into()
}

fn ribbon_color_chip<'a>(r: u8, g: u8, b: u8) -> Element<'a, Message> {
    let col = crate::domain::Color { r, g, b, a: 255 };
    button(Space::new().width(12).height(12))
        .on_press(Message::from(
            crate::ui::slides::Message::SlideColorChanged(col),
        ))
        .padding([2, 2])
        .style(move |_: &iced::Theme, _| iced::widget::button::Style {
            background: Some(Background::Color(Color::from_rgb8(r, g, b))),
            border: Border {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn small_icon_btn(
    icon: &'static str,
    label: &'static str,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let content = row![
        fa_icon_solid(icon)
            .size(12.0_f32)
            .color(theme::TEXT_SECONDARY),
        if label.is_empty() {
            Element::new(Space::new().width(0))
        } else {
            text(label).size(10).color(theme::TEXT_MUTED).into()
        }
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let mut b = button(content).padding([4, 8]).style(theme::ghost_button);
    if let Some(msg) = on_press {
        b = b.on_press(msg);
    }
    b.into()
}

// Re-export so callers can reference the right-dock width from one place.
pub const RIGHT_DOCK_W: f32 = RIGHT_W;
