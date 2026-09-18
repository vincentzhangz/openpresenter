use crate::ui::audio;
use crate::ui::editor;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, RightDockTab};
use crate::ui::presenter::canvas::presenter_composite_panel;
use crate::ui::theme;
use crate::ui::{props, triggers};
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Row, Space, button, column, container, row, scrollable, text, text_input},
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

    let layers = w.composite_layers_for_screen(&w.presenting.preview_screen_target);

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
        presenter_composite_panel(
            current,
            from,
            trans,
            progress,
            w.video.frame.as_ref(),
            &layers,
        )
    };

    let target = &w.presenting.preview_screen_target;
    let preview = container(
        column![
            container(
                row![
                    text("PREVIEW").size(10).color(theme::TEXT_MUTED),
                    Space::new().width(Length::Fill),
                    screen_toggle_btn(
                        "Audience",
                        target == "main",
                        Message::SelectPreviewScreen("main".to_string())
                    ),
                    screen_toggle_btn(
                        "Stream",
                        target == "stream",
                        Message::SelectPreviewScreen("stream".to_string())
                    ),
                    screen_toggle_btn(
                        "Stage",
                        target == "stage",
                        Message::SelectPreviewScreen("stage".to_string())
                    ),
                ]
                .align_y(Alignment::Center)
                .spacing(4)
                .padding([6, 8]),
            )
            .style(theme::section_header_style),
            container(preview_body).height(180).width(Length::Fill),
        ]
        .spacing(0),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style);

    let look_bar = look_quick_bar(w);
    let clears = clear_section(w);
    let transport = transport_row(w);
    let controls = show_controls(w);

    let col = column![preview, look_bar, clears, transport, controls]
        .spacing(8)
        .padding([8, 8]);

    container(col)
        .width(DOCK_W)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

