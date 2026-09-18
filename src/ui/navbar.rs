use crate::ui::messages::{Message, ViewMode};
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
    live: bool,
}

impl ToolButton {
    fn new(
        icon: &'static str,
        label: &'static str,
        on_press: Option<Message>,
        active: bool,
    ) -> Self {
        Self {
            icon,
            label,
            on_press,
            active,
            live: false,
        }
    }

    fn live(
        icon: &'static str,
        label: &'static str,
        on_press: Option<Message>,
        is_live: bool,
    ) -> Self {
        Self {
            icon,
            label,
            on_press,
            active: is_live,
            live: is_live,
        }
    }
}

fn tool_btn(b: ToolButton) -> Element<'static, Message> {
    let icon_color = if b.live {
        theme::LIVE_GREEN
    } else if b.active {
        theme::ACCENT_ORANGE
    } else {
        theme::TEXT_SECONDARY
    };
    let text_color = if b.live {
        theme::LIVE_GREEN
    } else if b.active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };
    let icon = fa_icon_solid(b.icon).size(16.0_f32).color(icon_color);
    let content = row![
        icon,
        if b.label.is_empty() {
            Element::new(Space::new().width(0))
        } else {
            text(b.label).size(11).color(text_color).into()
        }
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut btn = button(content)
        .padding([6, 10])
        .style(move |_t: &iced::Theme, status| {
            let bg = if b.live {
                if matches!(status, iced::widget::button::Status::Hovered) {
                    Color::from_rgba(0.204, 0.780, 0.349, 0.28)
                } else {
                    Color::from_rgba(0.204, 0.780, 0.349, 0.16)
                }
            } else if b.active {
                Color::from_rgba(0.941, 0.216, 0.031, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            };
            let border_color = if b.live {
                theme::LIVE_GREEN
            } else if b.active {
                theme::ACCENT_ORANGE
            } else {
                theme::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: Border {
                    color: border_color,
                    width: if b.active || b.live { 1.0 } else { 0.0 },
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

fn toolbar_divider() -> Element<'static, Message> {
    container(Space::new().width(1).height(18))
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))),
            ..Default::default()
        })
        .into()
}

fn screen_status_btn(
    label: &'static str,
    active: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let dot_color = if active {
        theme::LIVE_GREEN
    } else {
        theme::TEXT_MUTED
    };
    let text_color = if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };
    let content = row![
        container(Space::new().width(7).height(7)).style(move |_: &iced::Theme| {
            iced::widget::container::Style {
                background: Some(Background::Color(dot_color)),
                border: Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        text(label).size(11).color(text_color),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    button(content)
        .on_press(on_press)
        .padding([5, 10])
        .style(move |_: &iced::Theme, status| {
            let bg = if active {
                Color::from_rgba(0.204, 0.780, 0.349, 0.16)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.04)
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    color: if active {
                        theme::LIVE_GREEN
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

pub(crate) fn navbar<'a>(shell: &'a ShellState, w: NavbarState) -> Element<'a, Message> {
    let logo = row![
        container(
            text("OpenPresenter")
                .size(13)
                .color(theme::TEXT_PRIMARY)
                .font(iced::Font::DEFAULT)
        )
        .padding([4, 6])
    ];

    let text_active = shell.current_mode == ViewMode::Edit
        && shell.inspector_tab == crate::ui::messages::InspectorTab::Text;
    let theme_active = shell.current_mode == ViewMode::Edit
        && shell.inspector_tab == crate::ui::messages::InspectorTab::Theme;

    // Left tools: Search, Text, Theme
    let left_tools = row![
        tool_btn(ToolButton::new(
            "magnifying-glass",
            "Search",
            Some(Message::FocusSearch),
            false,
        )),
        tool_btn(ToolButton::new(
            "font",
            "Text",
            Some(Message::OpenTextEditor),
            text_active,
        )),
        tool_btn(ToolButton::new(
            "palette",
            "Theme",
            Some(Message::OpenTheme),
            theme_active,
        )),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    // Mode selector: Show, Edit, Bible
    let show_active = shell.current_mode == ViewMode::Show;
    let edit_active = shell.current_mode == ViewMode::Edit;

    let mode_buttons = row![
        tool_btn(ToolButton::new(
            "tv",
            "Show",
            Some(Message::SwitchMode(ViewMode::Show)),
            show_active,
        )),
        tool_btn(ToolButton::new(
            "pen-to-square",
            "Edit",
            Some(Message::SwitchMode(ViewMode::Edit)),
            edit_active,
        )),
        tool_btn(ToolButton::new(
            "book-bible",
            "Bible",
            Some(Message::SelectLeftSection(
                crate::ui::messages::SidebarTab::Bible,
            )),
            w.bible_active,
        )),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    // Media and Looks toggles
    let media_looks = row![
        tool_btn(ToolButton::new(
            "photo-film",
            "Media",
            Some(Message::ToggleMediaBin),
            shell.media_bin_open,
        )),
        tool_btn(ToolButton::new(
            "glasses",
            "Looks",
            Some(Message::ToggleLooksMatrixModal),
            w.matrix_open,
        )),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    // Right controls: Capture/REC, NDI, Audience toggle, Stage toggle, Help, Settings
    let rec_active = w.recording_active;
    let right_tools = row![
        tool_btn(ToolButton::live(
            "circle",
            if rec_active { "REC" } else { "Capture" },
            Some(if rec_active {
                Message::Recording(crate::ui::recording::Message::Stop)
            } else {
                Message::Recording(crate::ui::recording::Message::Start)
            }),
            rec_active,
        )),
        tool_btn(ToolButton::live(
            "tower-broadcast",
            if w.ndi_active { "NDI LIVE" } else { "NDI" },
            Some(Message::Ndi(crate::ui::ndi::Message::Toggle)),
            w.ndi_active,
        )),
        toolbar_divider(),
        screen_status_btn("Audience", w.audience_active, Message::ToggleOutputWindow,),
        screen_status_btn("Stage", w.stage_active, Message::ToggleStageDisplay,),
        toolbar_divider(),
        tool_btn(ToolButton::new(
            "circle-question",
            "",
            Some(Message::ToggleShortcutsOverlay),
            false,
        )),
        tool_btn(ToolButton::new(
            "gear",
            "",
            Some(Message::Output(crate::ui::output::Message::SettingsOpen)),
            false,
        )),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let bar = row![
        logo,
        toolbar_divider(),
        left_tools,
        toolbar_divider(),
        mode_buttons,
        toolbar_divider(),
        media_looks,
        Space::new().width(Length::Fill),
        right_tools,
    ]
    .spacing(8)
    .padding([0, 10])
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
    pub audience_active: bool,
    pub stage_active: bool,
    pub reduce_motion: bool,
    pub recording_active: bool,
    pub matrix_open: bool,
    pub bible_active: bool,
}
