use crate::domain::props::{Look, Mask, Prop, PropContent, PropManager};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Task,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};

/// Messages owned by the Props feature module (see `AGENTS.md`).
///
/// `SetMask` and `TogglePropsPanel` stay as root variants (they are global
/// rendering/navigation controls, not props-feature state).
#[derive(Debug, Clone)]
pub enum Message {
    Toggle(String),
    Remove(String),
    AddText,
    AddImage,
    NewNameChanged(String),
    LowerThirdTitleChanged(String),
    LowerThirdSubtitleChanged(String),
    CreateLowerThird,
    ApplyLook(String),
    RemoveLook(String),
    SaveLook,
    LookNameChanged(String),
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Props(msg)
}

/// Render the props panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    props_panel(
        &w.props.manager,
        &w.props.new_prop_name,
        &w.props.new_look_name,
        &w.props.lower_third_title,
        &w.props.lower_third_subtitle,
    )
}

/// Dispatch a props message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::Toggle(id) => prop_toggle(w, id),
        Message::Remove(id) => prop_remove(w, id),
        Message::AddText => prop_add_text(w),
        Message::AddImage => prop_add_image(w),
        Message::NewNameChanged(name) => new_name_changed(w, name),
        Message::LowerThirdTitleChanged(t) => lower_third_title_changed(w, t),
        Message::LowerThirdSubtitleChanged(s) => lower_third_subtitle_changed(w, s),
        Message::CreateLowerThird => create_lower_third(w),
        Message::ApplyLook(id) => apply_look(w, id),
        Message::RemoveLook(id) => remove_look(w, id),
        Message::SaveLook => save_look(w),
        Message::LookNameChanged(name) => look_name_changed(w, name),
    }
}

pub(crate) fn prop_toggle(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.props.manager.toggle_prop(&id);
    Task::none()
}

pub(crate) fn prop_remove(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.props.manager.remove_prop(&id);
    Task::none()
}

pub(crate) fn prop_add_text(w: &mut MainWindow) -> Task<RootMessage> {
    let name = if w.props.new_prop_name.trim().is_empty() {
        "New Text Prop".to_string()
    } else {
        w.props.new_prop_name.trim().to_string()
    };
    let prop = Prop {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        content: PropContent::Text {
            text: "Text".to_string(),
            font_size: 32.0,
            color: [1.0, 1.0, 1.0, 1.0],
            bold: false,
            italic: false,
        },
        x: 0.1,
        y: 0.1,
        width: 0.8,
        height: 0.1,
        visible: true,
    };
    w.props.manager.add_prop(prop);
    w.props.new_prop_name.clear();
    Task::none()
}

pub(crate) fn prop_add_image(w: &mut MainWindow) -> Task<RootMessage> {
    let path = rfd::FileDialog::new()
        .set_title("Select Image for Prop")
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
        .pick_file();

    if let Some(p) = path {
        let filename = p
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Image")
            .to_string();
        let name = if w.props.new_prop_name.trim().is_empty() {
            filename
        } else {
            w.props.new_prop_name.trim().to_string()
        };
        let prop = Prop {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            content: PropContent::Image {
                path: p.to_string_lossy().into_owned(),
            },
            x: 0.80,
            y: 0.03,
            width: 0.15,
            height: 0.10,
            visible: true,
        };
        w.props.manager.add_prop(prop);
        w.props.new_prop_name.clear();
    }
    Task::none()
}

pub(crate) fn new_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.props.new_prop_name = name;
    Task::none()
}

pub(crate) fn lower_third_title_changed(w: &mut MainWindow, title: String) -> Task<RootMessage> {
    w.props.lower_third_title = title;
    Task::none()
}

pub(crate) fn lower_third_subtitle_changed(
    w: &mut MainWindow,
    subtitle: String,
) -> Task<RootMessage> {
    w.props.lower_third_subtitle = subtitle;
    Task::none()
}

pub(crate) fn create_lower_third(w: &mut MainWindow) -> Task<RootMessage> {
    let props = crate::domain::props::Prop::new_lower_third(
        &w.props.lower_third_title,
        &w.props.lower_third_subtitle,
    );
    for prop in props {
        w.props.manager.add_prop(prop);
    }
    Task::none()
}