fn look_quick_bar<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let current_look_name = w
        .props
        .manager
        .current_look()
        .map(|l| l.name.as_str())
        .unwrap_or("Default");

    let look_info = row![
        fa_icon_solid("sliders")
            .size(11.0_f32)
            .color(theme::ACCENT_ORANGE),
        text(format!("LOOK: {current_look_name}"))
            .size(11)
            .color(theme::TEXT_PRIMARY),
        Space::new().width(Length::Fill),
        button(
            row![
                text("Matrix").size(10).color(theme::TEXT_SECONDARY),
                fa_icon_solid("table-cells")
                    .size(10.0_f32)
                    .color(theme::TEXT_SECONDARY),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleLooksMatrixModal)
        .padding([3, 8])
        .style(theme::secondary_button),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding([5, 8]);

    container(look_info)
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}

fn screen_toggle_btn<'a>(
    label: &'static str,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let color = if active {
        theme::LIVE_GREEN
    } else {
        theme::TEXT_MUTED
    };
    let content = row![
        container(Space::new().width(6).height(6)).style(move |_: &iced::Theme| {
            iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
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
        .on_press(on_press)
        .padding([2, 5])
        .style(theme::ghost_button)
        .into()
}

fn clear_section<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let black = w.output.black_screen;
    let clear_all = button(
        row![
            fa_icon_solid("ban").size(11.0_f32),
            text("Clear All").size(11).color(theme::TEXT_PRIMARY),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ClearAll)
    .padding([6, 10])
    .width(Length::FillPortion(1))
    .style(theme::danger_button);

    let black_btn = button(
        row![
            fa_icon_solid("moon").size(11.0_f32),
            text(if black { "Unblack" } else { "Black" }).size(11),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ToggleOutputBlackScreen)
    .padding([6, 10])
    .width(Length::FillPortion(1))
    .style(if black {
        theme::danger_button
    } else {
        theme::secondary_button
    });

    let pres = w.presenting.presentation.as_ref();
    let current = pres.and_then(|p| p.slides.get(w.presenting.slide_index));

    let audio_active = w.audio.playing;
    let messages_active = w.props.manager.live_message.is_some();
    let props_active = w.props.manager.props.iter().any(|p| p.visible);
    let slide_active = w.presenting.slide_layer_active && current.is_some();
    let media_active = w.presenting.media_layer_active
        && (w.video.frame.is_some()
            || current
                .map(|s| {
                    matches!(
                        s.background,
                        crate::domain::Background::Image(_) | crate::domain::Background::Video(_)
                    )
                })
                .unwrap_or(false));
    let live_active = w.presenting.ndi_output.is_some();

    let row1 = row![
        layer_clear_btn("Slide", "clone", slide_active, Message::ClearSlide),
        layer_clear_btn("Media", "photo-film", media_active, Message::ClearMedia),
        layer_clear_btn("Props", "gauge", props_active, Message::ClearProps),
    ]
    .spacing(4)
    .width(Length::Fill);

    let row2 = row![
        layer_clear_btn(
            "Messages",
            "bullhorn",
            messages_active,
            Message::ClearMessages
        ),
        layer_clear_btn("Audio", "volume-high", audio_active, Message::ClearAudio),
        layer_clear_btn(
            "Live",
            "tower-broadcast",
            live_active,
            Message::NdiBlackScreen
        ),
    ]
    .spacing(4)
    .width(Length::Fill);

    container(
        column![
            row![clear_all, black_btn].spacing(6).width(Length::Fill),
            row1,
            row2,
        ]
        .spacing(6)
        .padding([8, 8]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn layer_clear_btn<'a>(
    label: &'static str,
    icon: &'static str,
    active: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let color = if active {
        theme::DANGER_RED
    } else {
        theme::TEXT_MUTED
    };
    let content = row![
        container(Space::new().width(5).height(5)).style(move |_: &iced::Theme| {
            iced::widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        fa_icon_solid(icon).size(10.0_f32).color(if active {
            theme::DANGER_RED
        } else {
            theme::TEXT_MUTED
        }),
        text(label).size(10).color(if active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        }),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    button(content)
        .on_press(on_press)
        .width(Length::FillPortion(1))
        .padding([5, 4])
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
        (RightDockTab::Messages, "Messages", "bullhorn"),
        (RightDockTab::Timers, "Timers", "clock"),
        (RightDockTab::Audio, "Audio", "volume-high"),
        (RightDockTab::Triggers, "Triggers", "bolt"),
    ];
    let mut tab_row = Row::new().spacing(2).align_y(Alignment::Center);
    for (t, label, icon) in tabs {
        tab_row = tab_row.push(control_tab(label, icon, t == tab));
    }

    let body: Element<'a, Message> = match tab {
        RightDockTab::Props => props::view(w),
        RightDockTab::Messages => messages_panel(w),
        RightDockTab::Triggers => triggers::view(w),
        RightDockTab::Audio => audio::view(&w.audio),
        RightDockTab::Timers => timers_panel(w),
        RightDockTab::ShowControls => show_controls_default(w),
    };

    container(
        column![
            container(
                scrollable(tab_row)
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default()
                    ))
                    .width(Length::Fill)
            )
            .style(theme::section_header_style),
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
        fa_icon_solid(icon).size(11.0_f32).color(if active {
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
    .spacing(3)
    .align_y(Alignment::Center);
    button(content)
        .on_press(Message::SelectRightDockTab(tab_of(label)))
        .padding([4, 6])
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
        "Messages" => RightDockTab::Messages,
        "Triggers" => RightDockTab::Triggers,
        "Audio" => RightDockTab::Audio,
        "Timers" => RightDockTab::Timers,
        _ => RightDockTab::ShowControls,
    }
}

fn messages_panel<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let msg_active = w.props.manager.live_message.is_some();
    let msg_input = &w.output.message_input;
    column![
        row![
            fa_icon_solid("bullhorn")
                .size(12.0_f32)
                .color(theme::ACCENT_ORANGE),
            text("STAGE / AUDIENCE ALERT")
                .size(11)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            if msg_active {
                button(text("Dismiss Alert").size(10).color(theme::DANGER_RED))
                    .on_press(Message::DismissLiveMessage)
                    .padding([2, 6])
                    .style(theme::ghost_button)
            } else {
                button(Space::new().width(0).height(0)).style(theme::ghost_button)
            },
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        row![
            text_input("Alert text (e.g. Parent #402)", msg_input)
                .on_input(Message::LiveMessageInputChanged)
                .on_submit(Message::SendLiveMessage(msg_input.clone()))
                .size(11)
                .padding([6, 8])
                .width(Length::Fill),
            button(text("Send").size(11))
                .on_press(Message::SendLiveMessage(msg_input.clone()))
                .padding([6, 10])
                .style(theme::primary_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(8)
    .padding([8, 8])
    .into()
}

fn show_controls_default<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let msg_active = w.props.manager.live_message.is_some();
    let msg_input = &w.output.message_input;
    let msg_section = column![
        row![
            fa_icon_solid("bullhorn")
                .size(11.0_f32)
                .color(theme::ACCENT_ORANGE),
            text("STAGE / LIVE ALERT")
                .size(10)
                .color(theme::TEXT_PRIMARY),
            Space::new().width(Length::Fill),
            if msg_active {
                button(text("Dismiss").size(9).color(theme::DANGER_RED))
                    .on_press(Message::DismissLiveMessage)
                    .padding([2, 6])
                    .style(theme::ghost_button)
            } else {
                button(Space::new().width(0).height(0)).style(theme::ghost_button)
            },
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        row![
            text_input("Alert (e.g. Parent #402)", msg_input)
                .on_input(Message::LiveMessageInputChanged)
                .on_submit(Message::SendLiveMessage(msg_input.clone()))
                .size(11)
                .padding([4, 6])
                .width(Length::Fill),
            button(text("Send").size(10))
                .on_press(Message::SendLiveMessage(msg_input.clone()))
                .padding([4, 8])
                .style(theme::primary_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    column![
        control_action(
            "Looks & Screens Matrix",
            "sliders",
            Message::ToggleLooksMatrixModal
        ),
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
        container(msg_section)
            .padding([6, 8])
            .style(theme::dark_panel_style),
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
    let editing = w.editor.editing.as_ref();
    if editing.is_none() && w.shell.inspector_tab != crate::ui::messages::InspectorTab::Theme {
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
    let selected = w.editor.selected_slide_index;
    let slide = editing.and_then(|p| selected.and_then(|i| p.slides.get(i)));

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
        can_paste_style: w.editor.copied_style.is_some(),
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
