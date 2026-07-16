use crate::domain::Presentation;
use crate::ui::editor::canvas;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, ViewMode};
use crate::ui::shell;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Row, Space, button, column, container, row, scrollable, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

const RIGHT_W: f32 = 320.0;

/// Central column of the unified ProPresenter shell.
///
/// * Show mode: presentation header + scrollable slide-thumbnail grid + bottom
///   transition / view switch strip.
/// * Edit mode: large slide canvas (no rulers) + bottom object strip.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    match w.shell.current_mode {
        ViewMode::Show => show_center(w),
        ViewMode::Edit => edit_center(w),
    }
}

fn show_center<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    if w.presenting.stage_display_active {
        return crate::ui::stage::view(w);
    }
    let pres = w.presenting.presentation.as_ref();
    let header = presentation_header(w, pres);

    let grid = shell::show::slides_workspace(
        pres,
        w.presenting.slide_index,
        w.presenting.slide_context_index,
        w.presenting.slide_context_pos,
        w.presenting.group_submenu,
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
    let name = pres.map(|p| p.name.as_str()).unwrap_or("No Presentation");
    let group = pres
        .and_then(|p| p.slides.get(w.presenting.slide_index))
        .and_then(|s| s.group.as_deref())
        .unwrap_or("—");

    let header_row = row![
        fa_icon_solid("circle-play")
            .size(13.0_f32)
            .color(theme::LIVE_GREEN),
        text(name).size(13).color(theme::TEXT_PRIMARY),
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
        Space::new().width(Length::Fill),
        small_icon_btn("magnifying-glass", "find", true),
        small_icon_btn("clock", "timer", true),
        small_icon_btn("layer-group", "layers", true),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 12]);

    container(header_row)
        .width(Length::Fill)
        .style(theme::section_header_style)
        .into()
}

fn bottom_bar<'a>(_w: &'a MainWindow) -> Element<'a, Message> {
    let transition = container(
        row![
            text("Transition").size(11).color(theme::TEXT_MUTED),
            Space::new().width(6),
            text("Default").size(11).color(theme::TEXT_SECONDARY),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding([6, 10]);

    let view_switch = row![
        view_mode_btn("Grid", true),
        view_mode_btn("Easy", false),
        view_mode_btn("Outline", false),
        small_icon_btn("gear", "settings", false),
    ]
    .spacing(4);

    let row_el = row![transition, Space::new().width(Length::Fill), view_switch,]
        .align_y(Alignment::Center)
        .padding([6, 12])
        .spacing(8);

    container(row_el)
        .width(Length::Fill)
        .style(theme::section_header_style)
        .into()
}

fn view_mode_btn(label: &'static str, active: bool) -> Element<'static, Message> {
    button(text(label).size(10).color(if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    }))
    .padding([4, 10])
    .style(move |_t: &iced::Theme, _s| iced::widget::button::Style {
        background: Some(Background::Color(if active {
            Color::from_rgba(0.941, 0.216, 0.031, 0.18)
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

fn edit_center<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let editing = match w.editor.editing.as_ref() {
        Some(p) => p,
        None => {
            // No presentation open in edit mode: reuse the Show center instead.
            return show_center(w);
        }
    };
    let selected = w.editor.selected_slide_index;
    let slide = selected.and_then(|i| editing.slides.get(i));

    let top_bar = container(
        row![
            button(text("‹ Library").size(12).color(theme::TEXT_SECONDARY))
                .on_press(Message::BackToList)
                .padding([6, 12])
                .style(theme::ghost_button),
            Space::new().width(10),
            text_input("Presentation name", &editing.name)
                .on_input(Message::RenamePresentationChanged)
                .padding([6, 8])
                .size(13)
                .width(220),
            button(text("Rename").size(11).color(theme::TEXT_SECONDARY))
                .on_press(Message::RenamePresentation)
                .padding([6, 10])
                .style(theme::secondary_button),
            Space::new().width(Length::Fill),
            button(text("🗑 Delete").size(11))
                .on_press(Message::DeletePresentationClicked(editing.id.clone()))
                .padding([6, 12])
                .style(theme::danger_button),
        ]
        .spacing(8)
        .padding([8, 14])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::toolbar_style);

    let stage = container(canvas::canvas_panel(
        slide,
        w.video.frame.as_ref(),
        selected,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([24, 24])
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color::from_rgb(0.52, 0.54, 0.58))),
        border: Border {
            color: theme::BORDER_STRONG,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    });

    let objects = object_strip(w);

    let col = column![top_bar, stage, objects]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn object_strip<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let editing = match w.editor.editing.as_ref() {
        Some(p) => p,
        None => return Space::new().height(0).into(),
    };
    let selected = w.editor.selected_slide_index;
    let slide = selected.and_then(|i| editing.slides.get(i));
    let objects = slide.map(|s| s.effective_layers()).unwrap_or_default();

    let mut row_el = Row::new().spacing(6).padding([8, 10]);
    row_el = row_el.push(
        button(text("+").size(14).color(theme::TEXT_PRIMARY))
            .on_press(Message::from(crate::ui::slides::Message::AddSlide))
            .padding([4, 12])
            .style(theme::secondary_button),
    );
    for (i, obj) in objects.iter().enumerate() {
        let is_sel = w.layer.selected_index == Some(i);
        let label = match &obj.content {
            crate::domain::ObjectContent::Text { text, .. } => {
                crate::ui::components::truncate(text, 16)
            }
            crate::domain::ObjectContent::Image { .. } => "Image".to_string(),
            crate::domain::ObjectContent::Video { .. } => "Video".to_string(),
            crate::domain::ObjectContent::Shape { shape_type, .. } => format!("{shape_type:?}"),
        };
        let b = button(text(label).size(11).color(if is_sel {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        }))
        .padding([6, 12])
        .style(move |_t: &iced::Theme, status| {
            let bg = if is_sel {
                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::BG_DARK
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: if is_sel {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::BORDER_PANEL
                    },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });
        row_el = row_el.push(b);
    }

    container(scrollable(row_el).height(Length::Fill))
        .width(Length::Fill)
        .style(theme::section_header_style)
        .into()
}

fn small_icon_btn(
    icon: &'static str,
    label: &'static str,
    _enabled: bool,
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
    button(content)
        .padding([4, 8])
        .style(theme::ghost_button)
        .into()
}

// Re-export so callers can reference the right-dock width from one place.
pub const RIGHT_DOCK_W: f32 = RIGHT_W;
