use crate::domain::{Background, LibraryAsset, Presentation, Slide, SlideContent, Transition};
use crate::ui::components::group_color::group_option_color;
use crate::ui::components::{search_input, truncate};
use crate::ui::messages::Message;
use crate::ui::presenter::TransitionState;
use crate::ui::presenter::canvas::{next_slide_canvas_panel, presenter_canvas_panel};
use crate::ui::theme;
use iced::{
    Alignment, Background as IcedBackground, Border, Color, Element, Length,
    widget::{
        Column, Row, Space, button, column, container, mouse_area, pin, row, scrollable, stack,
        text,
    },
};
use iced_font_awesome::fa_icon_solid;

const SHOW_LEFT_W: f32 = 250.0;
const SHOW_RIGHT_W: f32 = 320.0;
const SHOW_MEDIA_H: f32 = 220.0;
const SHOW_SLIDE_COLS: usize = 4;

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    presentations: &'a [Presentation],
    search_query: &'a str,
    active_id: Option<&'a str>,
    lib_assets: &'a [LibraryAsset],
    selected_asset_id: Option<&'a str>,
    presentation: Option<&'a Presentation>,
    slide_index: usize,
    output_open: bool,
    output_black_screen: bool,
    ndi_active: bool,
    recording_active: bool,
    output_is_fullscreen: bool,
    transition: Option<&'a TransitionState>,
    video_frame: Option<&'a iced::widget::image::Handle>,
    context_slide_index: Option<usize>,
    context_position: Option<iced::Point>,
    show_group_submenu: bool,
) -> Element<'a, Message> {
    let left = left_show_rail(
        presentations,
        search_query,
        active_id,
        presentation,
        slide_index,
        lib_assets,
    );

    let slides = slides_workspace(
        presentation,
        slide_index,
        context_slide_index,
        context_position,
        show_group_submenu,
        false,
        false,
        SHOW_SLIDE_COLS,
        crate::ui::messages::SlideViewMode::Grid,
    );
    let media_bin = media_bin_workspace(lib_assets, selected_asset_id);
    let center = column![slides, media_bin]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0);

    let right = show_right_dock(
        presentation,
        slide_index,
        transition,
        video_frame,
        output_open,
        output_black_screen,
        ndi_active,
        recording_active,
        output_is_fullscreen,
    );

    row![left, center, right]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn left_show_rail<'a>(
    presentations: &'a [Presentation],
    search_query: &'a str,
    active_id: Option<&'a str>,
    presentation: Option<&'a Presentation>,
    slide_index: usize,
    lib_assets: &'a [LibraryAsset],
) -> Element<'a, Message> {
    let mut library_list = Column::new().spacing(2).padding([4, 6]);
    let query = search_query.to_lowercase();
    for pres in presentations {
        if !query.is_empty() && !pres.name.to_lowercase().contains(&query) {
            continue;
        }
        let is_active = active_id.map(|id| id == pres.id).unwrap_or(false);
        library_list = library_list.push(
            button(
                row![
                    text(&pres.name).size(12).color(if is_active {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    }),
                    Space::new().width(Length::Fill),
                    text(format!("{}", pres.slides.len()))
                        .size(10)
                        .color(theme::TEXT_MUTED),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::OpenPresentation(pres.id.clone()))
            .padding([5, 8])
            .style(
                move |_t: &iced::Theme, status| iced::widget::button::Style {
                    background: Some(IcedBackground::Color(if is_active {
                        Color::from_rgba(0.204, 0.471, 0.965, 0.20)
                    } else if matches!(status, iced::widget::button::Status::Hovered) {
                        theme::BG_HOVER
                    } else {
                        theme::TRANSPARENT
                    })),
                    border: Border {
                        color: if is_active {
                            theme::ACCENT_BLUE
                        } else {
                            theme::TRANSPARENT
                        },
                        width: if is_active { 1.0 } else { 0.0 },
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                },
            ),
        );
    }
    if presentations.is_empty() {
        library_list =
            library_list.push(text("No presentations").size(11).color(theme::TEXT_MUTED));
    }

    let library_section = section_panel(
        "LIBRARY",
        column![
            search_input("Search", search_query, Message::SearchQueryChanged),
            scrollable(library_list).height(Length::Fill),
        ]
        .spacing(4),
    )
    .height(Length::FillPortion(3));

    let playlist_section = section_panel(
        "PLAYLIST",
        column![
            button(text("Default Playlist").size(12))
                .padding([6, 8])
                .style(theme::secondary_button),
            text("1 playlist").size(10).color(theme::TEXT_MUTED),
        ]
        .spacing(6),
    )
    .height(Length::FillPortion(1));

    let mut item_list = Column::new().spacing(2).padding([4, 6]);
    if let Some(pres) = presentation {
        for (i, slide) in pres.slides.iter().enumerate() {
            let label = match &slide.content {
                SlideContent::Text { text, .. } => {
                    if text.trim().is_empty() {
                        String::from("(empty)")
                    } else {
                        truncate(text, 28)
                    }
                }
                SlideContent::Image { .. } => String::from("Image"),
                SlideContent::Video { .. } => String::from("Video"),
            };
            let live = i == slide_index;
            item_list = item_list.push(
                button(
                    row![
                        text(format!("{}", i + 1)).size(10).color(if live {
                            theme::LIVE_GREEN
                        } else {
                            theme::TEXT_MUTED
                        }),
                        Space::new().width(8),
                        text(label).size(11).color(theme::TEXT_SECONDARY),
                    ]
                    .align_y(Alignment::Center),
                )
                .on_press(Message::PresentingSelectSlide(i))
                .padding([4, 6])
                .style(move |_t: &iced::Theme, status| {
                    iced::widget::button::Style {
                        background: Some(IcedBackground::Color(if live {
                            Color::from_rgba(0.204, 0.471, 0.965, 0.18)
                        } else if matches!(status, iced::widget::button::Status::Hovered) {
                            theme::BG_HOVER
                        } else {
                            theme::TRANSPARENT
                        })),
                        border: Border {
                            color: if live {
                                theme::ACCENT_BLUE
                            } else {
                                theme::TRANSPARENT
                            },
                            width: if live { 1.0 } else { 0.0 },
                            radius: 3.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            );
        }
    } else {
        item_list = item_list.push(
            text("Open a presentation to load show items")
                .size(11)
                .color(theme::TEXT_MUTED),
        );
    }

    let items_section = section_panel("ITEMS", scrollable(item_list).height(Length::Fill))
        .height(Length::FillPortion(4));

    let media_section = section_panel(
        "MEDIA",
        column![
            row![
                text(format!("{} assets", lib_assets.len()))
                    .size(10)
                    .color(theme::TEXT_MUTED),
                Space::new().width(Length::Fill),
                button(text("Import").size(10))
                    .on_press(Message::Library(crate::ui::library::Message::ImportAsset))
                    .padding([3, 8])
                    .style(theme::primary_button),
            ]
            .align_y(Alignment::Center),
            container(
                text("Use Media Bin below for browsing and preview")
                    .size(11)
                    .color(theme::TEXT_MUTED)
            )
            .padding([6, 4]),
        ]
        .spacing(8),
    )
    .height(Length::FillPortion(2));

    container(
        column![
            library_section,
            playlist_section,
            items_section,
            media_section
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .width(SHOW_LEFT_W)
    .height(Length::Fill)
    .style(theme::panel_style)
    .into()
}

fn section_panel<'a>(
    title: &'a str,
    body: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    container(
        column![
            container(text(title).size(10).color(theme::TEXT_MUTED))
                .padding([6, 8])
                .width(Length::Fill)
                .style(theme::section_header_style),
            container(body)
                .padding([4, 4])
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(0)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
}

#[allow(clippy::too_many_arguments)]
pub fn slides_workspace<'a>(
    presentation: Option<&'a Presentation>,
    slide_index: usize,
    context_slide_index: Option<usize>,
    context_position: Option<iced::Point>,
    show_group_submenu: bool,
    show_cue_submenu: bool,
    show_transition_submenu: bool,
    cols: usize,
    view_mode: crate::ui::messages::SlideViewMode,
) -> Element<'a, Message> {
    let list_content: Element<'a, Message> = if let Some(pres) = presentation {
        let total = pres.slides.len();
        if total == 0 {
            container(
                text("No slides in this presentation")
                    .size(13)
                    .color(theme::TEXT_MUTED),
            )
            .padding(20)
            .center_x(Length::Fill)
            .into()
        } else if view_mode == crate::ui::messages::SlideViewMode::Table {
            let thead = container(
                row![
                    container(text("#").size(10).color(theme::TEXT_MUTED))
                        .width(36)
                        .center_x(36),
                    container(text("GROUP").size(10).color(theme::TEXT_MUTED)).width(110),
                    container(text("CONTENT").size(10).color(theme::TEXT_MUTED))
                        .width(Length::Fill),
                    container(text("NOTES").size(10).color(theme::TEXT_MUTED)).width(140),
                    container(text("CUES").size(10).color(theme::TEXT_MUTED)).width(80),
                    container(text("STATUS").size(10).color(theme::TEXT_MUTED)).width(60),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .padding([6, 12]),
            )
            .style(theme::section_header_style);

            let mut table_col = Column::new().spacing(4).padding([6, 8]);
            for (i, slide) in pres.slides.iter().enumerate() {
                let is_live = slide_index == i;
                let is_next = slide_index + 1 == i;
                table_col = table_col.push(slide_table_row(slide, i, is_live, is_next));
            }
            column![thead, scrollable(table_col).height(Length::Fill)].into()
        } else {
            let num_cols = cols.clamp(2, 6);
            let mut grid_col = Column::new().spacing(8).padding([10, 10]);
            let mut index = 0usize;
            while index < total {
                let mut r = Row::new().spacing(8);
                for offset in 0..num_cols {
                    let i = index + offset;
                    if i < total {
                        let is_live = slide_index == i;
                        let is_next = slide_index + 1 == i;
                        r = r.push(slide_tile(&pres.slides[i], i, is_live, is_next));
                    } else {
                        r = r.push(Space::new().width(Length::FillPortion(1)));
                    }
                }
                grid_col = grid_col.push(r);
                index += num_cols;
            }
            scrollable(grid_col).height(Length::Fill).into()
        }
    } else {
        container(
            text("Select a presentation from the Library to load slides")
                .size(13)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    };

    let tracked_base = mouse_area(
        container(list_content)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_move(Message::ShowSlidesCursorMoved);

    let content: Element<'a, Message> =
        if let (Some(pres), Some(ctx_idx)) = (presentation, context_slide_index) {
            if let Some(ctx_slide) = pres.slides.get(ctx_idx) {
                let current_group = ctx_slide.group.as_deref();
                let current_trans = &ctx_slide.transition;
                let context = context_position.unwrap_or(iced::Point::new(24.0, 24.0));
                let slide_id = ctx_slide.id.clone();
                let backdrop: Element<'a, Message> = mouse_area(container(
                    Space::new().width(Length::Fill).height(Length::Fill),
                ))
                .on_press(Message::HideSlideContextMenu)
                .on_right_press(Message::HideSlideContextMenu)
                .into();

                let menu_overlay: Element<'a, Message> = pin(context_menu_panel(
                    ctx_idx,
                    &slide_id,
                    current_group,
                    current_trans,
                    show_group_submenu,
                    show_cue_submenu,
                    show_transition_submenu,
                ))
                .x(context.x + 2.0)
                .y(context.y + 2.0)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

                stack([tracked_base.into(), backdrop, menu_overlay])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                tracked_base.into()
            }
        } else {
            tracked_base.into()
        };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn slide_table_row<'a>(
    slide: &'a Slide,
    index: usize,
    live: bool,
    is_next: bool,
) -> Element<'a, Message> {
    let group = slide.group.as_deref().unwrap_or("Verse");
    let group_color = group_option_color(group);

    let idx_cell = container(text(format!("{}", index + 1)).size(12).color(if live {
        theme::LIVE_GREEN
    } else {
        theme::TEXT_MUTED
    }))
    .width(36)
    .center_x(36);

    let group_pill = container(text(group).size(10).color(Color::WHITE))
        .padding([2, 8])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(IcedBackground::Color(group_color)),
            border: Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let group_cell = container(group_pill).width(110);

    let preview_text = match &slide.content {
        SlideContent::Text { text, .. } if !text.trim().is_empty() => Some(text.as_str()),
        _ => slide.layers.iter().find_map(|l| match &l.content {
            crate::domain::ObjectContent::Text { text, .. } if !text.trim().is_empty() => {
                Some(text.as_str())
            }
            _ => None,
        }),
    };

    let text_cell: Element<'a, Message> = if let Some(txt) = preview_text {
        text(truncate(txt, 90))
            .size(12)
            .color(if live {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            })
            .width(Length::Fill)
            .into()
    } else {
        match &slide.background {
            Background::Image(_) => row![
                fa_icon_solid("image")
                    .size(11.0_f32)
                    .color(theme::ACCENT_BLUE),
                Space::new().width(6),
                text("Background Image").size(11).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center)
            .into(),
            Background::Video(_) => row![
                fa_icon_solid("video")
                    .size(11.0_f32)
                    .color(theme::WARNING_AMBER),
                Space::new().width(6),
                text("Background Video").size(11).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center)
            .into(),
            _ => text("(Empty Slide)")
                .size(11)
                .color(theme::TEXT_MUTED)
                .into(),
        }
    };

    let notes_cell: Element<'a, Message> =
        if let Some(notes) = slide.notes.as_deref().filter(|n| !n.trim().is_empty()) {
            row![
                fa_icon_solid("note-sticky")
                    .size(10.0_f32)
                    .color(theme::TEXT_MUTED),
                Space::new().width(4),
                text(truncate(notes, 20)).size(10).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center)
            .width(140)
            .into()
        } else {
            Space::new().width(140).into()
        };

    let cue_cell: Element<'a, Message> = if slide.cues.is_empty() {
        Space::new().width(80).into()
    } else {
        let mut cue_row = Row::new().spacing(3).align_y(Alignment::Center);
        for cue in &slide.cues {
            let icon_name = match &cue.action {
                crate::domain::Action::StartTimer
                | crate::domain::Action::StopTimer
                | crate::domain::Action::ResetTimer => "clock",
                crate::domain::Action::ClearSlide
                | crate::domain::Action::ClearMedia
                | crate::domain::Action::ClearProps
                | crate::domain::Action::ClearMessages
                | crate::domain::Action::ClearAudio
                | crate::domain::Action::ClearOutput => "ban",
                crate::domain::Action::ApplyLook(_) => "sliders",
                crate::domain::Action::TriggerProp(_) => "gauge",
                crate::domain::Action::ShowMessage(_) | crate::domain::Action::HideMessage => {
                    "bullhorn"
                }
                _ => "bolt",
            };
            cue_row = cue_row.push(
                container(fa_icon_solid(icon_name).size(8.0_f32).color(Color::WHITE))
                    .padding([1, 3])
                    .style(|_: &iced::Theme| iced::widget::container::Style {
                        background: Some(IcedBackground::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.5,
                        ))),
                        border: Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );
        }
        container(cue_row).width(80).into()
    };

    let status_cell: Element<'a, Message> = if live {
        container(
            row![
                container(Space::new().width(6).height(6)).style(|_: &iced::Theme| {
                    iced::widget::container::Style {
                        background: Some(IcedBackground::Color(theme::LIVE_GREEN)),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
                Space::new().width(4),
                text("LIVE").size(9).color(theme::LIVE_GREEN),
            ]
            .align_y(Alignment::Center),
        )
        .width(60)
        .into()
    } else if is_next {
        container(
            row![
                container(Space::new().width(6).height(6)).style(|_: &iced::Theme| {
                    iced::widget::container::Style {
                        background: Some(IcedBackground::Color(theme::ACCENT_ORANGE)),
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
                Space::new().width(4),
                text("NEXT").size(9).color(theme::ACCENT_ORANGE),
            ]
            .align_y(Alignment::Center),
        )
        .width(60)
        .into()
    } else {
        Space::new().width(60).into()
    };

    let row_content = row![
        idx_cell,
        group_cell,
        container(text_cell).width(Length::Fill),
        notes_cell,
        cue_cell,
        status_cell,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([5, 8]);

    let row_btn = button(row_content)
        .on_press(Message::PresentingSelectSlide(index))
        .width(Length::Fill)
        .style(move |_t: &iced::Theme, status| {
            let (bg, border_color, border_width) = if live {
                (
                    Color::from_rgba(0.204, 0.780, 0.349, 0.12),
                    theme::LIVE_GREEN,
                    1.5,
                )
            } else if is_next {
                (
                    Color::from_rgba(0.941, 0.216, 0.031, 0.08),
                    theme::ACCENT_ORANGE,
                    1.0,
                )
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                (theme::BG_HOVER, theme::BORDER_PANEL, 1.0)
            } else if index.is_multiple_of(2) {
                (
                    Color::from_rgba(1.0, 1.0, 1.0, 0.02),
                    theme::BORDER_STRONG,
                    1.0,
                )
            } else {
                (theme::TRANSPARENT, theme::BORDER_STRONG, 1.0)
            };
            iced::widget::button::Style {
                background: Some(IcedBackground::Color(bg)),
                border: Border {
                    color: border_color,
                    width: border_width,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        });

    mouse_area(row_btn)
        .on_right_press(Message::ShowSlideContextMenu(index))
        .into()
}

fn slide_tile<'a>(
    slide: &'a Slide,
    index: usize,
    live: bool,
    is_next: bool,
) -> Element<'a, Message> {
    let (bg, bg_media_icon) = match &slide.background {
        Background::Solid(c) => (Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0), None),
        Background::Image(_) => (Color::from_rgb(0.08, 0.12, 0.16), Some("image")),
        Background::Video(_) => (Color::from_rgb(0.12, 0.08, 0.16), Some("video")),
    };

    let preview = match &slide.content {
        SlideContent::Text { text, .. } if !text.trim().is_empty() => truncate(text, 44),
        SlideContent::Image { .. } => String::from("Image"),
        SlideContent::Video { .. } => String::from("Video"),
        _ => slide
            .layers
            .iter()
            .find_map(|l| match &l.content {
                crate::domain::ObjectContent::Text { text, .. } if !text.trim().is_empty() => {
                    Some(truncate(text, 44))
                }
                crate::domain::ObjectContent::Image { .. } => Some(String::from("Image")),
                crate::domain::ObjectContent::Video { .. } => Some(String::from("Video")),
                crate::domain::ObjectContent::Shape { shape_type, .. } => {
                    Some(format!("{:?}", shape_type))
                }
                _ => None,
            })
            .unwrap_or_else(|| String::from("(Empty)")),
    };

    let group = slide.group.as_deref().unwrap_or("Verse");
    let group_color = group_option_color(group);

    let (border_color, border_width) = if live {
        (theme::LIVE_GREEN, 2.5)
    } else if is_next {
        (theme::ACCENT_ORANGE, 2.0)
    } else {
        (theme::BORDER_STRONG, 1.0)
    };

    let media_badge: Element<'a, Message> = if let Some(icon) = bg_media_icon {
        container(fa_icon_solid(icon).size(10.0_f32).color(theme::TEXT_MUTED))
            .padding([2, 4])
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::new().width(0).into()
    };

    let thumb_content = column![
        row![media_badge, Space::new().width(Length::Fill)].align_y(Alignment::Center),
        text(preview)
            .size(10)
            .color(theme::TEXT_PRIMARY)
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill);

    let thumb = container(thumb_content)
        .padding([6, 8])
        .width(Length::Fill)
        .height(92)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(IcedBackground::Color(bg)),
            border: Border {
                color: border_color,
                width: border_width,
                radius: 2.0.into(),
            },
            ..Default::default()
        });

    let cue_badges: Element<'a, Message> = if slide.cues.is_empty() {
        Space::new().width(0).into()
    } else {
        let mut cue_row = Row::new().spacing(3).align_y(Alignment::Center);
        for cue in &slide.cues {
            let icon_name = match &cue.action {
                crate::domain::Action::StartTimer
                | crate::domain::Action::StopTimer
                | crate::domain::Action::ResetTimer => "clock",
                crate::domain::Action::ClearSlide
                | crate::domain::Action::ClearMedia
                | crate::domain::Action::ClearProps
                | crate::domain::Action::ClearMessages
                | crate::domain::Action::ClearAudio
                | crate::domain::Action::ClearOutput => "ban",
                crate::domain::Action::ApplyLook(_) => "sliders",
                crate::domain::Action::TriggerProp(_) => "gauge",
                crate::domain::Action::ShowMessage(_) | crate::domain::Action::HideMessage => {
                    "bullhorn"
                }
                _ => "bolt",
            };
            cue_row = cue_row.push(
                container(fa_icon_solid(icon_name).size(8.0_f32).color(Color::WHITE))
                    .padding([1, 3])
                    .style(|_: &iced::Theme| iced::widget::container::Style {
                        background: Some(IcedBackground::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.5,
                        ))),
                        border: Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            );
        }
        cue_row.into()
    };

    let next_pill: Element<'a, Message> = if is_next && !live {
        container(text("NEXT").size(8).color(Color::BLACK))
            .padding([1, 4])
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(theme::ACCENT_ORANGE)),
                border: Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    } else {
        Space::new().width(0).into()
    };

    let ribbon = container(
        row![
            text(format!("{}", index + 1)).size(11).color(Color::WHITE),
            Space::new().width(6),
            text(group).size(11).color(Color::WHITE),
            next_pill,
            Space::new().width(Length::Fill),
            cue_badges,
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(IcedBackground::Color(group_color)),
        ..Default::default()
    });

    mouse_area(
        button(column![thumb, ribbon].spacing(0).width(Length::Fill))
            .on_press(Message::PresentingSelectSlide(index))
            .padding([4, 4])
            .width(Length::FillPortion(1))
            .style(
                move |_theme: &iced::Theme, status| iced::widget::button::Style {
                    background: Some(IcedBackground::Color(
                        if matches!(status, iced::widget::button::Status::Hovered) {
                            theme::BG_HOVER
                        } else {
                            theme::TRANSPARENT
                        },
                    )),
                    border: Border {
                        color: theme::TRANSPARENT,
                        width: 0.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                },
            ),
    )
    .on_right_press(Message::ShowSlideContextMenu(index))
    .into()
}

fn context_menu_panel<'a>(
    index: usize,
    slide_id: &str,
    current_group: Option<&str>,
    current_transition: &Transition,
    show_group_submenu: bool,
    show_cue_submenu: bool,
    show_transition_submenu: bool,
) -> Element<'a, Message> {
    const GROUPS: &[&str] = &[
        "Verse",
        "Verse 1",
        "Verse 2",
        "Verse 3",
        "Verse 4",
        "Verse 5",
        "Verse 6",
        "Chorus",
        "Chorus 1",
        "Chorus 2",
        "Chorus 3",
        "Chorus 4",
        "Bridge",
        "Bridge 1",
        "Bridge 2",
        "Bridge 3",
        "PreChorus",
        "Tag",
        "Intro",
        "Ending",
        "Outro",
        "Interlude",
        "Vamp",
        "Turnaround",
        "Blank",
    ];

    let menu_bg = Color::from_rgba(0.11, 0.12, 0.14, 0.98);
    let menu_shadow = iced::Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        offset: iced::Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    };
    let current = current_group.unwrap_or_default().to_lowercase();

    /// Styled context menu item.
    fn menu_item<'a>(
        content: impl Into<Element<'a, Message>>,
        on_press: Message,
        selected: bool,
    ) -> Element<'a, Message> {
        button(content)
            .on_press(on_press)
            .width(Length::Fill)
            .padding([7, 10])
            .style(
                move |_t: &iced::Theme, status| iced::widget::button::Style {
                    background: Some(IcedBackground::Color(if selected {
                        Color::from_rgba(0.204, 0.471, 0.965, 0.22)
                    } else if matches!(status, iced::widget::button::Status::Hovered) {
                        theme::BG_HOVER
                    } else {
                        theme::TRANSPARENT
                    })),
                    border: Border {
                        color: if selected {
                            theme::ACCENT_BLUE
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

    /// Thin horizontal separator between menu sections.
    fn menu_divider<'a>() -> Element<'a, Message> {
        container(Space::new().height(1).width(Length::Fill))
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(theme::BORDER_PANEL)),
                ..Default::default()
            })
            .padding([4, 0])
            .into()
    }

    let edit_slide = menu_item(
        row![
            fa_icon_solid("pen-to-square")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Edit Slide").size(13).color(theme::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
        Message::EditSlide(index),
        false,
    );

    let add_after = menu_item(
        row![
            fa_icon_solid("plus")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Add Slide After").size(13).color(theme::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
        Message::from(crate::ui::slides::Message::AddSlideAfter(index)),
        false,
    );
    let duplicate = menu_item(
        row![
            fa_icon_solid("copy")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Duplicate").size(13).color(theme::TEXT_PRIMARY),
        ]
        .align_y(Alignment::Center),
        Message::from(crate::ui::slides::Message::DuplicateSlide(index)),
        false,
    );
    let delete = menu_item(
        row![
            fa_icon_solid("trash")
                .size(11.0_f32)
                .color(theme::DANGER_RED),
            Space::new().width(8),
            text("Delete").size(13).color(theme::DANGER_RED),
        ]
        .align_y(Alignment::Center),
        Message::from(crate::ui::slides::Message::DeleteSlide(
            slide_id.to_string(),
        )),
        false,
    );

    let group_trigger = menu_item(
        row![
            fa_icon_solid("layer-group")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Group").size(13).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            fa_icon_solid("chevron-right")
                .size(10.0_f32)
                .color(theme::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
        Message::ShowSlideGroupSubmenu,
        show_group_submenu,
    );

    let cue_trigger = menu_item(
        row![
            fa_icon_solid("bolt")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Add Action").size(13).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            fa_icon_solid("chevron-right")
                .size(10.0_f32)
                .color(theme::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
        Message::ShowSlideCueSubmenu,
        show_cue_submenu,
    );

    let transition_trigger = menu_item(
        row![
            fa_icon_solid("wand-magic-sparkles")
                .size(11.0_f32)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(8),
            text("Transition").size(13).color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            fa_icon_solid("chevron-right")
                .size(10.0_f32)
                .color(theme::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
        Message::ShowSlideTransitionSubmenu,
        show_transition_submenu,
    );

    let clear_cues = menu_item(
        row![
            fa_icon_solid("ban").size(11.0_f32).color(theme::TEXT_MUTED),
            Space::new().width(8),
            text("Clear All Cues").size(13).color(theme::TEXT_MUTED),
        ]
        .align_y(Alignment::Center),
        Message::ClearSlideCues(index),
        false,
    );

    let main_items = column![
        edit_slide,
        menu_divider(),
        add_after,
        duplicate,
        delete,
        menu_divider(),
        group_trigger,
        cue_trigger,
        transition_trigger,
        menu_divider(),
        clear_cues,
    ]
    .spacing(2);

    let main_panel: Element<'a, Message> = container(main_items)
        .width(184)
        .padding([6, 6])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(IcedBackground::Color(menu_bg)),
            border: Border {
                color: theme::BORDER_STRONG,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: menu_shadow,
            ..Default::default()
        })
        .into();

    if show_group_submenu {
        let mut options = Column::new().spacing(2);
        options = options.push(menu_item(
            text("Ungroup").size(13).color(theme::TEXT_PRIMARY),
            Message::from(crate::ui::slides::Message::SetSlideGroupLabel(
                index,
                String::new(),
            )),
            current.is_empty(),
        ));
        options = options.push(menu_divider());
        for label in GROUPS {
            let color = group_option_color(label);
            let selected = current == label.to_lowercase();

            let swatch =
                container(Space::new().width(14).height(14)).style(move |_: &iced::Theme| {
                    iced::widget::container::Style {
                        background: Some(IcedBackground::Color(color)),
                        border: Border {
                            color: Color::WHITE,
                            width: 1.0,
                            radius: 2.0.into(),
                        },
                        ..Default::default()
                    }
                });

            options = options.push(menu_item(
                row![
                    swatch,
                    Space::new().width(8),
                    text(*label).size(13).color(theme::TEXT_PRIMARY),
                ]
                .align_y(Alignment::Center),
                Message::from(crate::ui::slides::Message::SetSlideGroupLabel(
                    index,
                    (*label).to_string(),
                )),
                selected,
            ));
        }

        let group_panel: Element<'a, Message> = container(scrollable(options).height(420))
            .width(210)
            .padding([6, 6])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(menu_bg)),
                border: Border {
                    color: theme::BORDER_STRONG,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: menu_shadow,
                ..Default::default()
            })
            .into();

        return row![main_panel, Space::new().width(6), group_panel]
            .align_y(Alignment::Start)
            .into();
    }

    if show_cue_submenu {
        let cues_list: &[(&str, &str, crate::domain::Action)] = &[
            (
                "Clear All",
                "circle-xmark",
                crate::domain::Action::ClearOutput,
            ),
            ("Clear Slide", "ban", crate::domain::Action::ClearSlide),
            (
                "Clear Media",
                "photo-film",
                crate::domain::Action::ClearMedia,
            ),
            ("Clear Props", "gauge", crate::domain::Action::ClearProps),
            (
                "Clear Messages",
                "bullhorn",
                crate::domain::Action::ClearMessages,
            ),
            (
                "Clear Audio",
                "volume-xmark",
                crate::domain::Action::ClearAudio,
            ),
            ("Start Timer", "play", crate::domain::Action::StartTimer),
            ("Stop Timer", "pause", crate::domain::Action::StopTimer),
            (
                "Reset Timer",
                "rotate-left",
                crate::domain::Action::ResetTimer,
            ),
            (
                "Default Look",
                "sliders",
                crate::domain::Action::ApplyLook("Default".to_string()),
            ),
        ];

        let mut cue_items = Column::new().spacing(2);
        for (name, icon, action) in cues_list {
            cue_items = cue_items.push(menu_item(
                row![
                    fa_icon_solid(icon)
                        .size(11.0_f32)
                        .color(theme::TEXT_PRIMARY),
                    Space::new().width(8),
                    text(*name).size(12).color(theme::TEXT_PRIMARY),
                ]
                .align_y(Alignment::Center),
                Message::AddSlideCue(index, crate::domain::Cue::new(*name, action.clone())),
                false,
            ));
        }

        let cue_panel: Element<'a, Message> = container(cue_items)
            .width(170)
            .padding([6, 6])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(menu_bg)),
                border: Border {
                    color: theme::BORDER_STRONG,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: menu_shadow,
                ..Default::default()
            })
            .into();

        return row![main_panel, Space::new().width(6), cue_panel]
            .align_y(Alignment::Start)
            .into();
    }

    if show_transition_submenu {
        let is_cut = matches!(current_transition, Transition::Cut);
        let is_fade_300 = matches!(current_transition, Transition::Fade { duration_ms: 300 });
        let is_fade_500 = matches!(current_transition, Transition::Fade { duration_ms: 500 });
        let is_fade_1000 = matches!(current_transition, Transition::Fade { duration_ms: 1000 });
        let is_fade_2000 = matches!(current_transition, Transition::Fade { duration_ms: 2000 });

        let transitions: &[(&str, Transition, bool)] = &[
            ("Cut", Transition::Cut, is_cut),
            (
                "0.3s Dissolve",
                Transition::Fade { duration_ms: 300 },
                is_fade_300,
            ),
            (
                "0.5s Dissolve",
                Transition::Fade { duration_ms: 500 },
                is_fade_500,
            ),
            (
                "1.0s Dissolve",
                Transition::Fade { duration_ms: 1000 },
                is_fade_1000,
            ),
            (
                "2.0s Dissolve",
                Transition::Fade { duration_ms: 2000 },
                is_fade_2000,
            ),
        ];

        let mut trans_items = Column::new().spacing(2);
        for (name, trans, selected) in transitions {
            trans_items = trans_items.push(menu_item(
                row![
                    fa_icon_solid(if *selected {
                        "check"
                    } else {
                        "wand-magic-sparkles"
                    })
                    .size(11.0_f32)
                    .color(if *selected {
                        theme::ACCENT_BLUE
                    } else {
                        theme::TEXT_MUTED
                    }),
                    Space::new().width(8),
                    text(*name).size(12).color(theme::TEXT_PRIMARY),
                ]
                .align_y(Alignment::Center),
                Message::SetSlideTransition(index, *trans),
                *selected,
            ));
        }

        let trans_panel: Element<'a, Message> = container(trans_items)
            .width(170)
            .padding([6, 6])
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(menu_bg)),
                border: Border {
                    color: theme::BORDER_STRONG,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: menu_shadow,
                ..Default::default()
            })
            .into();

        return row![main_panel, Space::new().width(6), trans_panel]
            .align_y(Alignment::Start)
            .into();
    }

    main_panel
}

pub fn media_bin_workspace<'a>(
    assets: &'a [LibraryAsset],
    selected_asset_id: Option<&'a str>,
) -> Element<'a, Message> {
    let header = container(
        row![
            text("MEDIA BIN").size(10).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
            button(text("Import").size(10))
                .on_press(Message::Library(crate::ui::library::Message::ImportAsset))
                .padding([3, 8])
                .style(theme::primary_button),
        ]
        .align_y(Alignment::Center)
        .padding([8, 12])
        .spacing(4),
    )
    .width(Length::Fill)
    .style(theme::section_header_style);

    let mut items = Row::new().spacing(8).padding([8, 10]);
    if assets.is_empty() {
        items = items.push(
            container(text("No media items").size(12).color(theme::TEXT_MUTED))
                .padding([8, 8])
                .width(Length::Fill),
        );
    } else {
        for asset in assets.iter().take(24) {
            let selected = selected_asset_id.map(|id| id == asset.id).unwrap_or(false);
            items = items.push(media_chip(asset, selected));
        }
    }

    container(column![
        header,
        scrollable(items).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
        ))
    ])
    .width(Length::Fill)
    .height(SHOW_MEDIA_H)
    .style(theme::panel_style)
    .into()
}

