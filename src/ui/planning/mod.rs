use crate::slides::{Presentation, ServiceItem, ServicePlan, Song};
use crate::ui::components::{divider, live_badge};
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Background, Color, Element, Length, Padding,
    widget::{Column, Space, button, column, container, row, scrollable, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

#[allow(clippy::too_many_arguments)]
pub fn planning_panel<'a>(
    plans: &'a [ServicePlan],
    editing: Option<&'a ServicePlan>,
    plan_name_edit: &'a str,
    presentations: &'a [Presentation],
    songs: &'a [Song],
    active_plan_id: Option<&'a str>,
    service_item_index: usize,
) -> Element<'a, Message> {
    let list = plan_list(plans, editing.map(|p| p.id.as_str()), active_plan_id);
    let editor: Element<'a, Message> = if let Some(plan) = editing {
        plan_editor(
            plan,
            plan_name_edit,
            presentations,
            songs,
            active_plan_id,
            service_item_index,
        )
    } else {
        container(
            text("Select a service plan or create a new one")
                .size(14)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::canvas_bg_style)
        .into()
    };

    row![list, editor]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn plan_list<'a>(
    plans: &'a [ServicePlan],
    active_id: Option<&'a str>,
    active_plan_id: Option<&'a str>,
) -> Element<'a, Message> {
    let mut list_col = Column::new().spacing(2).padding([4u16, 0u16]);

    if plans.is_empty() {
        list_col = list_col.push(
            container(text("No plans yet").size(13).color(theme::TEXT_MUTED))
                .padding([12u16, 14u16]),
        );
    } else {
        for plan in plans {
            let is_active_edit = active_id == Some(plan.id.as_str());
            let is_live = active_plan_id == Some(plan.id.as_str());
            let sty = if is_active_edit {
                theme::primary_button
            } else {
                theme::ghost_button
            };

            let live_el: Element<'_, Message> = if is_live {
                live_badge("LIVE")
            } else {
                Space::new().width(0).into()
            };

            let item_count = text(format!("{} items", plan.items.len()))
                .size(11)
                .color(theme::TEXT_MUTED);

            let card = column![
                row![
                    text(plan.name.clone()).size(13),
                    Space::new().width(Length::Fill),
                    live_el
                ]
                .align_y(Alignment::Center),
                item_count,
            ]
            .spacing(2);

            list_col = list_col.push(
                button(card)
                    .on_press(Message::OpenServicePlan(plan.id.clone()))
                    .padding([8u16, 12u16])
                    .width(Length::Fill)
                    .style(sty),
            );
        }
    }

    container(
        column![
            scrollable(list_col)
                .height(Length::Fill)
                .width(Length::Fill),
            container(
                button(text("+ New Plan").size(13))
                    .on_press(Message::NewServicePlan)
                    .padding([8u16, 14u16])
                    .width(Length::Fill)
                    .style(theme::primary_button),
            )
            .padding([8u16, 10u16])
            .width(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(240)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn plan_editor<'a>(
    plan: &'a ServicePlan,
    plan_name_edit: &'a str,
    presentations: &'a [Presentation],
    songs: &'a [Song],
    active_plan_id: Option<&'a str>,
    service_item_index: usize,
) -> Element<'a, Message> {
    let is_live = active_plan_id == Some(plan.id.as_str());

    let name_row = row![
        text_input("Plan name", plan_name_edit)
            .on_input(Message::ServicePlanNameChanged)
            .padding([6u16, 10u16])
            .size(15)
            .width(Length::Fill),
    ]
    .padding([10u16, 16u16]);

    let live_label: Element<'_, Message> = if is_live {
        text(format!(
            "LIVE  {}/{}",
            service_item_index + 1,
            plan.items.len()
        ))
        .size(13)
        .color(theme::LIVE_GREEN)
        .into()
    } else {
        Space::new().width(0).into()
    };

    let start_stop: Element<'_, Message> = if is_live {
        button(text("End Service").size(12))
            .on_press(Message::EndService)
            .style(theme::danger_button)
            .padding([6u16, 14u16])
            .into()
    } else {
        button(text("Start Service").size(12))
            .on_press(Message::StartService)
            .style(theme::primary_button)
            .padding([6u16, 14u16])
            .into()
    };

    let actions = row![
        button(text("Save").size(13))
            .on_press(Message::SaveServicePlan)
            .style(theme::primary_button)
            .padding([6u16, 14u16]),
        start_stop,
        live_label,
        Space::new().width(Length::Fill),
        button(text("Delete Plan").size(12))
            .on_press(Message::DeleteServicePlanClicked(plan.id.clone()))
            .style(theme::danger_button)
            .padding([6u16, 14u16]),
    ]
    .spacing(8)
    .padding(Padding {
        top: 0.0,
        right: 16.0,
        bottom: 10.0,
        left: 16.0,
    })
    .align_y(Alignment::Center);

    let total = plan.items.len();
    let mut items_col = Column::new().spacing(6).padding(Padding {
        top: 0.0,
        right: 16.0,
        bottom: 8.0,
        left: 16.0,
    });

    if plan.items.is_empty() {
        items_col = items_col.push(
            container(
                text("No items yet — add from the pickers below")
                    .size(13)
                    .color(theme::TEXT_MUTED),
            )
            .padding([16u16, 0u16]),
        );
    } else {
        for (i, item) in plan.items.iter().enumerate() {
            let is_current = is_live && i == service_item_index;
            items_col = items_col.push(item_card(i, item, total, is_current));
        }
    }

    let add_header = row![
        button(text("+ Header").size(12))
            .on_press(Message::AddHeaderItem)
            .style(theme::secondary_button)
            .padding([5u16, 10u16]),
        button(text("+ Blank").size(12))
            .on_press(Message::AddBlankItem)
            .style(theme::secondary_button)
            .padding([5u16, 10u16]),
    ]
    .spacing(6)
    .padding([6u16, 16u16]);

    let pres_label =
        container(text("Presentations").size(12).color(theme::TEXT_MUTED)).padding(Padding {
            top: 6.0,
            right: 16.0,
            bottom: 2.0,
            left: 16.0,
        });

    let mut pres_col = Column::new().spacing(2).padding([0u16, 16u16]);
    for p in presentations {
        pres_col = pres_col.push(
            button(text(p.name.clone()).size(12))
                .on_press(Message::AddPresentationItem(p.id.clone()))
                .padding([4u16, 8u16])
                .width(Length::Fill)
                .style(theme::ghost_button),
        );
    }

    let song_label = container(text("Songs").size(12).color(theme::TEXT_MUTED)).padding(Padding {
        top: 6.0,
        right: 16.0,
        bottom: 2.0,
        left: 16.0,
    });

    let mut song_col = Column::new().spacing(2).padding([0u16, 16u16]);
    for s in songs {
        song_col = song_col.push(
            button(text(s.title.clone()).size(12))
                .on_press(Message::AddSongItem(s.id.clone()))
                .padding([4u16, 8u16])
                .width(Length::Fill)
                .style(theme::ghost_button),
        );
    }

    let add_section =
        scrollable(column![pres_label, pres_col, song_label, song_col].width(Length::Fill))
            .height(200);

    container(
        column![
            name_row,
            divider(),
            actions,
            divider(),
            scrollable(items_col).height(Length::Fill),
            divider(),
            add_header,
            add_section,
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::canvas_bg_style)
    .into()
}

