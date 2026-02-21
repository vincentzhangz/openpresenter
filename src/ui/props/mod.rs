use crate::slides::props::{Look, Mask, Prop, PropContent, PropManager};
use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};

pub fn props_panel<'a>(
    manager: &'a PropManager,
    new_prop_name: &'a str,
    new_look_name: &'a str,
    lower_third_title: &'a str,
    lower_third_subtitle: &'a str,
) -> Element<'a, Message> {
    let props_header = row![text("Props").size(14.0), Space::new().width(Length::Fill),]
        .align_y(Alignment::Center);

    let mut props_list = column![].spacing(2);
    for prop in &manager.props {
        props_list = props_list.push(prop_row(prop));
    }

    let add_prop_row = row![
        text_input("New prop name…", new_prop_name)
            .on_input(Message::PropNewNameChanged)
            .padding([3, 6])
            .width(140),
        button(text("+ Text").size(11.0))
            .on_press(Message::PropAddText)
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("+ Image").size(11.0))
            .on_press(Message::PropAddImage)
            .padding([3, 6])
            .style(theme::ghost_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let lt_section = column![
        text("Lower Third").size(12.0),
        row![
            text_input("Title…", lower_third_title)
                .on_input(Message::LowerThirdTitleChanged)
                .padding([3, 6])
                .width(120),
            text_input("Subtitle…", lower_third_subtitle)
                .on_input(Message::LowerThirdSubtitleChanged)
                .padding([3, 6])
                .width(120),
            button(text("Create").size(11.0))
                .on_press(Message::CreateLowerThird)
                .padding([3, 8])
                .style(theme::primary_button),
        ]
        .spacing(4),
    ]
    .spacing(4);

    let mask_label = text(format!("Mask: {}", manager.active_mask)).size(12.0);
    let mask_row = row![
        mask_label,
        Space::new().width(Length::Fill),
        button(text("None").size(11.0))
            .on_press(Message::SetMask(Mask::None))
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("Letterbox").size(11.0))
            .on_press(Message::SetMask(Mask::Letterbox { bar_fraction: 0.1 }))
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("Oval").size(11.0))
            .on_press(Message::SetMask(Mask::Oval { feather: 0.05 }))
            .padding([3, 6])
            .style(theme::ghost_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let looks_header = row![text("Looks").size(14.0), Space::new().width(Length::Fill),]
        .align_y(Alignment::Center);

    let mut looks_list = column![].spacing(2);
    for look in &manager.looks {
        looks_list = looks_list.push(look_row(look));
    }

    let save_look_row = row![
        text_input("Look name…", new_look_name)
            .on_input(Message::LookNameChanged)
            .padding([3, 6])
            .width(140),
        button(text("Save Look").size(11.0))
            .on_press(Message::SaveLook)
            .padding([3, 8])
            .style(theme::primary_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    container(
        scrollable(
            column![
                props_header,
                props_list,
                add_prop_row,
                lt_section,
                mask_row,
                looks_header,
                looks_list,
                save_look_row,
            ]
            .spacing(8)
            .padding(12),
        )
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn prop_row<'a>(prop: &'a Prop) -> Element<'a, Message> {
    let vis_label = if prop.visible { "On" } else { "Off" };
    let type_label = match &prop.content {
        PropContent::Text { .. } => "T",
        PropContent::Image { .. } => "Img",
        PropContent::Rectangle { .. } => "R",
    };

    row![
        button(text(vis_label).size(13.0))
            .on_press(Message::PropToggle(prop.id.clone()))
            .padding([2, 6])
            .style(theme::ghost_button),
        text(type_label).size(11.0).width(16),
        text(&prop.name).size(12.0).width(Length::Fill),
        button(text("X").size(10.0))
            .on_press(Message::PropRemove(prop.id.clone()))
            .padding([2, 5])
            .style(theme::danger_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn look_row<'a>(look: &'a Look) -> Element<'a, Message> {
    row![
        text(&look.name).size(12.0).width(Length::Fill),
        button(text("Apply").size(11.0))
            .on_press(Message::ApplyLook(look.id.clone()))
            .padding([2, 6])
            .style(theme::primary_button),
        button(text("X").size(10.0))
            .on_press(Message::RemoveLook(look.id.clone()))
            .padding([2, 5])
            .style(theme::danger_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}
