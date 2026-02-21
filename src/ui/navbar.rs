use crate::ui::components::live_badge;
use crate::ui::messages::{Message, ViewMode};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, text},
};

pub fn navbar<'a>(
    current_mode: ViewMode,
    ndi_active: bool,
    stage_active: bool,
    reduce_motion: bool,
    audio_panel_visible: bool,
    triggers_panel_open: bool,
    recording_active: bool,
) -> Element<'a, Message> {
    let logo = text("OpenPresenter").size(17).color(theme::TEXT_PRIMARY);

    let edit_tab = button(text("Edit").size(13))
        .padding([6, 22])
        .on_press(Message::SwitchMode(ViewMode::Edit))
        .style(if current_mode == ViewMode::Edit {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        });

    let show_tab = button(text("Show").size(13))
        .padding([6, 22])
        .on_press(Message::SwitchMode(ViewMode::Show))
        .style(if current_mode == ViewMode::Show && !stage_active {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        });

    let plan_tab = button(text("Plan").size(13))
        .padding([6, 22])
        .on_press(Message::SwitchMode(ViewMode::Plan))
        .style(if current_mode == ViewMode::Plan {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        });

    let stage_tab = button(text("Stage").size(13))
        .padding([6, 18])
        .on_press(Message::ToggleStageDisplay)
        .style(if stage_active {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        });

    let ndi_badge = if ndi_active {
        live_badge("NDI")
    } else {
        container(
            row![text("NDI Off").size(11).color(theme::TEXT_MUTED),]
                .spacing(4)
                .align_y(Alignment::Center)
                .padding([3, 8]),
        )
        .style(|_t: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.0,
            ))),
            border: iced::Border {
                color: theme::BORDER_PANEL,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    };

    let bar = row![
        logo,
        Space::new().width(Length::Fill),
        edit_tab,
        show_tab,
        plan_tab,
        stage_tab,
        Space::new().width(Length::Fill),
        ndi_badge,
        button(
            text(if reduce_motion {
                "Motion Off"
            } else {
                "Motion On"
            })
            .size(11)
            .color(if reduce_motion {
                theme::DANGER_RED
            } else {
                theme::TEXT_MUTED
            }),
        )
        .on_press(Message::ToggleReduceMotion)
        .padding([4, 10])
        .style(theme::ghost_button),
        button(text("?").size(13).color(theme::TEXT_MUTED))
            .on_press(Message::ToggleShortcutsOverlay)
            .padding([4, 10])
            .style(theme::ghost_button),
        button(text("Audio").size(11).color(if audio_panel_visible {
            theme::ACCENT_BLUE
        } else {
            theme::TEXT_MUTED
        }),)
        .on_press(Message::AudioTogglePanel)
        .padding([4, 10])
        .style(theme::ghost_button),
        button(text("Triggers").size(11).color(if triggers_panel_open {
            theme::ACCENT_BLUE
        } else {
            theme::TEXT_MUTED
        }),)
        .on_press(Message::ToggleTriggersPanel)
        .padding([4, 10])
        .style(theme::ghost_button),
        button(
            text(if recording_active { "REC" } else { "Rec" })
                .size(11)
                .color(if recording_active {
                    theme::DANGER_RED
                } else {
                    theme::TEXT_MUTED
                }),
        )
        .on_press(Message::ToggleRecordingPanel)
        .padding([4, 10])
        .style(theme::ghost_button),
    ]
    .spacing(6)
    .padding([0, 16])
    .align_y(Alignment::Center)
    .height(44);

    container(bar)
        .width(Length::Fill)
        .style(theme::navbar_style)
        .into()
}