fn item_card<'a>(
    i: usize,
    item: &'a ServiceItem,
    total: usize,
    is_current: bool,
) -> Element<'a, Message> {
    let badge_color = match item {
        ServiceItem::Presentation { .. } => Color::from_rgb(0.29, 0.56, 0.89),
        ServiceItem::Song { .. } => Color::from_rgb(0.56, 0.36, 0.89),
        ServiceItem::MediaCue { .. } => Color::from_rgb(0.89, 0.56, 0.29),
        ServiceItem::Header { .. } => Color::from_rgb(0.36, 0.72, 0.50),
        ServiceItem::Blank => Color::from_rgb(0.45, 0.45, 0.45),
    };

    let badge = container(text(item.type_label()).size(9).color(Color::WHITE))
        .padding([2u16, 5u16])
        .style(move |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(badge_color)),
            border: iced::Border {
                radius: 3.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let name_el: Element<'_, Message> = match item {
        ServiceItem::Header { text: t } => text_input("Section header...", t)
            .on_input(move |v| Message::ServiceItemHeaderChanged(i, v))
            .padding([3u16, 6u16])
            .size(13)
            .into(),
        _ => text(item.display_name()).size(13).into(),
    };

    let current_indicator: Element<'_, Message> = if is_current {
        fa_icon_solid("caret-right")
            .size(12.0)
            .color(theme::LIVE_GREEN)
            .into()
    } else {
        Space::new().width(12).into()
    };

    let up_el: Element<'_, Message> = if i > 0 {
        button(text("^").size(11))
            .on_press(Message::MoveServiceItemUp(i))
            .style(theme::secondary_button)
            .padding([3u16, 7u16])
            .into()
    } else {
        Space::new().width(26).into()
    };

    let down_el: Element<'_, Message> = if i + 1 < total {
        button(text("v").size(11))
            .on_press(Message::MoveServiceItemDown(i))
            .style(theme::secondary_button)
            .padding([3u16, 7u16])
            .into()
    } else {
        Space::new().width(26).into()
    };

    let jump_el: Element<'_, Message> = if is_current {
        Space::new().width(0).into()
    } else {
        button(text("Go").size(10))
            .on_press(Message::ServiceJumpToItem(i))
            .style(theme::ghost_button)
            .padding([3u16, 7u16])
            .into()
    };

    let dup = button(text("⧉").size(11))
        .on_press(Message::DuplicateServiceItem(i))
        .style(theme::secondary_button)
        .padding([3u16, 7u16]);

    let del = button(text("x").size(11))
        .on_press(Message::RemoveServiceItem(i))
        .style(theme::danger_button)
        .padding([3u16, 7u16]);

    let card_bg = if is_current {
        |_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(0.20, 0.78, 0.35, 0.08))),
            border: iced::Border {
                color: Color::from_rgba(0.20, 0.78, 0.35, 0.30),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    } else {
        theme::dark_panel_style
    };

    container(
        row![
            current_indicator,
            badge,
            Space::new().width(8),
            name_el,
            Space::new().width(Length::Fill),
            jump_el,
            up_el,
            down_el,
            dup,
            del,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([6u16, 8u16]),
    )
    .width(Length::Fill)
    .style(card_bg)
    .into()
}
