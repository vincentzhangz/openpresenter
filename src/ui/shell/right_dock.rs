use crate::ui::audio;
use crate::ui::editor;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, RightDockTab};
use crate::ui::presenter::canvas::presenter_canvas_panel;
use crate::ui::theme;
use crate::ui::{props, triggers};
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Row, Space, button, column, container, row, scrollable, text},
};
use iced_font_awesome::fa_icon_solid;

const DOCK_W: f32 = 320.0;

/// Right dock of the unified shell.
///
/// * Show mode: 16:9 Preview + Clear-All / layer clears + transport + tabbed
///   Show Controls (Audio / Props / Triggers / Timers).
/// * Edit mode: Inspector (Slide / Text / Shape / Build / Theme / Actions tabs)
///   reusing the existing `editor::inspector` panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    match w.shell.current_mode {
        crate::ui::messages::ViewMode::Show => show_dock(w),
        crate::ui::messages::ViewMode::Edit => edit_dock(w),
    }
}

fn show_dock<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let pres = w.presenting.presentation.as_ref();
    let slide_index = w.presenting.slide_index;
    let current = pres.and_then(|p| p.slides.get(slide_index));
    let (from, trans, progress) = match &w.presenting.transition {
        Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
        None => (None, crate::domain::Transition::Cut, 1.0),
    };

    let preview_body: Element<'a, Message> = if w.output.black_screen {
        container(text("BLACK").size(14).color(theme::TEXT_MUTED))
            .width(Length::Fill)
            .height(180)
            .center(Length::Fill)
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into()
    } else {
        presenter_canvas_panel(current, from, trans, progress, w.video.frame.as_ref())
    };

    let preview = container(
        column![
            container(
                row![
                    text("PREVIEW").size(10).color(theme::TEXT_MUTED),
                    Space::new().width(Length::Fill),
                    audience_stage_toggle("Audience", true),
                    audience_stage_toggle("Stage", w.presenting.stage_display_active),
                ]
                .align_y(Alignment::Center)
                .spacing(6)
                .padding([6, 8]),
            )
            .style(theme::section_header_style),
            container(preview_body).height(180).width(Length::Fill),
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style);

    let clears = clear_section(w);
    let transport = transport_row(w);
    let controls = show_controls(w);

    let col = column![preview, clears, transport, controls]
        .spacing(8)
        .padding([8, 8]);

    container(col)
        .width(DOCK_W)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

fn audience_stage_toggle(label: &'static str, on: bool) -> Element<'static, Message> {
    let color = if on {
        theme::LIVE_GREEN
    } else {
        theme::TEXT_MUTED
    };
    let content = row![
        container(Space::new().width(8).height(8)).style(move |_: &iced::Theme| {
            iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 9.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        text(label).size(10).color(theme::TEXT_SECONDARY),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    button(content)
        .padding([2, 6])
        .style(theme::ghost_button)
        .into()
}

fn clear_section<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let black = w.output.black_screen;
    let clear_all = button(text("Clear All").size(11).color(theme::TEXT_PRIMARY))
        .on_press(Message::ClearOutput)
        .padding([6, 10])
        .width(Length::Fill)
        .style(theme::danger_button);

    let layer_clears = [
        ("Audio", false),
        ("Messages", false),
        ("Props", w.props.panel_open),
        ("Slide", false),
        ("Media", false),
        ("Live", false),
    ];

    let mut layer_row = Row::new().spacing(4).padding([0, 0]);
    for (label, active) in layer_clears {
        layer_row = layer_row.push(layer_clear_btn(label, active));
    }

    let black_btn = button(text(if black { "Unblack" } else { "Black" }).size(11))
        .on_press(Message::ToggleOutputBlackScreen)
        .padding([6, 10])
        .width(Length::Fill)
        .style(if black {
            theme::danger_button
        } else {
            theme::secondary_button
        });

    container(
        column![row![clear_all, black_btn].spacing(6), layer_row,]
            .spacing(6)
            .padding([8, 8]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn layer_clear_btn(label: &'static str, active: bool) -> Element<'static, Message> {
    let color = if active {
        theme::DANGER_RED
    } else {
        theme::TEXT_MUTED
    };
    let content = row![
        container(Space::new().width(7).height(7)).style(move |_: &iced::Theme| {
            iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        text(label).size(9).color(theme::TEXT_SECONDARY),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    button(content)
        .padding([4, 8])
        .style(move |_t: &iced::Theme, status| {
            let bg = if active {
                Color::from_rgba(1.0, 0.271, 0.227, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::BG_DARK
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: if active {
                        theme::DANGER_RED
                    } else {
                        theme::BORDER_PANEL
                    },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn transport_row<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let has_pres = w.presenting.presentation.is_some();
    let prev = {
        let b = button(
            fa_icon_solid("backward-step")
                .size(13.0_f32)
                .color(theme::TEXT_PRIMARY),
        )
        .padding([6, 10])
        .style(theme::ghost_button);
        if has_pres {
            b.on_press(Message::PresentingPrevSlide)
        } else {
            b
        }
    };
    let next = {
        let b = button(
            fa_icon_solid("forward-step")
                .size(13.0_f32)
                .color(theme::TEXT_PRIMARY),
        )
        .padding([6, 10])
        .style(theme::primary_button);
        if has_pres {
            b.on_press(Message::PresentingNextSlide)
        } else {
            b
        }
    };
    let go = button(text("Go").size(11).color(theme::TEXT_PRIMARY))
        .on_press(Message::PresentingNextSlide)
        .padding([6, 16])
        .style(theme::primary_button);

    container(
        row![prev, go, next]
            .spacing(6)
            .align_y(Alignment::Center)
            .padding([8, 8]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn show_controls<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let tab = w.shell.right_dock_tab;
    let tabs = [
        (RightDockTab::ShowControls, "Show", "sliders"),
        (RightDockTab::Props, "Props", "gauge"),
        (RightDockTab::Triggers, "Triggers", "bolt"),
        (RightDockTab::Audio, "Audio", "volume-high"),
        (RightDockTab::Timers, "Timers", "clock"),
    ];
    let mut tab_row = Row::new().spacing(2);
    for (t, label, icon) in tabs {
        tab_row = tab_row.push(control_tab(label, icon, t == tab));
    }

    let body: Element<'a, Message> = match tab {
        RightDockTab::Props => props::view(w),
        RightDockTab::Triggers => triggers::view(w),
        RightDockTab::Audio => audio::view(&w.audio),
        RightDockTab::Timers => timers_panel(w),
        RightDockTab::ShowControls => show_controls_default(w),
    };

    container(
        column![
            container(tab_row).style(theme::section_header_style),
            container(scrollable(body).height(Length::Fill))
                .width(Length::Fill)
                .padding([8, 8]),
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn control_tab(label: &'static str, icon: &'static str, active: bool) -> Element<'static, Message> {
    let content = row![
        fa_icon_solid(icon).size(12.0_f32).color(if active {
            theme::ACCENT_ORANGE
        } else {
            theme::TEXT_MUTED
        }),
        text(label).size(10).color(if active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    button(content)
        .on_press(Message::SelectRightDockTab(tab_of(label)))
        .padding([6, 8])
        .style(move |_t: &iced::Theme, status| {
            let bg = if active {
                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: if active {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TRANSPARENT
                    },
                    width: if active { 1.0 } else { 0.0 },
                    radius: 4.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn tab_of(label: &str) -> RightDockTab {
    match label {
        "Props" => RightDockTab::Props,
        "Triggers" => RightDockTab::Triggers,
        "Audio" => RightDockTab::Audio,
        "Timers" => RightDockTab::Timers,
        _ => RightDockTab::ShowControls,
    }
}

fn show_controls_default<'a>(_w: &'a MainWindow) -> Element<'a, Message> {
    column![
        control_action("Open Output Screen", "desktop", Message::OpenOutputWindow),
        control_action(
            "Toggle Stage Display",
            "person-shelter",
            Message::ToggleStageDisplay
        ),
        control_action(
            "Toggle NDI",
            "tower-broadcast",
            Message::Ndi(crate::ui::ndi::Message::Toggle)
        ),
        control_action("Clear Output", "eraser", Message::ClearOutput),
    ]
    .spacing(6)
    .into()
}

fn control_action<'a>(
    label: &'static str,
    icon: &'static str,
    on_press: Message,
) -> Element<'a, Message> {
    let content = row![
        fa_icon_solid(icon)
            .size(13.0_f32)
            .color(theme::TEXT_SECONDARY),
        text(label).size(12).color(theme::TEXT_SECONDARY),
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    button(content)
        .on_press(on_press)
        .width(Length::Fill)
        .padding([8, 10])
        .style(theme::ghost_button)
        .into()
}

fn timers_panel<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let secs = w.presenting.timer_secs;
    let mm = secs / 60;
    let ss = secs % 60;
    column![
        container(
            text(format!("{mm:02}:{ss:02}"))
                .size(28)
                .color(theme::TEXT_PRIMARY),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .padding([12, 0]),
        row![
            control_action("Start", "play", Message::StartTimer),
            control_action("Stop", "stop", Message::StopTimer),
            control_action("Reset", "rotate-left", Message::ResetTimer),
        ]
        .spacing(4),
    ]
    .spacing(8)
    .into()
}

fn edit_dock<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let editing = match w.editor.editing.as_ref() {
        Some(p) => p,
        None => {
            return container(
                text("Open a presentation to edit")
                    .size(12)
                    .color(theme::TEXT_MUTED),
            )
            .width(DOCK_W)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(theme::panel_style)
            .into();
        }
    };
    let selected = w.editor.selected_slide_index;
    let slide = selected.and_then(|i| editing.slides.get(i));

    let layer_state = editor::LayerPanelState {
        selected_layer_index: w.layer.selected_index,
        editing_text: w.layer.text.clone(),
        editing_font_size: w.layer.font_size.clone(),
        editing_pos_x: w.layer.pos_x.clone(),
        editing_pos_y: w.layer.pos_y.clone(),
        editing_width: w.layer.width.clone(),
        editing_height: w.layer.height.clone(),
        editing_stroke_width: w.layer.stroke_width.clone(),
        editing_font_family: w.layer.font_family.clone(),
        editing_line_height: w.layer.line_height.clone(),
        editing_letter_spacing: w.layer.letter_spacing.clone(),
        editing_glow_radius: w.layer.glow_radius.clone(),
        editing_text_stroke_width: w.layer.text_stroke_width.clone(),
    };

    let inspector = editor::inspector::inspector_panel(
        slide,
        w.shell.inspector_tab,
        &w.editor.editing_slide_text,
        &w.editor.editing_slide_font_size,
        &w.editor.editing_transition_duration,
        &w.editor.editing_group_label,
        &w.editor.editing_slide_notes,
        &w.theme_state.list,
        w.theme_state.selected_theme_id.as_deref(),
        &w.theme_state.new_theme_name,
        w.video.player.as_ref().is_some_and(|p| p.is_playing()),
        w.video.looping,
        w.video.volume,
        w.video.muted,
        w.video.speed,
        w.video.position,
        w.video
            .player
            .as_ref()
            .map(|p| p.duration_secs())
            .unwrap_or(0.0),
        &layer_state,
    );

    container(inspector)
        .width(DOCK_W)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

pub const DOCK_WIDTH: f32 = DOCK_W;
