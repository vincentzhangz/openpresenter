pub mod canvas;

use crate::slides::{Presentation, Slide, Transition};
use crate::ui::components::{group_label_widget, live_badge as live_badge_widget, truncate};
use crate::ui::editor::canvas::bg_to_color;
use crate::ui::messages::Message;
use crate::ui::presenter::canvas::{next_slide_canvas_panel, presenter_canvas_panel};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{
        Column, Space, button, checkbox, column, container, row, scrollable, text, text_input,
    },
};
use iced_font_awesome::fa_icon_solid;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TransitionState {
    pub from_slide: Slide,
    pub transition: Transition,
    pub progress: f32,
    pub start: Instant,
}

#[allow(clippy::too_many_arguments)]
pub fn presenting_view<'a>(
    presentation: &'a Presentation,
    slide_index: usize,
    ndi_active: bool,
    output_open: bool,
    output_black_screen: bool,
    output_is_fullscreen: bool,
    show_output_settings: bool,
    output_screen_x: &'a str,
    output_screen_y: &'a str,
    output_auto_fullscreen: bool,
    transition: Option<&'a TransitionState>,
    video_frame: Option<&'a iced::widget::image::Handle>,
    is_recording: bool,
) -> Element<'a, Message> {
    let top_bar = top_controls(
        presentation,
        ndi_active,
        output_open,
        output_black_screen,
        output_is_fullscreen,
        show_output_settings,
        output_screen_x,
        output_screen_y,
        output_auto_fullscreen,
        is_recording,
    );

    if presentation.slides.is_empty() {
        return container(column![
            top_bar,
            container(
                text("No slides in this presentation")
                    .size(18)
                    .color(theme::TEXT_MUTED),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let current = &presentation.slides[slide_index];
    let next_slide = presentation.slides.get(slide_index + 1);

    let (from_slide, trans_type, trans_progress) = match transition {
        Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
        None => (None, Transition::Cut, 1.0),
    };

    let strip = container(scrollable(slide_strip(presentation, slide_index)).height(Length::Fill))
        .width(210)
        .height(Length::Fill)
        .style(theme::dark_panel_style);

    let center_col = column![
        container(presenter_canvas_panel(
            Some(current),
            from_slide,
            trans_type,
            trans_progress,
            video_frame,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::canvas_bg_style),
        info_bar(current),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    let right = right_panel(
        next_slide,
        output_open,
        output_black_screen,
        current,
        video_frame,
    );

    let body = row![strip, center_col, right]
        .width(Length::Fill)
        .height(Length::Fill);

    container(column![top_bar, body])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[allow(clippy::too_many_arguments)]
fn top_controls<'a>(
    presentation: &'a Presentation,
    ndi_active: bool,
    output_open: bool,
    output_black_screen: bool,
    output_is_fullscreen: bool,
    show_output_settings: bool,
    output_screen_x: &'a str,
    output_screen_y: &'a str,
    output_auto_fullscreen: bool,
    is_recording: bool,
) -> Element<'a, Message> {
    let (ndi_label, ndi_style): (&str, fn(&_, _) -> _) = if ndi_active {
        ("NDI Live", theme::primary_button)
    } else {
        ("NDI Off", theme::secondary_button)
    };

    let (output_label, output_style): (&str, fn(&_, _) -> _) = if output_open {
        ("Output Live", theme::primary_button)
    } else {
        ("Open Output", theme::secondary_button)
    };
    let output_toggle_msg = if output_open {
        Message::CloseOutputWindow
    } else {
        Message::OpenOutputWindow
    };

    let black_btn = if output_open {
        let (bs_text, bs_style): (&str, fn(&_, _) -> _) = if output_black_screen {
            ("Unblank", theme::primary_button)
        } else {
            ("Black Out", theme::ghost_button)
        };
        button(
            row![
                fa_icon_solid("moon").size(12.0),
                text(format!(" {bs_text}")).size(12),
            ]
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ToggleOutputBlackScreen)
        .padding([6, 12])
        .style(bs_style)
    } else {
        button(
            row![
                fa_icon_solid("moon").size(12.0),
                text(" Black Out").size(12),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([6, 12])
        .style(theme::ghost_button)
    };

    let fullscreen_btn: Element<'a, Message> = if output_open {
        let (fs_label, fs_style): (&str, fn(&_, _) -> _) = if output_is_fullscreen {
            ("Windowed", theme::secondary_button)
        } else {
            ("Fullscreen", theme::ghost_button)
        };
        let fs_icon = if output_is_fullscreen {
            "compress"
        } else {
            "expand"
        };
        button(
            row![
                fa_icon_solid(fs_icon).size(12.0),
                text(format!(" {fs_label}")).size(12),
            ]
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::OutputFullscreenToggled)
        .padding([6, 12])
        .style(fs_style)
        .into()
    } else {
        Space::new().width(0).into()
    };

    let settings_style: fn(&_, _) -> _ = if show_output_settings {
        theme::primary_button
    } else {
        theme::ghost_button
    };
    let settings_btn = button(fa_icon_solid("gear").size(12.0))
        .on_press(Message::ToggleOutputSettings)
        .padding([6, 10])
        .style(settings_style);

    let toolbar_row = row![
        button(
            row![
                fa_icon_solid("arrow-left")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
                text(" Back").size(12).color(theme::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::BackToList)
        .padding([6, 14])
        .style(theme::ghost_button),
        Space::new().width(14),
        text(&presentation.name).size(16).color(theme::TEXT_PRIMARY),
        Space::new().width(Length::Fill),
        button(text(output_label).size(12))
            .on_press(output_toggle_msg)
            .padding([6, 14])
            .style(output_style),
        black_btn,
        fullscreen_btn,
        settings_btn,
        button(text(ndi_label).size(12))
            .on_press(Message::ToggleNdi)
            .padding([6, 14])
            .style(ndi_style),
        button(text("Send").size(12))
            .on_press(Message::NdiSendCurrent)
            .padding([6, 12])
            .style(theme::secondary_button),
        button(text("Clear").size(12))
            .on_press(Message::ClearOutput)
            .padding([6, 12])
            .style(theme::ghost_button),
        {
            let (rec_label, rec_msg, rec_style): (&str, Message, fn(&_, _) -> _) = if is_recording {
                ("Stop Rec", Message::RecordingStop, theme::danger_button)
            } else {
                ("Record", Message::RecordingStart, theme::ghost_button)
            };
            button(text(rec_label).size(12))
                .on_press(rec_msg)
                .padding([6, 12])
                .style(rec_style)
        },
    ]
    .spacing(8)
    .padding([8, 14])
    .align_y(Alignment::Center);

    if show_output_settings {
        let settings_row = container(
            row![
                text("Screen X:").size(11).color(theme::TEXT_SECONDARY),
                text_input("0", output_screen_x)
                    .on_input(Message::OutputScreenXChanged)
                    .width(70)
                    .size(11)
                    .padding([4, 6]),
                text("Y:").size(11).color(theme::TEXT_SECONDARY),
                text_input("0", output_screen_y)
                    .on_input(Message::OutputScreenYChanged)
                    .width(70)
                    .size(11)
                    .padding([4, 6]),
                Space::new().width(12),
                checkbox(output_auto_fullscreen)
                    .label("Auto-fullscreen on open")
                    .on_toggle(Message::OutputAutoFullscreenToggled)
                    .text_size(11)
                    .size(14),
                Space::new().width(12),
                text("(drag the output window to your desired monitor first)")
                    .size(10)
                    .color(theme::TEXT_MUTED),
            ]
            .spacing(8)
            .padding([6, 14])
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                r: 0.08,
                g: 0.08,
                b: 0.12,
                a: 1.0,
            })),
            ..Default::default()
        });

        container(column![
            container(toolbar_row)
                .width(Length::Fill)
                .style(theme::toolbar_style),
            settings_row,
        ])
        .width(Length::Fill)
        .into()
    } else {
        container(toolbar_row)
            .width(Length::Fill)
            .style(theme::toolbar_style)
            .into()
    }
}

fn slide_strip<'a>(presentation: &'a Presentation, active: usize) -> Column<'a, Message> {
    let mut col = Column::new().spacing(0).padding([6, 6]);

    let mut groups: Vec<(Option<String>, usize, usize)> = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let lbl = slide.group_label.clone();
        if let Some(last) = groups.last_mut()
            && last.0 == lbl
        {
            last.2 += 1;
            continue;
        }
        groups.push((lbl, i, 1));
    }

    for (group_label, start, count) in groups {
        if let Some(ref lbl) = group_label {
            col = col.push(Space::new().height(6));
            col = col.push(group_label_widget(lbl));
            col = col.push(Space::new().height(2));
        }

        for i in start..(start + count) {
            let slide = &presentation.slides[i];
            let is_live = i == active;
            let bg_color = bg_to_color(&slide.background);
            let preview_str = match &slide.content {
                crate::slides::SlideContent::Text { text, .. } => {
                    if text.is_empty() {
                        "(Empty)".to_string()
                    } else {
                        truncate(text, 28)
                    }
                }
                crate::slides::SlideContent::Image { .. } => "\u{1F4F7} Image".to_string(),
                crate::slides::SlideContent::Video { .. } => "\u{1F3AC} Video".to_string(),
            };

            let txt_color = if let crate::slides::SlideContent::Text { style, .. } = &slide.content
            {
                Color::from_rgba8(
                    style.color.r,
                    style.color.g,
                    style.color.b,
                    style.color.a as f32 / 255.0,
                )
            } else {
                Color::WHITE
            };

            let thumb = container(
                container(text(preview_str).size(8).color(txt_color))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(5)
                    .center(Length::Fill),
            )
            .width(174)
            .height(98)
            .style(move |_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                border: Border {
                    color: if is_live {
                        theme::LIVE_GREEN
                    } else {
                        theme::BORDER_STRONG
                    },
                    width: if is_live { 2.0 } else { 1.0 },
                    radius: 3.0.into(),
                },
                ..Default::default()
            });

            let live_badge_el: Element<Message> = if is_live {
                live_badge_widget("LIVE")
            } else {
                text(format!("{}", i + 1))
                    .size(9)
                    .color(theme::TEXT_MUTED)
                    .into()
            };

            let card = column![
                live_badge_el,
                button(thumb)
                    .on_press(Message::PresentingSelectSlide(i))
                    .padding(0)
                    .style(theme::invisible_button),
            ]
            .spacing(3)
            .padding([2, 0]);

            col = col.push(card);
            col = col.push(Space::new().height(4));
        }
    }

    col
}

