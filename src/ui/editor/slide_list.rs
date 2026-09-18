use crate::domain::{Background, ObjectContent, Presentation, ShapeType, Slide};
use crate::ui::components::group_color::group_option_color;
use crate::ui::components::truncate;
use crate::ui::messages::Message;
use crate::ui::theme;
use crate::ui::{layers, slides};
use iced::{
    Alignment, Color, Element, Length,
    widget::{Column, Id, Space, button, column, container, row, scrollable, text},
};
use iced_font_awesome::fa_icon_solid;
use std::sync::OnceLock;

pub const PANEL_WIDTH: f32 = 260.0;
static SLIDE_LIST_SCROLL: OnceLock<Id> = OnceLock::new();

pub fn scrollable_id() -> Id {
    SLIDE_LIST_SCROLL.get_or_init(Id::unique).clone()
}

pub fn slide_list<'a>(
    presentation: &'a Presentation,
    selected_index: Option<usize>,
    selected_layer_index: Option<usize>,
) -> Element<'a, Message> {
    let header = container(
        row![
            text(&presentation.name).size(13).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            button(
                row![
                    fa_icon_solid("plus").size(10.0_f32),
                    text("Slide").size(11).color(theme::TEXT_PRIMARY),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .on_press(Message::from(slides::Message::AddSlide))
            .padding([3, 8])
            .style(theme::secondary_button),
        ]
        .align_y(Alignment::Center)
        .padding([8, 10]),
    )
    .width(Length::Fill)
    .style(theme::section_header_style);

    let active_group = selected_index
        .and_then(|i| presentation.slides.get(i))
        .and_then(|s| s.group.as_deref())
        .unwrap_or("Verse");

    let group_strip = container(
        row![
            container(Space::new().width(4).height(12)).style(move |_: &iced::Theme| {
                iced::widget::container::Style {
                    background: Some(iced::Background::Color(group_option_color(active_group))),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
            Space::new().width(4),
            text(format!("Group: {active_group}"))
                .size(11)
                .color(theme::TEXT_SECONDARY),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([3, 10])
    .style(theme::dark_panel_style);

    let mut slides_col = Column::new().spacing(8).padding([8, 6]);
    if presentation.slides.is_empty() {
        slides_col = slides_col.push(
            container(
                text("No slides in presentation")
                    .size(11)
                    .color(theme::TEXT_MUTED),
            )
            .padding([12, 8])
            .width(Length::Fill),
        );
    } else {
        for (i, slide) in presentation.slides.iter().enumerate() {
            slides_col = slides_col.push(slide_card(slide, i, selected_index == Some(i)));
        }
    }

    let selected_slide = selected_index.and_then(|i| presentation.slides.get(i));
    let objects_panel = objects_panel(selected_slide, selected_layer_index);

    container(
        column![
            header,
            group_strip,
            scrollable(slides_col)
                .id(scrollable_id())
                .height(Length::FillPortion(3)),
            objects_panel,
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .width(PANEL_WIDTH)
    .height(Length::Fill)
    .style(theme::panel_style)
    .into()
}

fn slide_card<'a>(slide: &'a Slide, index: usize, selected: bool) -> Element<'a, Message> {
    let bg = match &slide.background {
        Background::Solid(c) => Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0),
        _ => Color::BLACK,
    };

    let preview = match &slide.content {
        crate::domain::SlideContent::Text { text, .. } => {
            if text.trim().is_empty() {
                String::from("(Empty text)")
            } else {
                truncate(text, 28)
            }
        }
        crate::domain::SlideContent::Image { .. } => String::from("Image Slide"),
        crate::domain::SlideContent::Video { .. } => String::from("Video Slide"),
    };

    let label = slide.group.as_deref().unwrap_or("Verse");
    let ribbon_color = group_option_color(label);

    let thumb = container(
        container(
            text(preview)
                .size(10)
                .color(theme::TEXT_PRIMARY)
                .width(Length::Fill),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .height(60),
    )
    .width(Length::Fill)
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            color: if selected {
                theme::ACCENT_ORANGE
            } else {
                theme::BORDER_PANEL
            },
            width: if selected { 2.0 } else { 1.0 },
            radius: 4.0.into(),
        },
        ..Default::default()
    });

    let ribbon = row![
        container(text(format!("{}", index + 1)).size(9).color(Color::WHITE),)
            .padding([1, 5])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(ribbon_color)),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        Space::new().width(4),
        text(label).size(10).color(theme::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let mut card_col = column![thumb, ribbon].spacing(2);

    if selected {
        let actions = row![
            button(
                fa_icon_solid("arrow-up")
                    .size(9.0_f32)
                    .color(theme::TEXT_MUTED)
            )
            .on_press(Message::from(slides::Message::MoveSlideUp(index)))
            .padding([2, 5])
            .style(theme::ghost_button),
            button(
                fa_icon_solid("arrow-down")
                    .size(9.0_f32)
                    .color(theme::TEXT_MUTED)
            )
            .on_press(Message::from(slides::Message::MoveSlideDown(index)))
            .padding([2, 5])
            .style(theme::ghost_button),
            Space::new().width(Length::Fill),
            button(fa_icon_solid("copy").size(9.0_f32).color(theme::TEXT_MUTED))
                .on_press(Message::from(slides::Message::DuplicateSlide(index)))
                .padding([2, 5])
                .style(theme::ghost_button),
            button(
                fa_icon_solid("trash")
                    .size(9.0_f32)
                    .color(theme::DANGER_RED)
            )
            .on_press(Message::from(slides::Message::DeleteSlide(
                slide.id.clone()
            )))
            .padding([2, 5])
            .style(theme::ghost_button),
        ]
        .spacing(2)
        .align_y(Alignment::Center)
        .padding([2, 4]);

        card_col = card_col.push(actions);
    }

    button(card_col)
        .on_press(Message::from(slides::Message::SelectSlide(index)))
        .padding([4, 4])
        .width(Length::Fill)
        .style(
            move |_theme: &iced::Theme, status| iced::widget::button::Style {
                background: Some(iced::Background::Color(if selected {
                    Color::from_rgba(0.941, 0.216, 0.031, 0.10)
                } else if matches!(status, iced::widget::button::Status::Hovered) {
                    theme::BG_HOVER
                } else {
                    theme::TRANSPARENT
                })),
                border: iced::Border {
                    color: if selected {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TRANSPARENT
                    },
                    width: if selected { 1.0 } else { 0.0 },
                    radius: 4.0.into(),
                },
                ..Default::default()
            },
        )
        .into()
}

fn objects_panel<'a>(
    slide: Option<&'a Slide>,
    selected_layer_index: Option<usize>,
) -> Element<'a, Message> {
    let header = container(
        row![
            text("LAYERS").size(10).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
            button(
                row![
                    fa_icon_solid("plus").size(8.0_f32),
                    text("Text").size(9).color(theme::TEXT_SECONDARY),
                ]
                .spacing(2)
                .align_y(Alignment::Center),
            )
            .on_press(Message::from(layers::Message::AddTextLayer))
            .padding([2, 4])
            .style(theme::secondary_button),
            button(
                row![
                    fa_icon_solid("plus").size(8.0_f32),
                    text("Shape").size(9).color(theme::TEXT_SECONDARY),
                ]
                .spacing(2)
                .align_y(Alignment::Center),
            )
            .on_press(Message::from(layers::Message::AddShapeLayer(
                ShapeType::Rectangle,
            )))
            .padding([2, 4])
            .style(theme::secondary_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([6, 8])
    .style(theme::section_header_style);

    let mut list = Column::new().spacing(2).padding([4, 6]);

    if let Some(slide) = slide {
        if slide.layers.is_empty() {
            list = list.push(
                button(
                    row![
                        fa_icon_solid("font")
                            .size(11.0_f32)
                            .color(theme::ACCENT_BLUE),
                        Space::new().width(6),
                        text("Slide Text").size(11).color(theme::TEXT_SECONDARY),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::from(layers::Message::SelectLayer(Some(0))))
                .padding([4, 8])
                .width(Length::Fill)
                .style(
                    move |_theme: &iced::Theme, status| iced::widget::button::Style {
                        background: Some(iced::Background::Color(
                            if selected_layer_index == Some(0) {
                                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
                            } else if matches!(status, iced::widget::button::Status::Hovered) {
                                theme::BG_HOVER
                            } else {
                                theme::TRANSPARENT
                            },
                        )),
                        border: iced::Border {
                            color: if selected_layer_index == Some(0) {
                                theme::ACCENT_ORANGE
                            } else {
                                theme::TRANSPARENT
                            },
                            width: if selected_layer_index == Some(0) {
                                1.0
                            } else {
                                0.0
                            },
                            radius: 3.0.into(),
                        },
                        ..Default::default()
                    },
                ),
            );
        }

        for (i, layer) in slide.layers.iter().enumerate() {
            let is_sel = selected_layer_index == Some(i);
            let icon_name = match &layer.content {
                ObjectContent::Text { .. } => "font",
                ObjectContent::Shape { .. } => "square",
                ObjectContent::Image { .. } => "image",
                ObjectContent::Video { .. } => "video",
            };

            let eye_btn = button(
                fa_icon_solid(if layer.visible { "eye" } else { "eye-slash" })
                    .size(9.0_f32)
                    .color(if layer.visible {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_MUTED
                    }),
            )
            .on_press(Message::from(layers::Message::ToggleLayerVisibility(i)))
            .padding([2, 4])
            .style(theme::ghost_button);

            let lock_btn = button(
                fa_icon_solid(if layer.locked { "lock" } else { "lock-open" })
                    .size(9.0_f32)
                    .color(if layer.locked {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TEXT_MUTED
                    }),
            )
            .on_press(Message::from(layers::Message::ToggleLayerLock(i)))
            .padding([2, 4])
            .style(theme::ghost_button);

            let del_btn = button(
                fa_icon_solid("xmark")
                    .size(9.0_f32)
                    .color(theme::TEXT_MUTED),
            )
            .on_press(Message::from(layers::Message::DeleteLayer(i)))
            .padding([2, 4])
            .style(theme::ghost_button);

            let layer_btn = button(
                row![
                    fa_icon_solid(icon_name).size(10.0_f32).color(if is_sel {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TEXT_MUTED
                    }),
                    Space::new().width(4),
                    text(layer.display_name()).size(11).color(if is_sel {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    }),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::from(layers::Message::SelectLayer(Some(i))))
            .padding([3, 6])
            .width(Length::Fill)
            .style(
                move |_theme: &iced::Theme, status| iced::widget::button::Style {
                    background: Some(iced::Background::Color(if is_sel {
                        Color::from_rgba(0.941, 0.216, 0.031, 0.16)
                    } else if matches!(status, iced::widget::button::Status::Hovered) {
                        theme::BG_HOVER
                    } else {
                        theme::TRANSPARENT
                    })),
                    border: iced::Border {
                        color: if is_sel {
                            theme::ACCENT_ORANGE
                        } else {
                            theme::TRANSPARENT
                        },
                        width: if is_sel { 1.0 } else { 0.0 },
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                },
            );

            list = list.push(
                row![eye_btn, lock_btn, layer_btn, del_btn]
                    .spacing(2)
                    .align_y(Alignment::Center),
            );
        }
    } else {
        list = list.push(text("No objects").size(11).color(theme::TEXT_MUTED));
    }

    container(column![header, scrollable(list).height(Length::Fill)])
        .height(Length::FillPortion(2))
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}