pub(crate) fn apply_look(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.props.manager.apply_look(&id);
    Task::none()
}

pub(crate) fn remove_look(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.props.manager.remove_look(&id);
    Task::none()
}

pub(crate) fn save_look(w: &mut MainWindow) -> Task<RootMessage> {
    let name = if w.props.new_look_name.trim().is_empty() {
        format!("Look {}", w.props.manager.looks.len() + 1)
    } else {
        w.props.new_look_name.trim().to_string()
    };
    w.props.manager.save_look(name);
    w.props.new_look_name.clear();
    Task::none()
}

pub(crate) fn look_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.props.new_look_name = name;
    Task::none()
}

pub(crate) fn set_mask(w: &mut MainWindow, mask: Mask) -> Task<RootMessage> {
    w.props.manager.active_mask = mask;
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.props.panel_open = !w.props.panel_open;
    Task::none()
}

pub fn props_panel<'a>(
    manager: &'a PropManager,
    new_prop_name: &'a str,
    new_look_name: &'a str,
    lower_third_title: &'a str,
    lower_third_subtitle: &'a str,
) -> Element<'a, RootMessage> {
    let props_header = row![text("Props").size(14.0), Space::new().width(Length::Fill),]
        .align_y(Alignment::Center);

    let mut props_list = column![].spacing(2);
    for prop in &manager.props {
        props_list = props_list.push(prop_row(prop));
    }

    let add_prop_row = row![
        text_input("New prop name…", new_prop_name)
            .on_input(move |v| wrap(Message::NewNameChanged(v)))
            .padding([3, 6])
            .width(140),
        button(text("+ Text").size(11.0))
            .on_press(wrap(Message::AddText))
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("+ Image").size(11.0))
            .on_press(wrap(Message::AddImage))
            .padding([3, 6])
            .style(theme::ghost_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let lt_section = column![
        text("Lower Third").size(12.0),
        row![
            text_input("Title…", lower_third_title)
                .on_input(move |v| wrap(Message::LowerThirdTitleChanged(v)))
                .padding([3, 6])
                .width(120),
            text_input("Subtitle…", lower_third_subtitle)
                .on_input(move |v| wrap(Message::LowerThirdSubtitleChanged(v)))
                .padding([3, 6])
                .width(120),
            button(text("Create").size(11.0))
                .on_press(wrap(Message::CreateLowerThird))
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
            .on_press(RootMessage::SetMask(Mask::None))
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("Letterbox").size(11.0))
            .on_press(RootMessage::SetMask(Mask::Letterbox { bar_fraction: 0.1 }))
            .padding([3, 6])
            .style(theme::ghost_button),
        button(text("Oval").size(11.0))
            .on_press(RootMessage::SetMask(Mask::Oval { feather: 0.05 }))
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
            .on_input(move |v| wrap(Message::LookNameChanged(v)))
            .padding([3, 6])
            .width(140),
        button(text("Save Look").size(11.0))
            .on_press(wrap(Message::SaveLook))
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

fn prop_row<'a>(prop: &'a Prop) -> Element<'a, RootMessage> {
    let vis_label = if prop.visible { "On" } else { "Off" };
    let type_label = match &prop.content {
        PropContent::Text { .. } => "T",
        PropContent::Image { .. } => "Img",
        PropContent::Rectangle { .. } => "R",
    };

    row![
        button(text(vis_label).size(13.0))
            .on_press(wrap(Message::Toggle(prop.id.clone())))
            .padding([2, 6])
            .style(theme::ghost_button),
        text(type_label).size(11.0).width(16),
        text(&prop.name).size(12.0).width(Length::Fill),
        button(text("X").size(10.0))
            .on_press(wrap(Message::Remove(prop.id.clone())))
            .padding([2, 5])
            .style(theme::danger_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn look_row<'a>(look: &'a Look) -> Element<'a, RootMessage> {
    row![
        text(&look.name).size(12.0).width(Length::Fill),
        button(text("Apply").size(11.0))
            .on_press(wrap(Message::ApplyLook(look.id.clone())))
            .padding([2, 6])
            .style(theme::primary_button),
        button(text("X").size(10.0))
            .on_press(wrap(Message::RemoveLook(look.id.clone())))
            .padding([2, 5])
            .style(theme::danger_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}
