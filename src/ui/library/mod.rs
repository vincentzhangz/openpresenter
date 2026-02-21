use crate::slides::LibraryAsset;
use crate::ui::components::{add_button, search_input, section_header, truncate};
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length,
    widget::{Column, Space, button, column, container, row, scrollable, text},
};
use iced_font_awesome::fa_icon_solid;

pub const LIBRARY_PANEL_WIDTH: u16 = 240;

pub fn library_panel<'a>(
    assets: &'a [LibraryAsset],
    search_query: &'a str,
    selected_id: Option<&'a str>,
    recently_used_ids: &'a [String],
) -> Element<'a, Message> {
    let header = section_header("MEDIA LIBRARY");

    let search = search_input("Search assets…", search_query, Message::SearchQueryChanged);

    let query_lower = search_query.to_lowercase();

    let recent_assets: Vec<&LibraryAsset> = recently_used_ids
        .iter()
        .filter_map(|id| assets.iter().find(|a| &a.id == id))
        .collect();

    let mut scroll_col = Column::new().padding([4u16, 8u16]).spacing(6);

    if !recent_assets.is_empty() {
        scroll_col = scroll_col.push(section_divider("RECENTLY USED"));
        scroll_col = scroll_col.push(asset_grid(&recent_assets, selected_id));
    }

    let all_filtered: Vec<&LibraryAsset> = assets
        .iter()
        .filter(|a| query_lower.is_empty() || a.name.to_lowercase().contains(&query_lower))
        .collect();

    if all_filtered.is_empty() && assets.is_empty() {
        scroll_col = scroll_col.push(
            container(
                text("No assets imported yet.\nClick \"Import\" to add images or videos.")
                    .size(12)
                    .color(theme::TEXT_MUTED),
            )
            .padding([16, 8])
            .center_x(Length::Fill),
        );
    } else {
        if !recent_assets.is_empty() {
            scroll_col = scroll_col.push(section_divider("ALL ASSETS"));
        }
        if all_filtered.is_empty() {
            scroll_col = scroll_col.push(
                container(text("No results").size(12).color(theme::TEXT_MUTED)).padding([10, 8]),
            );
        } else {
            scroll_col = scroll_col.push(asset_grid(&all_filtered, selected_id));
        }
    }

    let action_bar: Element<Message> = if let Some(selected) = selected_id {
        let asset_id = selected.to_owned();
        container(
            button(
                text("Apply to Current Slide")
                    .size(12)
                    .color(theme::TEXT_PRIMARY),
            )
            .on_press(Message::LibraryApplyToSlide(asset_id))
            .width(Length::Fill)
            .padding([8, 10])
            .style(theme::primary_button),
        )
        .padding([6, 8])
        .into()
    } else {
        Space::new().height(0).into()
    };

    let import_btn = add_button("Import Asset", Message::LibraryImportAsset);

    let content = column![
        header,
        search,
        scrollable(scroll_col).height(Length::Fill),
        action_bar,
        import_btn,
    ]
    .width(LIBRARY_PANEL_WIDTH as f32);

    container(content)
        .width(LIBRARY_PANEL_WIDTH as f32)
        .height(Length::Fill)
        .style(theme::panel_style)
        .into()
}

fn asset_grid<'a>(
    assets: &[&'a LibraryAsset],
    selected_id: Option<&'a str>,
) -> Element<'a, Message> {
    let mut grid = Column::new().spacing(4);
    let mut row_buf: Vec<Element<Message>> = Vec::new();

    for (i, asset) in assets.iter().enumerate() {
        let selected = selected_id.map(|id| id == asset.id).unwrap_or(false);
        row_buf.push(asset_card(asset, selected));

        if row_buf.len() == 2 || i == assets.len() - 1 {
            let mut r = iced::widget::Row::new().spacing(4).width(Length::Fill);
            for card in row_buf.drain(..) {
                r = r.push(card);
            }
            if i == assets.len() - 1 && (i + 1) % 2 != 0 {
                r = r.push(Space::new().width(Length::Fill));
            }
            grid = grid.push(r);
        }
    }

    grid.into()
}

fn asset_card<'a>(asset: &'a LibraryAsset, selected: bool) -> Element<'a, Message> {
    let type_badge_text = if asset.is_image() { "IMG" } else { "VID" };
    let type_color = if asset.is_image() {
        theme::ACCENT_BLUE
    } else {
        theme::WARNING_AMBER
    };

    let icon = container(text(if asset.is_image() { "🖼" } else { "🎬" }).size(22))
        .width(Length::Fill)
        .height(52)
        .center(Length::Fill)
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::BLACK)),
            border: Border {
                color: if selected {
                    theme::ACCENT_BLUE
                } else {
                    theme::BORDER_STRONG
                },
                width: if selected { 2.0 } else { 1.0 },
                radius: 4.0.into(),
            },
            ..Default::default()
        });

    let type_badge = container(text(type_badge_text).size(8).color(type_color))
        .padding([1, 4])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                a: 0.15,
                ..type_color
            })),
            border: Border {
                color: type_color,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..Default::default()
        });

    let name_label = text(truncate(&asset.name, 14))
        .size(10)
        .color(theme::TEXT_SECONDARY);

    let del_id = asset.id.clone();
    let delete_btn = button(fa_icon_solid("xmark").size(11.0).color(theme::TEXT_MUTED))
        .on_press(Message::LibraryDeleteAsset(del_id))
        .padding([0, 3])
        .style(theme::ghost_button);

    let select_id = asset.id.clone();
    let card_inner = column![
        icon,
        row![type_badge, Space::new().width(Length::Fill), delete_btn].align_y(Alignment::Center),
        name_label,
    ]
    .spacing(2);

    button(card_inner)
        .on_press(Message::LibrarySelectAsset(select_id))
        .width(Length::Fill)
        .padding([4, 4])
        .style(move |_t: &iced::Theme, status| {
            let bg = if selected {
                Color::from_rgba(0.204, 0.471, 0.965, 0.15)
            } else if matches!(status, iced::widget::button::Status::Hovered) {
                theme::BG_HOVER
            } else {
                theme::TRANSPARENT
            };
            iced::widget::button::Style {
                background: Some(Background::Color(bg)),
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn section_divider<'a>(label: &'a str) -> Element<'a, Message> {
    row![
        container(Space::new().width(Length::Fill).height(1.0))
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(theme::BORDER_PANEL)),
                ..Default::default()
            })
            .width(Length::Fill),
        Space::new().width(6),
        text(label).size(9).color(theme::TEXT_MUTED),
        Space::new().width(6),
        container(Space::new().width(Length::Fill).height(1.0))
            .style(|_: &iced::Theme| iced::widget::container::Style {
                background: Some(Background::Color(theme::BORDER_PANEL)),
                ..Default::default()
            })
            .width(Length::Fill),
    ]
    .align_y(Alignment::Center)
    .padding([4, 0])
    .into()
}
