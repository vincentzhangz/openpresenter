use crate::domain::{Playlist, PlaylistItem, Presentation, Song};
use crate::ui::components::{divider, live_badge};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, ViewMode};
use crate::ui::theme;
use iced::{
    Alignment, Background, Color, Element, Length, Padding, Task,
    widget::{Column, Space, button, column, container, row, scrollable, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

/// Messages owned by the Playlist feature module.
///
/// The root [`RootMessage`] enum wraps this in `RootMessage::Playlist(..)`, so
/// every variant here is scoped to this feature instead of bloating the global
/// enum. See `AGENTS.md` for the nested-message convention.
#[derive(Debug, Clone)]
pub enum Message {
    New,
    Open(String),
    Save,
    DeleteClicked(String),
    ConfirmDelete,
    CancelDelete,
    NameChanged(String),
    AddPresentation(String),
    AddSong(String),
    AddHeader,
    AddBlank,
    ItemHeaderChanged(usize, String),
    RemoveItem(usize),
    MoveItemUp(usize),
    MoveItemDown(usize),
    DuplicateItem(usize),
    Start,
    Next,
    Prev,
    JumpTo(usize),
    End,
}

/// Wrap a playlist message into the root message type.
fn wrap(msg: Message) -> RootMessage {
    RootMessage::Playlist(msg)
}

/// Render the playlist (service-plan) panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let active_plan_id = w.service.active.as_ref().map(|p| p.id.as_str());
    planning_panel(
        &w.service.plans,
        w.service.editing.as_ref(),
        &w.service.name_edit,
        &w.editor.presentations,
        &w.song.songs,
        active_plan_id,
        w.service.item_index,
    )
}

/// Dispatch a playlist message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::New => new_plan(w),
        Message::Open(id) => open_plan(w, id),
        Message::Save => save_playlist(w),
        Message::DeleteClicked(id) => delete_clicked(w, id),
        Message::ConfirmDelete => confirm_delete(w),
        Message::CancelDelete => cancel_delete(w),
        Message::NameChanged(name) => plan_name_changed(w, name),
        Message::AddPresentation(id) => add_presentation_item(w, id),
        Message::AddSong(id) => add_song_item(w, id),
        Message::AddHeader => add_header(w),
        Message::AddBlank => add_blank(w),
        Message::ItemHeaderChanged(i, t) => item_header_changed(w, i, t),
        Message::RemoveItem(i) => remove_item(w, i),
        Message::MoveItemUp(i) => move_item_up(w, i),
        Message::MoveItemDown(i) => move_item_down(w, i),
        Message::DuplicateItem(i) => duplicate_item(w, i),
        Message::Start => start_service(w),
        Message::Next => service_next(w),
        Message::Prev => service_prev(w),
        Message::JumpTo(i) => jump_to_item(w, i),
        Message::End => end_service(w),
    }
}

pub(crate) fn new_plan(w: &mut MainWindow) -> Task<RootMessage> {
    let plan = Playlist::new("New Service Plan".to_string());
    w.service.name_edit = plan.name.clone();
    w.service.editing = Some(plan);
    w.service.to_delete = None;
    Task::none()
}

pub(crate) fn open_plan(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    match w.services.playlists.get(&id) {
        Ok(plan) => {
            w.service.name_edit = plan.name.clone();
            w.service.to_delete = None;
            w.service.editing = Some(plan);
        }
        Err(e) => w.set_error(format!("open plan error: {e}")),
    }
    Task::none()
}

pub(crate) fn save_playlist(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(mut plan) = w.service.editing.clone() else {
        return Task::none();
    };
    plan.name = w.service.name_edit.clone();
    plan.updated_at = chrono::Utc::now();
    if let Err(e) = w.services.playlists.save(&plan) {
        w.set_error(format!("save plan error: {e}"));
        return Task::none();
    }
    w.service.editing = Some(plan);
    w.load_service_plans();
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    crate::ui::editor::delete_confirm::open_delete_dialog(
        w,
        crate::ui::editor::delete_confirm::DeleteTarget::Playlist,
        id,
    )
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(id) = w.service.to_delete.take() else {
        return Task::none();
    };
    if let Err(e) = w.services.playlists.delete(&id) {
        w.set_error(format!("delete plan error: {e}"));
        return Task::none();
    }
    if w.service.editing.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.editing = None;
        w.service.name_edit = String::new();
    }
    if w.service.active.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.active = None;
    }
    w.load_service_plans();
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<RootMessage> {
    w.service.to_delete = None;
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}

