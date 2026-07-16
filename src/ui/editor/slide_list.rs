use crate::domain::{Background, ObjectContent, Presentation, Slide};
use crate::ui::components::group_color::group_option_color;
use crate::ui::components::truncate;
use crate::ui::messages::Message;
use crate::ui::theme;
use crate::ui::{layers, slides};
use iced::{
    Alignment, Color, Element, Length,
    widget::{Column, Id, Space, button, column, container, row, scrollable, text},
};
use std::sync::OnceLock;

pub const PANEL_WIDTH: f32 = 248.0;
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
            text(&presentation.name).size(14).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            button(text("+").size(14))
                .on_press(Message::from(slides::Message::AddSlide))
                .padding([1, 8])
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
        text(format!("> {active_group}"))
            .size(12)
            .color(Color::WHITE),
    )
    .width(Length::Fill)
    .padding([4, 8])
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(group_option_color(active_group))),
        ..Default::default()
    });

    let mut slides_col = Column::new().spacing(8).padding([8, 6]);
    if presentation.slides.is_empty() {
        slides_col = slides_col.push(
            container(text("No slides").size(12).color(theme::TEXT_MUTED))
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
                String::from("(Empty)")
            } else {
                truncate(text, 30)
            }
        }
        crate::domain::SlideContent::Image { .. } => String::from("Image"),
        crate::domain::SlideContent::Video { .. } => String::from("Video"),
    };

    let label = slide.group.as_deref().unwrap_or("Verse");
    let ribbon_color = group_option_color(label);

    let thumb = container(
        container(
            text(preview)
                .size(9)
                .color(theme::TEXT_PRIMARY)
                .width(Length::Fill),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .height(68),
    )
    .width(Length::Fill)
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            color: if selected {
                theme::ACCENT_BLUE
            } else {
                theme::BORDER_STRONG
            },
            width: if selected { 2.0 } else { 1.0 },
            radius: 2.0.into(),
        },
        ..Default::default()
    });

    let ribbon = container(
        row![
            text(format!("{}", index + 1)).size(10).color(Color::WHITE),
            Space::new().width(8),
            text(label).size(10).color(Color::WHITE),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([2, 6])
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(ribbon_color)),
        ..Default::default()
    });

    button(column![thumb, ribbon].spacing(0))
        .on_press(Message::from(slides::Message::SelectSlide(index)))
        .padding([2, 2])
        .width(Length::Fill)
        .style(
            move |_theme: &iced::Theme, status| iced::widget::button::Style {
                background: Some(iced::Background::Color(
                    if matches!(status, iced::widget::button::Status::Hovered) {
                        theme::BG_HOVER
                    } else {
                        theme::TRANSPARENT
                    },
                )),
                border: iced::Border {
                    color: theme::TRANSPARENT,
                    width: 0.0,
                    radius: 3.0.into(),
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
    let header = container(text("OBJECTS").size(10).color(theme::TEXT_MUTED))
        .width(Length::Fill)
        .padding([6, 8])
        .style(theme::section_header_style);

    let mut list = Column::new().spacing(2).padding([4, 6]);

    if let Some(slide) = slide {
        if slide.layers.is_empty() {
            list = list.push(
                button(
                    row![
                        text("T").size(14).color(theme::ACCENT_BLUE),
                        Space::new().width(8),
                        text("LYRICS").size(12).color(theme::TEXT_SECONDARY),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::from(layers::Message::SelectLayer(Some(0))))
                .padding([4, 8])
                .style(
                    move |_theme: &iced::Theme, status| iced::widget::button::Style {
                        background: Some(iced::Background::Color(
                            if selected_layer_index == Some(0) {
                                Color::from_rgba(0.204, 0.471, 0.965, 0.20)
                            } else if matches!(status, iced::widget::button::Status::Hovered) {
                                theme::BG_HOVER
                            } else {
                                theme::TRANSPARENT
                            },
                        )),
                        border: iced::Border {
                            color: if selected_layer_index == Some(0) {
                                theme::ACCENT_BLUE
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
            let icon = match &layer.content {
                ObjectContent::Text { .. } => "T",
                ObjectContent::Shape { .. } => "▭",
                ObjectContent::Image { .. } => "IMG",
                ObjectContent::Video { .. } => "VID",
            };
            list = list.push(
                button(
                    row![
                        text(icon).size(12).color(theme::TEXT_SECONDARY),
                        Space::new().width(8),
                        text(layer.display_name())
                            .size(12)
                            .color(theme::TEXT_SECONDARY),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::from(layers::Message::SelectLayer(Some(i))))
                .padding([4, 8])
                .style(
                    move |_theme: &iced::Theme, status| iced::widget::button::Style {
                        background: Some(iced::Background::Color(
                            if selected_layer_index == Some(i) {
                                Color::from_rgba(0.204, 0.471, 0.965, 0.20)
                            } else if matches!(status, iced::widget::button::Status::Hovered) {
                                theme::BG_HOVER
                            } else {
                                theme::TRANSPARENT
                            },
                        )),
                        border: iced::Border {
                            color: if selected_layer_index == Some(i) {
                                theme::ACCENT_BLUE
                            } else {
                                theme::TRANSPARENT
                            },
                            width: if selected_layer_index == Some(i) {
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
    } else {
        list = list.push(text("No objects").size(11).color(theme::TEXT_MUTED));
    }

    container(column![header, scrollable(list).height(Length::Fill)])
        .height(Length::FillPortion(2))
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}
