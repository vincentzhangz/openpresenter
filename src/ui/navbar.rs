use crate::ui::components::live_badge;
use crate::ui::messages::{Message, RightDockTab, ViewMode};
use crate::ui::state::ShellState;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Space, button, container, row, text},
};
use iced_font_awesome::fa_icon_solid;

struct ToolButton {
    icon: &'static str,
    label: &'static str,
    on_press: Option<Message>,
    active: bool,
}

fn tool_btn(b: ToolButton) -> Element<'static, Message> {
    let icon = fa_icon_solid(b.icon).size(16.0_f32).color(if b.active {
        theme::ACCENT_ORANGE
    } else {
        theme::TEXT_SECONDARY
    });
    let content = row![
        icon,
        if b.label.is_empty() {
            Element::new(Space::new().width(0))
        } else {
            text(b.label)
                .size(11)
                .color(if b.active {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_MUTED
                })
                .into()
        }
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut btn = button(content)
        .padding([6, 10])
        .style(move |_t: &iced::Theme, status| {
            let bg = if b.active {
                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    color: if b.active {
                        theme::ACCENT_ORANGE
                    } else {
                        theme::TRANSPARENT
                    },
                    width: if b.active { 1.0 } else { 0.0 },
                    radius: 4.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });
    if let Some(m) = b.on_press {
        btn = btn.on_press(m);
    }
    btn.into()
}

pub(crate) fn navbar<'a>(shell: &'a ShellState, w: NavbarState) -> Element<'a, Message> {
    let logo = row![
        container(
            text("OpenPresenter")
                .size(14)
                .color(theme::TEXT_PRIMARY)
                .font(iced::Font::DEFAULT)
        )
        .padding([4, 10])
    ];

    let show_active = shell.current_mode == ViewMode::Show;
    let edit_active = shell.current_mode == ViewMode::Edit;

    let mode_buttons = row![
        tool_btn(ToolButton {
            icon: "tv",
            label: "Show",
            on_press: Some(Message::SwitchMode(ViewMode::Show)),
            active: show_active,
        }),
        tool_btn(ToolButton {
            icon: "pen-to-square",
            label: "Edit",
            on_press: Some(Message::SwitchMode(ViewMode::Edit)),
            active: edit_active,
        }),
    ]
    .spacing(2);

    let left_tools = row![
        tool_btn(ToolButton {
            icon: "magnifying-glass",
            label: "",
            on_press: Some(Message::FocusSearch),
            active: false,
        }),
        tool_btn(ToolButton {
            icon: "font",
            label: "",
            on_press: None,
            active: false,
        }),
        tool_btn(ToolButton {
            icon: "palette",
            label: "",
            on_press: None,
            active: false,
        }),
        tool_btn(ToolButton {
            icon: "photo-film",
            label: "",
            on_press: Some(Message::ToggleMediaBin),
            active: shell.media_bin_open,
        }),
        tool_btn(ToolButton {
            icon: "glasses",
            label: "",
            on_press: None,
            active: false,
        }),
    ]
    .spacing(2);

    let more_btn = tool_btn(ToolButton {
        icon: "ellipsis",
        label: "More",
        on_press: Some(Message::ToggleTriggersPanel),
        active: false,
    });

    let rec_active = w.recording_active;
    let right_tools = row![
        tool_btn(ToolButton {
            icon: "circle",
            label: if rec_active { "REC" } else { "Rec" },
            on_press: Some(if rec_active {
                Message::Recording(crate::ui::recording::Message::Stop)
            } else {
                Message::Recording(crate::ui::recording::Message::Start)
            }),
            active: rec_active,
        }),
        if w.ndi_active {
            live_badge("NDI")
        } else {
            tool_btn(ToolButton {
                icon: "tower-broadcast",
                label: "NDI",
                on_press: Some(Message::Ndi(crate::ui::ndi::Message::Toggle)),
                active: false,
            })
        },
        tool_btn(ToolButton {
            icon: "person-shelter",
            label: "Stage",
            on_press: Some(Message::ToggleStageDisplay),
            active: w.stage_active,
        }),
        tool_btn(ToolButton {
            icon: "gauge",
            label: "Props",
            on_press: Some(Message::SelectRightDockTab(RightDockTab::Props)),
            active: shell.right_dock_tab == RightDockTab::Props,
        }),
        tool_btn(ToolButton {
            icon: "bolt",
            label: "Triggers",
            on_press: Some(Message::SelectRightDockTab(RightDockTab::Triggers)),
            active: shell.right_dock_tab == RightDockTab::Triggers,
        }),
        tool_btn(ToolButton {
            icon: if w.reduce_motion {
                "toggle-off"
            } else {
                "toggle-on"
            },
            label: "",
            on_press: Some(Message::ToggleReduceMotion),
            active: false,
        }),
        tool_btn(ToolButton {
            icon: "circle-question",
            label: "",
            on_press: Some(Message::ToggleShortcutsOverlay),
            active: false,
        }),
    ]
    .spacing(2);

    let bar = row![
        logo,
        Space::new().width(8),
        mode_buttons,
        Space::new().width(12),
        left_tools,
        Space::new().width(Length::Fill),
        more_btn,
        Space::new().width(12),
        right_tools,
    ]
    .spacing(2)
    .padding([0, 12])
    .align_y(Alignment::Center)
    .height(46);

    container(bar)
        .width(Length::Fill)
        .style(theme::navbar_style)
        .into()
}

#[derive(Clone, Copy)]
pub struct NavbarState {
    pub ndi_active: bool,
    pub stage_active: bool,
    pub reduce_motion: bool,
    pub recording_active: bool,
}