fn info_bar<'a>(slide: &'a Slide) -> Element<'a, Message> {
    let mut row_items: Vec<Element<Message>> = Vec::new();

    if let Some(ref lbl) = slide.group_label {
        row_items.push(group_label_widget(lbl));
        row_items.push(Space::new().width(10).into());
    }

    if let Some(ref notes) = slide.notes {
        row_items.push(
            text(notes.as_str())
                .size(12)
                .color(theme::TEXT_SECONDARY)
                .into(),
        );
    }

    if row_items.is_empty() {
        return Space::new().height(0).into();
    }

    let content = row_items.into_iter().fold(
        iced::widget::Row::new()
            .align_y(Alignment::Center)
            .padding([6, 12])
            .spacing(0),
        |r, e| r.push(e),
    );

    container(content)
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_DARK)),
            border: Border {
                color: theme::BORDER_PANEL,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn right_panel<'a>(
    next: Option<&'a Slide>,
    output_open: bool,
    output_black_screen: bool,
    current_slide: &'a Slide,
    video_frame: Option<&'a iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let next_section = next_panel(next);

    if !output_open {
        return next_section;
    }

    let conf_header = container(
        text("CONFIDENCE MONITOR")
            .size(9)
            .color(theme::TEXT_SECONDARY),
    )
    .padding([4, 10])
    .width(Length::Fill)
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(theme::BG_DARKEST)),
        ..Default::default()
    });

    let conf_preview: Element<Message> = if output_black_screen {
        container(text("BLACK").size(10).color(theme::TEXT_MUTED))
            .width(Length::Fill)
            .height(130)
            .center(Length::Fill)
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(iced::Color::BLACK)),
                ..Default::default()
            })
            .into()
    } else {
        container(next_slide_canvas_panel(Some(current_slide)))
            .width(Length::Fill)
            .height(130)
            .into()
    };

    let _ = video_frame;

    column![next_section, conf_header, conf_preview]
        .width(220)
        .height(Length::Fill)
        .into()
}

fn next_panel<'a>(next: Option<&'a Slide>) -> Element<'a, Message> {
    let header = container(text("NEXT").size(10).color(theme::LIVE_GREEN))
        .padding([6, 10])
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_DARKEST)),
            border: Border {
                color: theme::BORDER_PANEL,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

    let preview: Element<Message> = match next {
        Some(slide) => container(next_slide_canvas_panel(Some(slide)))
            .width(Length::Fill)
            .height(170)
            .into(),
        None => container(text("—").size(13).color(theme::TEXT_MUTED))
            .width(Length::Fill)
            .height(170)
            .center(Length::Fill)
            .into(),
    };

    let label: Element<Message> = match next.and_then(|s| s.group_label.as_deref()) {
        Some(lbl) => group_label_widget(lbl),
        None => Space::new().height(0).into(),
    };

    container(
        column![header, preview, label]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(220)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}
