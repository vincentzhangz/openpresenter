use crate::slides::{LibraryAsset, Presentation};
use crate::ui::components::{add_button, search_input, section_header, tab_bar, tab_btn};
use crate::ui::library;
use crate::ui::messages::{Message, SidebarTab};
use crate::ui::theme;
use iced::{
    Element, Length,
    widget::{Column, button, column, container, scrollable, text},
};

pub const SIDEBAR_WIDTH: u16 = 240;

#[allow(clippy::too_many_arguments)]
pub fn sidebar<'a>(
    presentations: &'a [Presentation],
    search_query: &'a str,
    active_id: Option<&'a str>,
    sidebar_tab: SidebarTab,
    lib_assets: &'a [LibraryAsset],
    selected_asset_id: Option<&'a str>,
    recently_used_ids: &'a [String],
) -> Element<'a, Message> {
    let tab_bar_el = tab_bar(vec![
        tab_btn(
            "Slides",
            sidebar_tab == SidebarTab::Presentations,
            Message::SwitchSidebarTab(SidebarTab::Presentations),
        ),
        tab_btn(
            "Library",
            sidebar_tab == SidebarTab::Library,
            Message::SwitchSidebarTab(SidebarTab::Library),
        ),
        tab_btn(
            "Songs",
            sidebar_tab == SidebarTab::Songs,
            Message::SwitchSidebarTab(SidebarTab::Songs),
        ),
        tab_btn(
            "Bible",
            sidebar_tab == SidebarTab::Bible,
            Message::SwitchSidebarTab(SidebarTab::Bible),
        ),
    ]);

    if sidebar_tab == SidebarTab::Library {
        return column![
            tab_bar_el,
            library::library_panel(
                lib_assets,
                search_query,
                selected_asset_id,
                recently_used_ids
            ),
        ]
        .width(SIDEBAR_WIDTH as f32)
        .height(Length::Fill)
        .into();
    }
    if sidebar_tab == SidebarTab::Songs {
        return container(tab_bar_el)
            .width(240)
            .height(Length::Fill)
            .style(theme::dark_panel_style)
            .into();
    }
    if sidebar_tab == SidebarTab::Bible {
        return container(tab_bar_el)
            .width(240)
            .height(Length::Fill)
            .style(theme::dark_panel_style)
            .into();
    }
    let header = section_header("PRESENTATIONS");

    let search = search_input("Search…", search_query, Message::SearchQueryChanged);

    let mut list = Column::new().spacing(2).padding([4u16, 8u16]);

    if presentations.is_empty() {
        list = list.push(crate::ui::components::empty_state("No presentations"));
    } else {
        let query_lower = search_query.to_lowercase();
        for pres in presentations {
            if !query_lower.is_empty() && !pres.name.to_lowercase().contains(&query_lower) {
                continue;
            }
            let is_active = active_id.map(|id| id == pres.id).unwrap_or(false);
            let item = presentation_item(pres, is_active);
            list = list.push(item);
        }
    }

    let new_btn = add_button("New Presentation", Message::NewPresentationClicked);

    let content = column![
        tab_bar_el,
        header,
        search,
        scrollable(list).height(Length::Fill),
        new_btn,
    ]
    .width(SIDEBAR_WIDTH as f32);

    container(content)
        .width(SIDEBAR_WIDTH as f32)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

fn presentation_item<'a>(pres: &'a Presentation, active: bool) -> Element<'a, Message> {
    let label = text(&pres.name).size(13).color(if active {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    });

    let slide_count = text(format!("{} slides", pres.slides.len()))
        .size(10)
        .color(theme::TEXT_MUTED);

    let item_content = column![label, slide_count].spacing(2).padding([5, 10]);

    button(item_content)
        .on_press(Message::OpenPresentation(pres.id.clone()))
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme, status| {
            let bg = if active {
                iced::Color::from_rgba(0.204, 0.471, 0.965, 0.18)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: iced::Border {
                    color: if active {
                        theme::ACCENT_BLUE
                    } else {
                        theme::TRANSPARENT
                    },
                    width: 0.0,
                    radius: 6.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        })
        .into()
}
