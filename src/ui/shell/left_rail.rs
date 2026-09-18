use crate::domain::Presentation;
use crate::ui::components::{add_button, search_input, section_header};
use crate::ui::library;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, RailContextTarget, SidebarTab};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{
        Column, Space, button, column, container, mouse_area, pin, row, scrollable, stack, text,
    },
};
use iced_font_awesome::fa_icon_solid;

pub const RAIL_WIDTH: f32 = 260.0;

/// Left rail: collapsible outline of libraries + playlists,
/// with a detail list underneath.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    if w.shell.current_mode == crate::ui::messages::ViewMode::Edit
        && let Some(ref pres) = w.editor.editing
    {
        return crate::ui::editor::slide_list::slide_list(
            pres,
            w.editor.selected_slide_index,
            w.layer.selected_index,
        );
    }

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

    let tracked_base: Element<'a, Message> = mouse_area(
        container(content)
            .width(RAIL_WIDTH)
            .height(Length::Fill)
            .style(theme::panel_style),
    )
    .on_move(Message::RailCursorMoved)
    .into();

    if let Some(ref target) = w.shell.rail_context_target {
        let pos = w
            .shell
            .rail_cursor_pos
            .unwrap_or(iced::Point::new(40.0, 100.0));
        let backdrop: Element<'a, Message> = mouse_area(container(
            Space::new().width(Length::Fill).height(Length::Fill),
        ))
        .on_press(Message::HideRailContextMenu)
        .on_right_press(Message::HideRailContextMenu)
        .into();

        let menu_x = (pos.x + 2.0).clamp(6.0, RAIL_WIDTH - 180.0);
        let menu_y = (pos.y + 2.0).max(6.0);

        let menu_overlay: Element<'a, Message> = pin(rail_context_menu_panel(target))
            .x(menu_x)
            .y(menu_y)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        stack![tracked_base, backdrop, menu_overlay]
            .width(RAIL_WIDTH)
            .height(Length::Fill)
            .into()
    } else {
        tracked_base
    }
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

    let import_btn = button(
        row![
            fa_icon_solid("file-import").size(11.0_f32),
            text("Import .opp").size(11),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(Message::ImportExport(
        crate::ui::import_export::Message::ImportOpp,
    ))
    .padding([5, 8])
    .style(theme::ghost_button);

    let export_btn = if w.editor.editing.is_some() {
        button(
            row![
                fa_icon_solid("file-export").size(11.0_f32),
                text("Export .opp").size(11),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ImportExport(
            crate::ui::import_export::Message::ExportOpp,
        ))
        .padding([5, 8])
        .style(theme::ghost_button)
    } else {
        button(
            row![
                fa_icon_solid("file-export").size(11.0_f32),
                text("Export .opp").size(11).color(theme::TEXT_MUTED),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .style(theme::ghost_button)
    };

    let actions = row![import_btn, Space::new().width(Length::Fill), export_btn]
        .padding([2, 4])
        .align_y(Alignment::Center);

    column![
        header,
        search,
        scrollable(list).height(Length::Fill),
        actions,
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
    let btn = button(inner)
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
        });

    mouse_area(btn)
        .on_right_press(Message::ShowRailContextMenu(
            RailContextTarget::Presentation(pres.id.clone()),
        ))
        .into()
}

fn playlist_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![
        section_header("PLAYLISTS"),
        crate::ui::playlist::list_view(w)
    ]
    .width(Length::Fill)
    .into()
}

fn songs_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![section_header("SONGS"), crate::ui::songs::list_view(w)]
        .width(Length::Fill)
        .into()
}

fn bible_detail<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    column![section_header("BIBLE"), crate::ui::bible::list_view(w)]
        .width(Length::Fill)
        .into()
}

