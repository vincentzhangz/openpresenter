use crate::domain::Presentation;
use crate::ui::components::{add_button, search_input, section_header};
use crate::ui::library;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, SidebarTab};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Column, Space, button, column, container, row, scrollable, text},
};
use iced_font_awesome::fa_icon_solid;

pub const RAIL_WIDTH: f32 = 260.0;

/// ProPresenter-style left rail: collapsible outline of libraries + playlists,
/// with a detail list underneath.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let tab = w.shell.sidebar_tab;

    let sections: Vec<(SidebarTab, &'static str, &'static str)> = vec![
        (SidebarTab::Presentations, "layer-group", "Presentations"),
        (SidebarTab::Playlists, "list-ol", "Playlists"),
        (SidebarTab::Library, "photo-film", "Library"),
        (SidebarTab::Songs, "music", "Songs"),
        (SidebarTab::Bible, "book-bible", "Bible"),
    ];

    let section_buttons = Column::new().spacing(2).padding([6, 6]);
    let section_buttons = sections
        .iter()
        .fold(section_buttons, |col, (st, icon, label)| {
            col.push(section_item(*st, icon, label, tab == *st))
        });

    let detail = detail_pane(w, tab);

    let content = column![section_buttons, divider(), detail]
        .width(RAIL_WIDTH)
        .height(Length::Fill);

    container(content)
        .width(RAIL_WIDTH)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

fn section_item(
    tab: SidebarTab,
    icon: &'static str,
    label: &'static str,
    active: bool,
) -> Element<'static, Message> {
    let icon_el = fa_icon_solid(icon).size(14.0_f32).color(if active {
        theme::ACCENT_ORANGE
    } else {
        theme::TEXT_SECONDARY
    });
    let content = row![
        icon_el,
        text(label).size(12).color(if active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([6, 8]);

    button(content)
        .on_press(Message::SelectLeftSection(tab))
        .width(Length::Fill)
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

fn divider() -> Element<'static, Message> {
    container(Space::new().height(1))
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BORDER_STRONG)),
            ..Default::default()
        })
        .into()
}

fn detail_pane<'a>(w: &'a MainWindow, tab: SidebarTab) -> Element<'a, Message> {
    match tab {
        SidebarTab::Presentations => presentations_detail(w),
        SidebarTab::Playlists => playlist_detail(w),
        SidebarTab::Library => library::view(w),
        SidebarTab::Songs => songs_detail(w),
        SidebarTab::Bible => bible_detail(w),
    }
}

fn presentations_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let header = section_header("PRESENTATIONS");
    let search = search_input(
        "Search…",
        &w.shell.search_query,
        Message::SearchQueryChanged,
    );

    let active_id = w
        .presenting
        .presentation
        .as_ref()
        .or(w.editor.editing.as_ref())
        .map(|p| p.id.as_str());

    let mut list = Column::new().spacing(2).padding([4, 6]);
    let q = w.shell.search_query.to_lowercase();
    for pres in &w.editor.presentations {
        if !q.is_empty() && !pres.name.to_lowercase().contains(&q) {
            continue;
        }
        list = list.push(presentation_row(
            pres,
            active_id.map(|id| id == pres.id).unwrap_or(false),
        ));
    }
    if w.editor.presentations.is_empty() {
        list = list.push(crate::ui::components::empty_state("No presentations"));
    }

    let new_btn = add_button("New Presentation", Message::NewPresentationClicked);

    column![
        header,
        search,
        scrollable(list).height(Length::Fill),
        new_btn,
    ]
    .width(Length::Fill)
    .into()
}

fn presentation_row<'a>(pres: &'a Presentation, active: bool) -> Element<'a, Message> {
    let label = text(&pres.name).size(12).color(if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    });
    let count = text(format!("{} slides", pres.slides.len()))
        .size(10)
        .color(theme::TEXT_MUTED);
    let inner = column![label, count].spacing(2).padding([6, 10]);
    button(inner)
        .on_press(Message::OpenPresentation(pres.id.clone()))
        .width(Length::Fill)
        .style(move |_t: &iced::Theme, status| {
            let bg = if active {
                Color::from_rgba(0.941, 0.216, 0.031, 0.14)
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
                    width: if active { 2.0 } else { 0.0 },
                    radius: 5.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn playlist_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![section_header("PLAYLISTS"), crate::ui::playlist::view(w)]
        .width(Length::Fill)
        .into()
}

fn songs_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![section_header("SONGS"), crate::ui::songs::view(w)]
        .width(Length::Fill)
        .into()
}

fn bible_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![section_header("BIBLE"), crate::ui::bible::view(w)]
        .width(Length::Fill)
        .into()
}