pub(crate) fn plan_name_changed(w: &mut MainWindow, name: String) -> Task<RootMessage> {
    w.service.name_edit = name;
    Task::none()
}

pub(crate) fn add_presentation_item(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    let name = w
        .editor
        .presentations
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(PlaylistItem::Presentation { id, name });
    }
    Task::none()
}

pub(crate) fn add_song_item(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    let title = w
        .song
        .songs
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.title.clone())
        .unwrap_or_default();
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(PlaylistItem::Song { id, title });
    }
    Task::none()
}

pub(crate) fn add_header(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(PlaylistItem::Header {
            text: String::new(),
        });
    }
    Task::none()
}

pub(crate) fn add_blank(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing {
        plan.items.push(PlaylistItem::Blank);
    }
    Task::none()
}

pub(crate) fn item_header_changed(
    w: &mut MainWindow,
    index: usize,
    text: String,
) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing
        && let Some(PlaylistItem::Header { text: t }) = plan.items.get_mut(index)
    {
        *t = text;
    }
    Task::none()
}

pub(crate) fn remove_item(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing
        && index < plan.items.len()
    {
        plan.items.remove(index);
    }
    Task::none()
}

pub(crate) fn move_item_up(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing
        && index > 0
        && index < plan.items.len()
    {
        plan.items.swap(index - 1, index);
    }
    Task::none()
}

pub(crate) fn move_item_down(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing
        && index + 1 < plan.items.len()
    {
        plan.items.swap(index, index + 1);
    }
    Task::none()
}

pub(crate) fn duplicate_item(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    if let Some(ref mut plan) = w.service.editing
        && index < plan.items.len()
    {
        let item = plan.items[index].clone();
        plan.items.insert(index + 1, item);
    }
    Task::none()
}

pub(crate) fn start_service(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(plan) = w.service.editing.clone() else {
        return Task::none();
    };
    w.service.active = Some(plan.clone());
    w.service.item_index = 0;
    load_item(w, 0);
    Task::none()
}

pub(crate) fn service_next(w: &mut MainWindow) -> Task<RootMessage> {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    let next = (w.service.item_index + 1).min(cap.saturating_sub(1));
    w.service.item_index = next;
    load_item(w, next);
    Task::none()
}

pub(crate) fn service_prev(w: &mut MainWindow) -> Task<RootMessage> {
    let prev = w.service.item_index.saturating_sub(1);
    w.service.item_index = prev;
    load_item(w, prev);
    Task::none()
}

pub(crate) fn jump_to_item(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    let idx = index.min(cap.saturating_sub(1));
    w.service.item_index = idx;
    load_item(w, idx);
    Task::none()
}

pub(crate) fn end_service(w: &mut MainWindow) -> Task<RootMessage> {
    w.service.active = None;
    w.service.item_index = 0;
    Task::none()
}

