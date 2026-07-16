pub mod canvas;

use crate::domain::{Cue, Presentation, Slide, Transition};
use crate::ui::components::{group_label_widget, live_badge as live_badge_widget, truncate};
use crate::ui::editor::canvas::bg_to_color;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::presenter::canvas::{next_slide_canvas_panel, presenter_canvas_panel};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Task,
    widget::{
        Column, Space, button, checkbox, column, container, row, scrollable, text, text_input,
    },
};
use iced_font_awesome::fa_icon_solid;
use std::time::{Duration, Instant};

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
                fa_icon_solid("moon").size(12.0_f32),
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
                fa_icon_solid("moon").size(12.0_f32),
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
                fa_icon_solid(fs_icon).size(12.0_f32),
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
    let settings_btn = button(fa_icon_solid("gear").size(12.0_f32))
        .on_press(Message::ToggleOutputSettings)
        .padding([6, 10])
        .style(settings_style);

    let toolbar_row = row![
        button(
            row![
                fa_icon_solid("arrow-left")
                    .size(12.0_f32)
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
            .on_press(Message::Ndi(crate::ui::ndi::Message::Toggle))
            .padding([6, 14])
            .style(ndi_style),
        button(text("Send").size(12))
            .on_press(Message::Ndi(crate::ui::ndi::Message::SendCurrent))
            .padding([6, 12])
            .style(theme::secondary_button),
        button(text("Clear").size(12))
            .on_press(Message::ClearOutput)
            .padding([6, 12])
            .style(theme::ghost_button),
        {
            let (rec_label, rec_msg, rec_style): (&str, Message, fn(&_, _) -> _) = if is_recording {
                (
                    "Stop Rec",
                    Message::Recording(crate::ui::recording::Message::Stop),
                    theme::danger_button,
                )
            } else {
                (
                    "Record",
                    Message::Recording(crate::ui::recording::Message::Start),
                    theme::ghost_button,
                )
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
        let lbl = slide.group.clone();
        if let Some(last) = groups.last_mut()
            && last.0 == lbl
        {
            last.2 += 1;
            continue;
        }
        groups.push((lbl, i, 1));
    }

    for (group, start, count) in groups {
        if let Some(ref lbl) = group {
            col = col.push(Space::new().height(6));
            col = col.push(group_label_widget(lbl));
            col = col.push(Space::new().height(2));
        }

        for i in start..(start + count) {
            let slide = &presentation.slides[i];
            let is_live = i == active;
            let bg_color = bg_to_color(&slide.background);
            let preview_str = match &slide.content {
                crate::domain::SlideContent::Text { text, .. } => {
                    if text.is_empty() {
                        "(Empty)".to_string()
                    } else {
                        truncate(text, 28)
                    }
                }
                crate::domain::SlideContent::Image { .. } => "\u{1F4F7} Image".to_string(),
                crate::domain::SlideContent::Video { .. } => "\u{1F3AC} Video".to_string(),
            };

            let txt_color = if let crate::domain::SlideContent::Text { style, .. } = &slide.content
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

    if let Some(ref lbl) = slide.group {
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

    let label: Element<Message> = match next.and_then(|s| s.group.as_deref()) {
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

pub(crate) fn activate_slide(w: &mut MainWindow, to_idx: usize, animate: bool) -> Task<Message> {
    let from_slide = {
        let Some(ref pres) = w.presenting.presentation else {
            return Task::none();
        };
        if pres.slides.get(to_idx).is_none() {
            return Task::none();
        }
        if animate {
            if to_idx == w.presenting.slide_index {
                return Task::none();
            }
            pres.slides.get(w.presenting.slide_index).cloned()
        } else {
            None
        }
    };
    begin_slide_change(w, from_slide, to_idx, animate)
}

pub(crate) fn select_slide(w: &mut MainWindow, i: usize) -> Task<Message> {
    activate_slide(w, i, true)
}

pub(crate) fn next_slide(w: &mut MainWindow) -> Task<Message> {
    let next = {
        let Some(ref pres) = w.presenting.presentation else {
            return Task::none();
        };
        let current_i = w.presenting.slide_index;
        let next = (current_i + 1).min(pres.slides.len().saturating_sub(1));
        if next == current_i {
            return Task::none();
        }
        next
    };
    activate_slide(w, next, true)
}

pub(crate) fn prev_slide(w: &mut MainWindow) -> Task<Message> {
    let prev = {
        if w.presenting.presentation.is_none() {
            return Task::none();
        }
        let current_i = w.presenting.slide_index;
        let prev = current_i.saturating_sub(1);
        if prev == current_i {
            return Task::none();
        }
        prev
    };
    activate_slide(w, prev, true)
}

pub(crate) fn animation_tick(w: &mut MainWindow) -> Task<Message> {
    if let Some(ref mut ts) = w.presenting.transition {
        let duration_ms = match ts.transition {
            Transition::Fade { duration_ms }
            | Transition::Dissolve { duration_ms }
            | Transition::Slide { duration_ms }
            | Transition::Push { duration_ms, .. }
            | Transition::Zoom { duration_ms }
            | Transition::Flip { duration_ms }
            | Transition::Clock { duration_ms }
            | Transition::Wipe { duration_ms, .. } => duration_ms as f32,
            Transition::Cut => 0.0,
        };
        ts.progress = if duration_ms > 0.0 {
            (ts.start.elapsed().as_millis() as f32 / duration_ms).min(1.0)
        } else {
            1.0
        };
        if ts.progress >= 1.0 {
            w.presenting.transition = None;
        }
    }
    Task::none()
}

fn queue_slide_cues(w: &MainWindow, cues: Vec<Cue>) {
    let tx = w.triggers.manager.sender();
    for cue in cues {
        if !cue.enabled {
            continue;
        }
        if cue.delay_ms == 0 {
            let _ = tx.try_send(cue.action.clone());
        } else {
            let delayed_tx = tx.clone();
            let action = cue.action.clone();
            let delay_ms = cue.delay_ms;
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let _ = delayed_tx.send(action).await;
            });
        }
    }
}

fn begin_slide_change(
    w: &mut MainWindow,
    from_slide: Option<Slide>,
    to_idx: usize,
    animate: bool,
) -> Task<Message> {
    let (transition, ndi_slide, cues) = {
        let Some(ref pres) = w.presenting.presentation else {
            return Task::none();
        };
        let Some(target) = pres.slides.get(to_idx) else {
            return Task::none();
        };
        (target.transition, target.clone(), target.cues.clone())
    };

    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.slide_index = to_idx;

    if animate && !matches!(transition, Transition::Cut) && !w.ui.reduce_motion {
        if let Some(from_slide) = from_slide {
            w.presenting.transition = Some(TransitionState {
                from_slide,
                transition,
                progress: 0.0,
                start: Instant::now(),
            });
        } else {
            w.presenting.transition = None;
        }
    } else {
        w.presenting.transition = None;
    }

    if let Some(ref ndi) = w.presenting.ndi_output {
        ndi.set_slide(ndi_slide);
    }

    crate::ui::video::on_presenter_slide_changed(w, to_idx);
    queue_slide_cues(w, cues);
    Task::none()
}