fn media_chip<'a>(asset: &'a LibraryAsset, selected: bool) -> Element<'a, Message> {
    let label = truncate(&asset.name, 20);
    let tag = if asset.is_image() { "IMG" } else { "VID" };
    let tag_color = if asset.is_image() {
        theme::ACCENT_BLUE
    } else {
        theme::WARNING_AMBER
    };
    button(
        column![
            container(text(tag).size(10).color(tag_color))
                .padding([2, 6])
                .style(move |_: &iced::Theme| iced::widget::container::Style {
                    background: Some(IcedBackground::Color(Color {
                        a: 0.14,
                        ..tag_color
                    })),
                    border: Border {
                        color: tag_color,
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }),
            text(label).size(11).color(theme::TEXT_PRIMARY),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .width(160),
    )
    .on_press(Message::Library(crate::ui::library::Message::SelectAsset(
        asset.id.clone(),
    )))
    .padding([10, 10])
    .style(move |_theme: &iced::Theme, status| {
        let bg = if selected {
            Color::from_rgba(0.204, 0.471, 0.965, 0.20)
        } else if matches!(status, iced::widget::button::Status::Hovered) {
            theme::BG_HOVER
        } else {
            theme::BG_DARK
        };
        iced::widget::button::Style {
            background: Some(IcedBackground::Color(bg)),
            border: Border {
                color: if selected {
                    theme::ACCENT_BLUE
                } else {
                    theme::BORDER_PANEL
                },
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

#[allow(clippy::too_many_arguments)]
fn show_right_dock<'a>(
    presentation: Option<&'a Presentation>,
    slide_index: usize,
    transition: Option<&'a TransitionState>,
    video_frame: Option<&'a iced::widget::image::Handle>,
    output_open: bool,
    output_black_screen: bool,
    ndi_active: bool,
    recording_active: bool,
    output_is_fullscreen: bool,
) -> Element<'a, Message> {
    let current = presentation.and_then(|p| p.slides.get(slide_index));
    let next = presentation.and_then(|p| p.slides.get(slide_index + 1));
    let (from_slide, trans, progress) = match transition {
        Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
        None => (None, Transition::Cut, 1.0),
    };

    let live_body: Element<'a, Message> = if output_black_screen {
        container(text("BLACK").size(14).color(theme::TEXT_MUTED))
            .width(Length::Fill)
            .height(170)
            .center(Length::Fill)
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(IcedBackground::Color(Color::BLACK)),
                ..Default::default()
            })
            .into()
    } else {
        presenter_canvas_panel(current, from_slide, trans, progress, video_frame)
    };

    let live_panel = preview_block("LIVE", live_body);
    let next_panel = preview_block("NEXT", next_slide_canvas_panel(next));

    let prev_btn = {
        let b = button(text("Prev").size(11))
            .padding([5, 10])
            .style(theme::ghost_button);
        if presentation.is_some() {
            b.on_press(Message::PresentingPrevSlide)
        } else {
            b
        }
    };
    let next_btn = {
        let b = button(text("Next").size(11))
            .padding([5, 10])
            .style(theme::primary_button);
        if presentation.is_some() {
            b.on_press(Message::PresentingNextSlide)
        } else {
            b
        }
    };

    let controls = container(
        column![
            row![
                button(text("Clear").size(11))
                    .on_press(Message::ClearOutput)
                    .padding([5, 10])
                    .style(theme::secondary_button),
                button(
                    text(if output_black_screen {
                        "Unblack"
                    } else {
                        "Black"
                    })
                    .size(11)
                )
                .on_press(Message::ToggleOutputBlackScreen)
                .padding([5, 10])
                .style(theme::ghost_button),
                button(text(if ndi_active { "NDI On" } else { "NDI Off" }).size(11))
                    .on_press(Message::Ndi(crate::ui::ndi::Message::Toggle))
                    .padding([5, 10])
                    .style(if ndi_active {
                        theme::primary_button
                    } else {
                        theme::ghost_button
                    }),
            ]
            .spacing(6),
            row![
                button(
                    text(if output_open {
                        "Close Screen"
                    } else {
                        "Open Screen"
                    })
                    .size(11)
                )
                .on_press(if output_open {
                    Message::CloseOutputWindow
                } else {
                    Message::OpenOutputWindow
                })
                .padding([5, 10])
                .style(theme::secondary_button),
                button(
                    text(if output_is_fullscreen {
                        "Windowed"
                    } else {
                        "Fullscreen"
                    })
                    .size(11)
                )
                .on_press(Message::OutputFullscreenToggled)
                .padding([5, 10])
                .style(theme::ghost_button),
            ]
            .spacing(6),
            row![
                prev_btn,
                next_btn,
                button(
                    text(if recording_active {
                        "Stop Rec"
                    } else {
                        "Record"
                    })
                    .size(11)
                )
                .on_press(if recording_active {
                    Message::Recording(crate::ui::recording::Message::Stop)
                } else {
                    Message::Recording(crate::ui::recording::Message::Start)
                })
                .padding([5, 10])
                .style(if recording_active {
                    theme::danger_button
                } else {
                    theme::ghost_button
                }),
            ]
            .spacing(6),
            row![
                button(text("Props").size(10))
                    .on_press(Message::TogglePropsPanel)
                    .padding([4, 8])
                    .style(theme::ghost_button),
                button(text("Triggers").size(10))
                    .on_press(Message::ToggleTriggersPanel)
                    .padding([4, 8])
                    .style(theme::ghost_button),
                button(text("Stage").size(10))
                    .on_press(Message::ToggleStageDisplay)
                    .padding([4, 8])
                    .style(theme::ghost_button),
            ]
            .spacing(4),
        ]
        .spacing(8)
        .padding([10, 10]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style);

    container(
        column![live_panel, next_panel, controls]
            .spacing(8)
            .padding([8, 8]),
    )
    .width(SHOW_RIGHT_W)
    .height(Length::Fill)
    .style(theme::panel_style)
    .into()
}

fn preview_block<'a>(label: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![
            container(text(label).size(10).color(theme::TEXT_MUTED))
                .padding([6, 8])
                .width(Length::Fill)
                .style(theme::section_header_style),
            container(body).height(170).width(Length::Fill),
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}