fn load_item(w: &mut MainWindow, index: usize) {
    let item = w
        .service
        .active
        .as_ref()
        .and_then(|p| p.items.get(index))
        .cloned();

    match item {
        Some(PlaylistItem::Presentation { id, .. }) => match w.services.presentations.get(&id) {
            Ok(pres) => {
                w.presenting.slide_index = 0;
                w.presenting.transition = None;
                w.presenting.presentation = Some(pres);
                w.shell.current_mode = ViewMode::Show;
                let _ = crate::ui::presenter::activate_slide(w, 0, false);
            }
            Err(e) => w.set_error(format!("load presentation item error: {e}")),
        },
        Some(PlaylistItem::Song { id, .. }) => match w.song.repo.get_song(&id) {
            Ok(song) => w.load_song_for_editing(song),
            Err(e) => w.set_error(format!("load song item error: {e}")),
        },
        Some(PlaylistItem::Blank) => {
            if let Some(ref ndi) = w.presenting.ndi_output {
                ndi.black_screen();
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn planning_panel<'a>(
    plans: &'a [Playlist],
    editing: Option<&'a Playlist>,
    plan_name_edit: &'a str,
    presentations: &'a [Presentation],
    songs: &'a [Song],
    active_plan_id: Option<&'a str>,
    service_item_index: usize,
) -> Element<'a, RootMessage> {
    let list = plan_list(plans, editing.map(|p| p.id.as_str()), active_plan_id);
    let editor: Element<'a, RootMessage> = if let Some(plan) = editing {
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
    plans: &'a [Playlist],
    active_id: Option<&'a str>,
    active_plan_id: Option<&'a str>,
) -> Element<'a, RootMessage> {
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

            let live_el: Element<'_, RootMessage> = if is_live {
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
                    .on_press(wrap(Message::Open(plan.id.clone())))
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
                    .on_press(wrap(Message::New))
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
    plan: &'a Playlist,
    plan_name_edit: &'a str,
    presentations: &'a [Presentation],
    songs: &'a [Song],
    active_plan_id: Option<&'a str>,
    service_item_index: usize,
) -> Element<'a, RootMessage> {
    let is_live = active_plan_id == Some(plan.id.as_str());

    let name_row = row![
        text_input("Plan name", plan_name_edit)
            .on_input(move |v| wrap(Message::NameChanged(v)))
            .padding([6u16, 10u16])
            .size(15)
            .width(Length::Fill),
    ]
    .padding([10u16, 16u16]);

    let live_label: Element<'_, RootMessage> = if is_live {
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

    let start_stop: Element<'_, RootMessage> = if is_live {
        button(text("End Service").size(12))
            .on_press(wrap(Message::End))
            .style(theme::danger_button)
            .padding([6u16, 14u16])
            .into()
    } else {
        button(text("Start Service").size(12))
            .on_press(wrap(Message::Start))
            .style(theme::primary_button)
            .padding([6u16, 14u16])
            .into()
    };

    let actions = row![
        button(text("Save").size(13))
            .on_press(wrap(Message::Save))
            .style(theme::primary_button)
            .padding([6u16, 14u16]),
        start_stop,
        live_label,
        Space::new().width(Length::Fill),
        button(text("Delete Plan").size(12))
            .on_press(wrap(Message::DeleteClicked(plan.id.clone())))
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
            .on_press(wrap(Message::AddHeader))
            .style(theme::secondary_button)
            .padding([5u16, 10u16]),
        button(text("+ Blank").size(12))
            .on_press(wrap(Message::AddBlank))
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
                .on_press(wrap(Message::AddPresentation(p.id.clone())))
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
                .on_press(wrap(Message::AddSong(s.id.clone())))
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
    item: &'a PlaylistItem,
    total: usize,
    is_current: bool,
) -> Element<'a, RootMessage> {
    let badge_color = match item {
        PlaylistItem::Presentation { .. } => Color::from_rgb(0.29, 0.56, 0.89),
        PlaylistItem::Song { .. } => Color::from_rgb(0.56, 0.36, 0.89),
        PlaylistItem::MediaCue { .. } => Color::from_rgb(0.89, 0.56, 0.29),
        PlaylistItem::Header { .. } => Color::from_rgb(0.36, 0.72, 0.50),
        PlaylistItem::Blank => Color::from_rgb(0.45, 0.45, 0.45),
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

    let name_el: Element<'_, RootMessage> = match item {
        PlaylistItem::Header { text: t } => text_input("Section header...", t)
            .on_input(move |v| wrap(Message::ItemHeaderChanged(i, v)))
            .padding([3u16, 6u16])
            .size(13)
            .into(),
        _ => text(item.display_name()).size(13).into(),
    };

    let current_indicator: Element<'_, RootMessage> = if is_current {
        fa_icon_solid("caret-right")
            .size(12.0_f32)
            .color(theme::LIVE_GREEN)
            .into()
    } else {
        Space::new().width(12).into()
    };

    let up_el: Element<'_, RootMessage> = if i > 0 {
        button(text("^").size(11))
            .on_press(wrap(Message::MoveItemUp(i)))
            .style(theme::secondary_button)
            .padding([3u16, 7u16])
            .into()
    } else {
        Space::new().width(26).into()
    };

    let down_el: Element<'_, RootMessage> = if i + 1 < total {
        button(text("v").size(11))
            .on_press(wrap(Message::MoveItemDown(i)))
            .style(theme::secondary_button)
            .padding([3u16, 7u16])
            .into()
    } else {
        Space::new().width(26).into()
    };

    let jump_el: Element<'_, RootMessage> = if is_current {
        Space::new().width(0).into()
    } else {
        button(text("Go").size(10))
            .on_press(wrap(Message::JumpTo(i)))
            .style(theme::ghost_button)
            .padding([3u16, 7u16])
            .into()
    };

    let dup = button(text("⧉").size(11))
        .on_press(wrap(Message::DuplicateItem(i)))
        .style(theme::secondary_button)
        .padding([3u16, 7u16]);

    let del = button(text("x").size(11))
        .on_press(wrap(Message::RemoveItem(i)))
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