fn rail_context_menu_panel<'a>(target: &RailContextTarget) -> Element<'a, Message> {
    let menu_bg = Color::from_rgba(0.11, 0.12, 0.14, 0.98);
    let menu_shadow = iced::Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
        offset: iced::Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    };

    fn menu_item<'a>(
        icon: &'static str,
        label: &'static str,
        on_press: Message,
        is_danger: bool,
    ) -> Element<'a, Message> {
        let icon_color = if is_danger {
            theme::DANGER_RED
        } else {
            theme::TEXT_PRIMARY
        };
        let label_color = if is_danger {
            theme::DANGER_RED
        } else {
            theme::TEXT_PRIMARY
        };

        button(
            row![
                fa_icon_solid(icon).size(11.0_f32).color(icon_color),
                Space::new().width(8),
                text(label).size(12).color(label_color),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(on_press)
        .width(Length::Fill)
        .padding([7, 10])
        .style(
            move |_t: &iced::Theme, status| iced::widget::button::Style {
                background: Some(Background::Color(
                    if matches!(status, iced::widget::button::Status::Hovered) {
                        if is_danger {
                            Color::from_rgba(0.9, 0.2, 0.2, 0.18)
                        } else {
                            theme::BG_HOVER
                        }
                    } else {
                        theme::TRANSPARENT
                    },
                )),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .into()
    }

    fn menu_divider<'a>() -> Element<'a, Message> {
        container(Space::new().height(1).width(Length::Fill))
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(theme::BORDER_PANEL)),
                ..Default::default()
            })
            .padding([3, 0])
            .into()
    }

    let items: Vec<Element<'a, Message>> = match target {
        RailContextTarget::Presentation(id) => {
            let id = id.clone();
            vec![
                menu_item(
                    "play",
                    "Present",
                    Message::OpenPresentation(id.clone()),
                    false,
                ),
                menu_item(
                    "pen-to-square",
                    "Edit Presentation",
                    Message::EditPresentation(id.clone()),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "copy",
                    "Duplicate",
                    Message::DuplicatePresentation(id.clone()),
                    false,
                ),
                menu_item(
                    "file-export",
                    "Export .opp",
                    Message::ExportPresentation(id.clone()),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "trash",
                    "Delete",
                    Message::DeletePresentationClicked(id),
                    true,
                ),
            ]
        }
        RailContextTarget::Playlist(id) => {
            let id = id.clone();
            vec![
                menu_item(
                    "folder-open",
                    "Open Playlist",
                    Message::Playlist(crate::ui::playlist::Message::Open(id.clone())),
                    false,
                ),
                menu_item(
                    "play",
                    "Start Service",
                    Message::Playlist(crate::ui::playlist::Message::StartPlan(id.clone())),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "copy",
                    "Duplicate",
                    Message::Playlist(crate::ui::playlist::Message::Duplicate(id.clone())),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "trash",
                    "Delete",
                    Message::Playlist(crate::ui::playlist::Message::DeleteClicked(id)),
                    true,
                ),
            ]
        }
        RailContextTarget::LibraryAsset(id) => {
            let id = id.clone();
            vec![
                menu_item(
                    "image",
                    "Apply to Slide",
                    Message::Library(crate::ui::library::Message::ApplyToSlide(id.clone())),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "trash",
                    "Delete Asset",
                    Message::Library(crate::ui::library::Message::DeleteAsset(id)),
                    true,
                ),
            ]
        }
        RailContextTarget::Song(id) => {
            let id = id.clone();
            vec![
                menu_item(
                    "pen-to-square",
                    "Open Song",
                    Message::Songs(crate::ui::songs::Message::Open(id.clone())),
                    false,
                ),
                menu_item(
                    "layer-group",
                    "To Presentation",
                    Message::SongToPresentation(id.clone()),
                    false,
                ),
                menu_divider(),
                menu_item(
                    "trash",
                    "Delete Song",
                    Message::Songs(crate::ui::songs::Message::DeleteClicked(id)),
                    true,
                ),
            ]
        }
    };

    let col = Column::with_children(items).spacing(2);

    container(col)
        .width(170)
        .padding([6, 6])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(menu_bg)),
            border: Border {
                color: theme::BORDER_STRONG,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: menu_shadow,
            ..Default::default()
        })
        .into()
}
