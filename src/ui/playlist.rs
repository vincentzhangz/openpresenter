//! Playlist (service plan) panel: plan CRUD, rundown item editing and
//! reordering, and playlist-driven presenting (Start/Next/Prev/Jump/End open
//! plan items as live presentations).
//!
//! Ported from the iced-era `ui/playlist/mod.rs`. The plan-name field and the
//! inline header editor are GPUI [`gpui_kit::component::input::Input`]
//! entities owned by [`crate::ui::main_window::MainWindow`]; their
//! subscriptions mirror text into `service.name_edit` / the header item as
//! the user types.

use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, Context, Div, IntoElement, ParentElement, SharedString, Styled, Window, div,
    px, rgb,
};

use crate::domain::{Playlist, PlaylistItem, Presentation};
use crate::ui::components::{self, truncate};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab, ViewMode};
use crate::ui::state::ServiceState;
use crate::ui::theme;

/// Mirror the plan-name state into the shared input entity.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let name = w.service.name_edit.clone();
    crate::ui::components::sync_text_field(&w.playlist_name_input, &name, window, cx);
}

/// Rail wrapper for [`list_view`].
pub(crate) fn rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> gpui_kit::AnyElement {
    list_view(w, cx).into_any_element()
}

/// Messages owned by the Playlist feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Playlist(..)`.
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
    /// Add a media cue: a file that plays at this point in the service.
    AddMedia,
    ItemHeaderChanged(usize, String),
    RemoveItem(usize),
    MoveItemUp(usize),
    MoveItemDown(usize),
    DuplicateItem(usize),
    Duplicate(String),
    StartPlan(String),
    Start,
    Next,
    Prev,
    JumpTo(usize),
    End,
    /// Turn the continuous loop on or off: the service wraps around at the
    /// end instead of stopping.
    SetContinuous(bool),
}

/// Dispatch a playlist message.
pub(crate) fn update(
    w: &mut MainWindow,
    msg: Message,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match msg {
        Message::New => new_plan(w, window, cx),
        Message::Open(id) => open_plan(w, id, window, cx),
        Message::Save => save_playlist(w, cx),
        Message::DeleteClicked(id) => delete_clicked(w, id, cx),
        Message::ConfirmDelete => confirm_delete(w, cx),
        Message::CancelDelete => cancel_delete(w, cx),
        Message::NameChanged(name) => plan_name_changed(w, name, cx),
        Message::AddPresentation(id) => add_presentation_item(w, id, cx),
        Message::AddSong(id) => add_song_item(w, id, cx),
        Message::AddHeader => add_header(w, cx),
        Message::AddBlank => add_blank(w, cx),
        Message::AddMedia => add_media(w, cx),
        Message::ItemHeaderChanged(i, t) => item_header_changed(w, i, t, cx),
        Message::RemoveItem(i) => remove_item(w, i, cx),
        Message::MoveItemUp(i) => move_item_up(w, i, cx),
        Message::MoveItemDown(i) => move_item_down(w, i, cx),
        Message::DuplicateItem(i) => duplicate_item(w, i, cx),
        Message::Duplicate(id) => duplicate_plan(w, id, window, cx),
        Message::StartPlan(id) => start_plan(w, id, window, cx),
        Message::Start => start_service(w, cx),
        Message::Next => service_next(w, cx),
        Message::Prev => service_prev(w, cx),
        Message::JumpTo(i) => jump_to_item(w, i, cx),
        Message::End => end_service(w, cx),
        Message::SetContinuous(on) => {
            w.service.continuous = on;
            cx.notify();
        }
    }
}

// ---------------------------------------------------------------------------
// Plan lifecycle
// ---------------------------------------------------------------------------

/// Load the plan-name input with `name` (the input is the source of truth the
/// editor renders; `set_value` does not emit `Change`).
fn sync_plan_name_input(
    w: &mut MainWindow,
    name: &str,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let name = name.to_string();
    w.playlist_name_input.update(cx, |state, input_cx| {
        state.set_value(&name, window, input_cx);
    });
}

pub(crate) fn new_plan(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let plan = Playlist::new(String::from("New Service Plan"));
    w.service.name_edit = plan.name.clone();
    w.service.editing = Some(plan);
    w.service.to_delete = None;
    w.service.header_editing = false;
    let name = w.service.name_edit.clone();
    sync_plan_name_input(w, &name, window, cx);
    cx.notify();
}

pub(crate) fn open_plan(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    match w.services.playlists.get(&id) {
        Ok(plan) => {
            w.service.name_edit = plan.name.clone();
            w.service.to_delete = None;
            w.service.header_editing = false;
            let running = w.service.active.as_ref().map(|plan| plan.id.clone());
            w.service.editing = Some(plan);
            if running.as_deref() == Some(id.as_str())
                && let Some(active) = w.service.active.clone()
            {
                // The running plan and the plan on screen are the same plan:
                // the rundown the operator reads has to be the one that is
                // playing, or "Go" jumps a different item from the one they
                // picked.
                w.service.editing = Some(active);
            }
            let name = w.service.name_edit.clone();
            sync_plan_name_input(w, &name, window, cx);
            // The item names are read here, not on every render.
            refresh_item_names(w);
        }
        Err(e) => w.set_error(format!("open plan error: {e}")),
    }
    cx.notify();
}

pub(crate) fn duplicate_plan(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    match w.services.playlists.duplicate(&id) {
        Ok(plan) => {
            w.load_service_plans();
            w.service.header_editing = false;
            // The copy is named "… (Copy)": the field has to show the copy's
            // own name, or pressing Save writes the source's name over it and
            // the rail holds two plans with nothing to tell them apart.
            w.service.name_edit = plan.name.clone();
            w.service.editing = Some(plan);
            let name = w.service.name_edit.clone();
            sync_plan_name_input(w, &name, window, cx);
        }
        Err(e) => w.set_error(format!("duplicate plan error: {e}")),
    }
    cx.notify();
}

pub(crate) fn start_plan(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    open_plan(w, id, window, cx);
    start_service(w, cx);
}

pub(crate) fn save_playlist(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(mut plan) = w.service.editing.clone() else {
        return;
    };
    plan.name = w.service.name_edit.clone();
    plan.updated_at = chrono::Utc::now();
    if let Err(e) = w.services.playlists.save(&plan) {
        w.set_error(format!("save plan error: {e}"));
        cx.notify();
        return;
    }
    w.service.editing = Some(plan);
    w.load_service_plans();
    cx.notify();
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.shell.rail_context_target = None;
    w.service.to_delete = Some(id);
    w.editor.show_delete_confirmation = true;
    cx.notify();
}

pub(crate) fn confirm_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(id) = w.service.to_delete.take() else {
        return;
    };
    if let Err(e) = w.services.playlists.delete(&id) {
        w.set_error(format!("delete plan error: {e}"));
        cx.notify();
        return;
    }
    if w.service.editing.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.editing = None;
        w.service.name_edit = String::new();
    }
    if w.service.active.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
        w.service.active = None;
        w.service.item_index = 0;
        // The last item stays on the output otherwise, so the service would
        // appear to still be running.
        crate::ui::navigation::clear_output(w, cx);
    }
    w.load_service_plans();
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

pub(crate) fn cancel_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.service.to_delete = None;
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

/// Mirror the plan-name input into `service.name_edit`. Called from the
/// input's subscription, so no redraw is triggered from here.
pub(crate) fn plan_name_changed(w: &mut MainWindow, name: String, cx: &mut Context<MainWindow>) {
    w.service.name_edit = name;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Rundown items
// ---------------------------------------------------------------------------

/// Clear the inline header editor if there are no items left.
fn sync_header_editing(w: &mut MainWindow) {
    let len = w
        .service
        .editing
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    if len == 0 {
        w.service.header_editing = false;
    }
}

/// Apply a change to the plan being edited.
///
/// While a service runs, the running plan is the same plan: the rundown the
/// operator sees is the one being played, so a change has to reach the
/// running copy too — otherwise "Go" would play a different item from the
/// one the operator picked, and the counter would describe a plan that is
/// not the one on the output.
fn edit_plan(w: &mut MainWindow, change: impl Fn(&mut Playlist)) {
    {
        let ServiceState {
            editing,
            active,
            item_index,
            ..
        } = &mut w.service;
        let running_id = active.as_ref().map(|plan| plan.id.clone());
        if let Some(plan) = editing {
            change(plan);
            if running_id.as_deref() == Some(plan.id.as_str())
                && let Some(running) = active
            {
                change(running);
            }
        }
        // The change may have removed the item that is playing.
        if let Some(running) = active {
            let last = running.items.len().saturating_sub(1);
            if *item_index > last {
                *item_index = last;
            }
        }
    }
    // The names are read again: an item that was added shows its own name.
    refresh_item_names(w);
}

pub(crate) fn add_presentation_item(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    let name = w
        .editor
        .presentations
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .unwrap_or_default();
    edit_plan(w, |plan| {
        plan.items.push(PlaylistItem::Presentation {
            id: id.clone(),
            name: name.clone(),
        });
    });
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn add_song_item(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    let title = w
        .song
        .songs
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.title.clone())
        .unwrap_or_default();
    edit_plan(w, |plan| {
        plan.items.push(PlaylistItem::Song {
            id: id.clone(),
            title: title.clone(),
        });
    });
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn add_header(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    edit_plan(w, |plan| {
        plan.items.push(PlaylistItem::Header {
            text: String::new(),
        });
    });
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn add_blank(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    edit_plan(w, |plan| {
        plan.items.push(PlaylistItem::Blank);
    });
    sync_header_editing(w);
    cx.notify();
}

/// Add a media cue to the plan: a file that plays at this point in the
/// service.
pub(crate) fn add_media(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Media",
            &[
                "mp4", "mov", "mkv", "webm", "mp3", "wav", "flac", "m4a", "aac",
            ],
        )
        .set_title("Select Media Cue")
        .pick_file()
    else {
        return;
    };
    let path = path.to_string_lossy().into_owned();
    edit_plan(w, |plan| {
        plan.items
            .push(PlaylistItem::MediaCue { path: path.clone() });
    });
    sync_header_editing(w);
    cx.notify();
}

/// Write `text` into the header item at `index`. Called from the shared
/// header input's subscription as the user types.
pub(crate) fn item_header_changed(
    w: &mut MainWindow,
    index: usize,
    text: String,
    cx: &mut Context<MainWindow>,
) {
    edit_plan(w, |plan| {
        if let Some(PlaylistItem::Header { text: t }) = plan.items.get_mut(index) {
            *t = text.clone();
        }
    });
    cx.notify();
}

pub(crate) fn remove_item(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    let running = w.service.active.is_some();
    let current = w.service.item_index;
    edit_plan(w, |plan| {
        if index < plan.items.len() {
            plan.items.remove(index);
        }
    });
    // The item that is playing may have been the one that was removed, or the
    // one before it: the output has to show what the plan now says, or the
    // picture stays up while the counter moves on.
    if running && index <= current {
        let last = w
            .service
            .active
            .as_ref()
            .map(|plan| plan.items.len())
            .unwrap_or(0)
            .saturating_sub(1);
        let at = current.min(last);
        w.service.item_index = at;
        load_item(w, at, cx);
    }
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn move_item_up(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    edit_plan(w, |plan| {
        if index > 0 && index < plan.items.len() {
            plan.items.swap(index - 1, index);
        }
    });
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn move_item_down(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    edit_plan(w, |plan| {
        if index + 1 < plan.items.len() {
            plan.items.swap(index, index + 1);
        }
    });
    sync_header_editing(w);
    cx.notify();
}

pub(crate) fn duplicate_item(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    edit_plan(w, |plan| {
        if index < plan.items.len() {
            let item = plan.items[index].clone();
            plan.items.insert(index + 1, item);
        }
    });
    sync_header_editing(w);
    cx.notify();
}

// ---------------------------------------------------------------------------
// Playlist-driven presenting
// ---------------------------------------------------------------------------

pub(crate) fn start_service(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(plan) = w.service.editing.clone() else {
        return;
    };
    w.service.active = Some(plan);
    w.service.item_index = 0;
    load_item(w, 0, cx);
    cx.notify();
}

pub(crate) fn service_next(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    if cap == 0 {
        return;
    }
    // Continuous mode wraps to the first item at the end, the way a
    // background loop does. Otherwise the last item is the end of the
    // plan: loading it again would restart the item — a presentation
    // would jump back to its first slide and a media cue would start from
    // zero.
    let next = if w.service.item_index + 1 >= cap {
        if w.service.continuous {
            0
        } else {
            return;
        }
    } else {
        w.service.item_index + 1
    };
    let previous = w.service.item_index;
    w.service.item_index = next;
    if !load_item(w, next, cx) {
        // The item could not be loaded, so the plan has not moved: a counter
        // that says the service is further along than it is is a counter the
        // operator cannot trust.
        w.service.item_index = previous;
    }
    cx.notify();
}

pub(crate) fn service_prev(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The first item is the start of the plan: there is nothing before it.
    if w.service.item_index == 0 {
        return;
    }
    let prev = w.service.item_index - 1;
    let previous = w.service.item_index;
    w.service.item_index = prev;
    if !load_item(w, prev, cx) {
        w.service.item_index = previous;
    }
    cx.notify();
}

pub(crate) fn jump_to_item(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    let cap = w
        .service
        .active
        .as_ref()
        .map(|p| p.items.len())
        .unwrap_or(0);
    let idx = index.min(cap.saturating_sub(1));
    let previous = w.service.item_index;
    w.service.item_index = idx;
    if !load_item(w, idx, cx) {
        // The item could not be loaded, so the plan has not moved.
        w.service.item_index = previous;
    }
    cx.notify();
}

pub(crate) fn end_service(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.service.active = None;
    w.service.item_index = 0;
    // The last item stays on the output otherwise, so the service would
    // appear to still be running.
    crate::ui::navigation::clear_output(w, cx);
    cx.notify();
}

/// Open the plan item at `index` in the live output: presentations and songs
/// become the live presentation, Blank drops every layer.
///
/// Returns whether the item really reached the output: an item whose deck
/// was deleted cannot, and the counter must not move past it.
fn load_item(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) -> bool {
    let item = w
        .service
        .active
        .as_ref()
        .and_then(|p| p.items.get(index))
        .cloned();

    match item {
        Some(PlaylistItem::Presentation { id, .. }) => match w.services.presentations.get(&id) {
            Ok(pres) => {
                open_item_presentation(w, pres, cx);
                true
            }
            Err(e) => {
                w.set_error(format!("load presentation item error: {e}"));
                false
            }
        },
        Some(PlaylistItem::Song { id, .. }) => match w.song.service.get(&id) {
            Ok(song) => {
                let slides: Vec<crate::domain::Slide> = song
                    .verses
                    .iter()
                    .map(|v| {
                        let mut slide = crate::domain::Slide::new_text_in_group(
                            v.content.clone(),
                            v.label.clone(),
                        );
                        slide.transition = w.presenting.global_transition;
                        slide
                    })
                    .collect();
                if slides.is_empty() {
                    // A song with no verses has nothing to show: the plan
                    // must not move past it, and the operator has to know
                    // why the room is still looking at the last slide.
                    w.set_error(format!("the song \"{}\" has no verses to show", song.title));
                    return false;
                }
                let pres = Presentation {
                    timeline: crate::domain::Timeline::default(),
                    id: format!("song-{}", song.id),
                    name: song.title.clone(),
                    slides,
                    created_at: song.created_at,
                    updated_at: song.updated_at,
                };
                open_item_presentation(w, pres, cx);
                true
            }
            Err(e) => {
                w.set_error(format!("load song item error: {e}"));
                false
            }
        },
        Some(PlaylistItem::Blank) => {
            // A blank moment blanks the whole output: the slide, the media,
            // the props and the message. Turning off only the layers would
            // leave the lower thirds on the projector while the broadcast
            // went black, and the two surfaces would disagree.
            crate::ui::navigation::clear_output(w, cx);
            true
        }
        Some(PlaylistItem::MediaCue { path }) => {
            // A media cue plays a file during the service: a clip, a track,
            // a recording.
            let path = path.clone();
            if is_video_path(&path) {
                crate::ui::video::open_video(w, &path);
            } else {
                // The cue plays the track: a media cue the room cannot hear
                // is a cue that is not there.
                crate::ui::audio::load_and_play(w, path, None, cx);
            }
            true
        }
        // Header items are rundown structure only; they never touch output.
        _ => true,
    }
}

/// Put a presentation (from a plan item) live at slide 0 in Show mode.
fn open_item_presentation(w: &mut MainWindow, pres: Presentation, cx: &mut Context<MainWindow>) {
    w.presenting.slide_index = 0;
    w.presenting.presentation = Some(pres);
    w.shell.current_mode = ViewMode::Show;
    // The operator stays in the plan: the plan's own transport — the item
    // counter, "Go", "Next" and "Prev" — lives there, and moving the
    // operator out of it would take the controls of the running service
    // away mid-service.
    crate::ui::navigation::activate_slide(w, 0, false, cx);
}

/// Convert a song library entry into a presentation (verse → grouped text
/// slide) and open it for editing. Uses only the song repository and the
/// presentation service, so it works without the songs editor panel (its own
/// phase).
pub(crate) fn song_to_presentation(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let Ok(song) = w.song.service.get(&id) else {
        w.set_error(format!("Song \"{id}\" was not found."));
        cx.notify();
        return;
    };
    let slides: Vec<crate::domain::Slide> = song
        .verses
        .iter()
        .map(|verse| {
            let mut slide =
                crate::domain::Slide::new_text_in_group(verse.content.clone(), verse.label.clone());
            slide.transition = crate::domain::Transition::Fade { duration_ms: 500 };
            slide
        })
        .collect();
    let pres_name = song.title.clone();
    // The presentation and its slides are written together: a presentation
    // whose slides failed to save would be an empty entry in the rail while
    // the editor showed the slides.
    match w
        .services
        .presentations
        .create_with_slides(&pres_name, &slides)
    {
        Ok(new_pres) => {
            w.load_presentations();
            w.editor.editing = Some(Presentation {
                timeline: crate::domain::Timeline::default(),
                id: new_pres.id,
                name: new_pres.name,
                slides,
                created_at: new_pres.created_at,
                updated_at: new_pres.updated_at,
            });
            let slide_count = w
                .editor
                .editing
                .as_ref()
                .map(|p| p.slides.len())
                .unwrap_or(0);
            w.editor.selected_slide_index = if slide_count > 0 { Some(0) } else { None };
            w.shell.current_mode = ViewMode::Edit;
            w.shell.sidebar_tab = SidebarTab::Presentations;
            crate::ui::navigation::load_slide_for_editing(w);
            w.editor.editing_name = pres_name.clone();
            crate::ui::components::sync_text_field(&w.sync_rename_input, &pres_name, window, cx);
        }
        Err(e) => w.set_error(format!("song_to_presentation create: {e}")),
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Views: rail plan list
// ---------------------------------------------------------------------------

/// Render the playlist list for the left rail (plan rows + "+ New Plan").
pub fn list_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let active_edit_id = w.service.editing.as_ref().map(|p| p.id.clone());
    let active_plan_id = w.service.active.as_ref().map(|p| p.id.clone());
    let plans = w.service.plans.clone();

    let mut list = div().flex().flex_col().gap(px(2.));
    if plans.is_empty() {
        list = list.child(components::empty_state("No plans yet"));
    }
    for plan in &plans {
        let id = plan.id.clone();
        let name = plan.name.clone();
        // The list carries the count, not the items, so the rail can say how
        // long a plan is without loading every plan's items.
        let count = plan.item_count;
        let is_active_edit = active_edit_id.as_deref() == Some(plan.id.as_str());
        let is_live = active_plan_id.as_deref() == Some(plan.id.as_str());
        let id_open = id.clone();
        let id_menu = id.clone();

        let row = div()
            .id(SharedString::from(format!("rail-playlist-{}", plan.id)))
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.))
            .py(px(6.))
            .px(px(10.))
            .rounded(px(5.))
            .border_1()
            .border_color(if is_active_edit {
                theme::c(theme::ACCENT_ORANGE)
            } else {
                gpui_kit::transparent_black()
            })
            .bg(if is_active_edit {
                theme::c(theme::ACCENT_ORANGE).opacity(0.14)
            } else {
                gpui_kit::transparent_black()
            })
            .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Playlist(Message::Open(id_open.clone())),
                    window,
                    cx,
                );
            }))
            .on_mouse_down(
                gpui_kit::MouseButton::Right,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::ShowRailContextMenu(
                            crate::ui::messages::RailContextTarget::Playlist(id_menu.clone()),
                        ),
                        window,
                        cx,
                    );
                }),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(if is_active_edit {
                                theme::c(theme::TEXT_PRIMARY)
                            } else {
                                theme::c(theme::TEXT_SECONDARY)
                            })
                            .child(truncate(&name, 30)),
                    )
                    .when(is_live, |el| {
                        el.child(components::live_badge("LIVE", is_live))
                    }),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(format!("{count} items")),
            );
        list = list.child(row);
    }

    let new_btn = components::add_button("rail-new-plan", "+ New Plan", {
        cx.listener(|w, _, window, cx| {
            w.handle_message(RootMessage::Playlist(Message::New), window, cx);
        })
    });

    div()
        .id("playlist-rail-list")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .child(list)
        .child(new_btn)
}

// ---------------------------------------------------------------------------
// Views: rundown editor (center panel)
// ---------------------------------------------------------------------------

/// Render the playlist editor / rundown panel for the center workspace.
pub fn editor_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let active_plan_id = w.service.active.as_ref().map(|p| p.id.clone());
    let Some(plan) = w.service.editing.clone() else {
        return empty_editor();
    };
    let item_index = w.service.item_index;
    let header_editing = w.service.header_editing;
    let is_live = active_plan_id.as_deref() == Some(plan.id.as_str());
    // The transport drives the running plan, so it belongs to whatever plan is
    // running: opening another plan must not take the controls of the service
    // away in the middle of it.
    let running = w.service.active.clone();
    let running_items = running.as_ref().map(|plan| plan.items.len()).unwrap_or(0);

    // Name row: the shared plan-name input entity.
    let name_row = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .py(px(10.))
        .px(px(16.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(&w.playlist_name_input.clone())),
        );

    // Actions row: save, start/end, live position, delete. The button acts on
    // the running plan: ending the service ends the one that is running.
    let start_stop = if running.is_some() {
        small_button(
            "plan-end",
            "End Service",
            theme::DANGER_RED,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::End), window, cx);
            }),
        )
    } else {
        small_button(
            "plan-start",
            "Start Service",
            theme::ACCENT_ORANGE,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::Start), window, cx);
            }),
        )
    };

    let mut actions = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pb(px(10.))
        .px(px(16.))
        .child(small_button(
            "plan-save",
            "Save",
            theme::ACCENT_ORANGE,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::Save), window, cx);
            }),
        ))
        .child(start_stop);

    if running.is_some() {
        actions = actions.child(
            div()
                .text_size(px(13.))
                .text_color(theme::c(theme::LIVE_GREEN))
                .child(format!("LIVE  {}/{}", item_index + 1, running_items)),
        );
        // The transport: a plan is run by moving through it, not by
        // clicking every item.
        actions = actions
            .child(small_button(
                "plan-prev",
                "← Prev",
                theme::TEXT_SECONDARY,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Playlist(Message::Prev), window, cx);
                }),
            ))
            .child(small_button(
                "plan-next",
                "Next →",
                theme::ACCENT_ORANGE,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Playlist(Message::Next), window, cx);
                }),
            ))
            // Continuous mode wraps the plan at the end, so the check mark
            // in the View menu is the truth about the service.
            .child(small_button(
                "plan-continuous",
                if w.service.continuous {
                    "Continuous ✓"
                } else {
                    "Continuous"
                },
                if w.service.continuous {
                    theme::LIVE_GREEN
                } else {
                    theme::TEXT_SECONDARY
                },
                cx.listener(|w, _, window, cx| {
                    let on = !w.service.continuous;
                    w.handle_message(
                        RootMessage::Playlist(Message::SetContinuous(on)),
                        window,
                        cx,
                    );
                }),
            ));
    }

    actions = actions.child(div().flex_1()).child(small_button(
        "plan-delete",
        "Delete Plan",
        theme::DANGER_RED,
        cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::Playlist(Message::DeleteClicked(plan.id.clone())),
                window,
                cx,
            );
        }),
    ));

    // Rundown items.
    let total = plan.items.len();
    let header_input = w.playlist_header_input.clone();
    let mut items_col = div().flex().flex_col().gap(px(6.)).pb(px(8.)).px(px(16.));
    if plan.items.is_empty() {
        items_col = items_col.child(
            div()
                .py(px(16.))
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No items yet — add from the pickers below"),
        );
    } else {
        for (i, item) in plan.items.iter().enumerate() {
            let is_current = is_live && i == item_index;
            items_col = items_col.child(item_card(
                ItemCard {
                    index: i,
                    item: item.clone(),
                    display_name: item_display_name(w, item),
                    total,
                    is_current,
                    is_live,
                    header_editing,
                    header_input: header_input.clone(),
                },
                cx,
            ));
        }
    }

    // Add section: header/blank buttons plus presentation/song pickers.
    let add_header = div()
        .w_full()
        .flex()
        .flex_row()
        .gap(px(6.))
        .py(px(6.))
        .px(px(16.))
        .child(small_button(
            "plan-add-header",
            "+ Header",
            theme::TEXT_SECONDARY,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::AddHeader), window, cx);
            }),
        ))
        .child(small_button(
            "plan-add-blank",
            "+ Blank",
            theme::TEXT_SECONDARY,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::AddBlank), window, cx);
            }),
        ))
        .child(small_button(
            "plan-add-media",
            "+ Media",
            theme::TEXT_SECONDARY,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::AddMedia), window, cx);
            }),
        ));

    let presentations = w.editor.presentations.clone();
    let songs = w.song.songs.clone();

    let mut pres_col = div().flex().flex_col().gap(px(2.)).px(px(16.));
    for p in &presentations {
        let id = p.id.clone();
        let name = p.name.clone();
        pres_col = pres_col.child(picker_row(
            SharedString::from(format!("plan-add-pres-{}", p.id)),
            name,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Playlist(Message::AddPresentation(id.clone())),
                    window,
                    cx,
                );
            }),
        ));
    }

    let mut song_col = div().flex().flex_col().gap(px(2.)).px(px(16.));
    for s in &songs {
        let id = s.id.clone();
        let title = s.title.clone();
        song_col = song_col.child(picker_row(
            SharedString::from(format!("plan-add-song-{}", s.id)),
            title,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Playlist(Message::AddSong(id.clone())),
                    window,
                    cx,
                );
            }),
        ));
    }

    let add_section = div()
        .h(px(200.))
        .flex()
        .flex_col()
        .child(picker_label("Presentations"))
        .child(
            div()
                .id("plan-pres-picker")
                .max_h(px(80.))
                .min_h_0()
                .overflow_y_scroll()
                .child(pres_col),
        )
        .child(picker_label("Songs"))
        .child(
            div()
                .id("plan-song-picker")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(song_col),
        );

    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARKEST))
        .child(name_row)
        .child(components::divider())
        .child(actions)
        .child(components::divider())
        .child(
            div()
                .id("plan-items-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(items_col),
        )
        .child(components::divider())
        .child(add_header)
        .child(add_section)
}

/// Empty state shown when no plan is open for editing.
fn empty_editor() -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .bg(theme::c(theme::BG_DARKEST))
        .child(
            div()
                .text_size(px(32.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("☰"),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Select a service plan from the sidebar, or click + New Plan"),
        )
}

fn picker_label(label: &'static str) -> Div {
    div()
        .pt(px(6.))
        .px(px(16.))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child(label)
}

fn picker_row(
    id: SharedString,
    label: String,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .py(px(4.))
        .px(px(8.))
        .rounded(px(4.))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| {
            s.bg(theme::c(theme::BG_HOVER))
                .text_color(theme::c(theme::TEXT_PRIMARY))
        })
        .on_click(on_click)
        .child(truncate(&label, 44))
}

/// Badge color per item type (ported from the iced item card).
fn item_badge_color(item: &PlaylistItem) -> gpui_kit::Hsla {
    match item {
        PlaylistItem::Presentation { .. } => rgb(0x4A8FE3).into(),
        PlaylistItem::Song { .. } => rgb(0x8F5CE3).into(),
        PlaylistItem::MediaCue { .. } => rgb(0xE38F4A).into(),
        PlaylistItem::Header { .. } => rgb(0x5CB880).into(),
        PlaylistItem::Blank => rgb(0x737373).into(),
    }
}

#[allow(clippy::too_many_arguments)]
/// The name a plan item shows, resolved from the library when it can be.
///
/// The names are read when the plan is opened and after every edit, and kept
/// in the state: a lookup per item per render is a lag the operator feels.
/// The name stored in the item is the fallback, so a plan that was never
/// opened still shows something sensible.
fn item_display_name(_w: &MainWindow, item: &PlaylistItem) -> String {
    match item {
        PlaylistItem::Presentation { name, .. } => name.clone(),
        PlaylistItem::Song { title, .. } => title.clone(),
        _ => item.display_name(),
    }
}

/// Read the names the plan's items show.
///
/// The names live in the library, so a renamed presentation shows its new
/// name. They are read when the plan is opened and after every edit, not on
/// every render.
pub(crate) fn refresh_item_names(w: &mut MainWindow) {
    let mut names: Vec<String> = Vec::new();
    let Some(plan) = w.service.editing.as_ref().or(w.service.active.as_ref()) else {
        w.service.item_names = names;
        return;
    };
    for item in &plan.items {
        match item {
            PlaylistItem::Presentation { id, name } => match w.services.presentations.get(id) {
                Ok(presentation) => names.push(presentation.name),
                Err(_) => names.push(name.clone()),
            },
            PlaylistItem::Song { id, title } => match w.song.service.get(id) {
                Ok(song) => names.push(song.title),
                Err(_) => names.push(title.clone()),
            },
            _ => names.push(item.display_name()),
        }
    }
    w.service.item_names = names;
}

/// Whether a media file is a video, by its extension.
///
/// A media cue plays a file during the service, and a video is shown on the
/// output while a track is only heard.
fn is_video_path(path: &str) -> bool {
    matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v")
    )
}

/// What one plan item's card shows.
struct ItemCard {
    index: usize,
    item: PlaylistItem,
    display_name: String,
    total: usize,
    is_current: bool,
    /// Whether this plan is the one that is live.
    is_live: bool,
    header_editing: bool,
    header_input: gpui_kit::Entity<gpui_kit::component::input::InputState>,
}

fn item_card(card: ItemCard, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let ItemCard {
        index: i,
        item,
        display_name,
        total,
        is_current,
        is_live,
        header_editing,
        header_input,
    } = card;
    let badge_color = item_badge_color(&item);
    let badge_label = item.type_label();

    let name_el: gpui_kit::AnyElement = match item {
        PlaylistItem::Header { text } => {
            if header_editing {
                // The shared inline editor input edits this header.
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&header_input))
                    .into_any_element()
            } else {
                let text = text.clone();
                let display = if text.trim().is_empty() {
                    String::from("Section header…")
                } else {
                    text.clone()
                };
                div()
                    .id(SharedString::from(format!("plan-header-{i}")))
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .hover(|s| s.text_color(theme::c(theme::TEXT_PRIMARY)))
                    .on_click(cx.listener(move |w, _, window, cx| {
                        let current = text.clone();
                        w.service.header_editing = true;
                        w.playlist_header_input.update(cx, |state, input_cx| {
                            state.set_value(&current, window, input_cx);
                        });
                        cx.notify();
                    }))
                    .child(display)
                    .into_any_element()
            }
        }
        _ => div()
            .flex_1()
            .min_w_0()
            .text_size(px(13.))
            .text_color(theme::c(theme::TEXT_PRIMARY))
            .child(truncate(&display_name, 48))
            .into_any_element(),
    };

    let card = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .py(px(6.))
        .px(px(8.))
        .rounded(px(4.))
        .border_1()
        .border_color(if is_current {
            theme::c(theme::LIVE_GREEN).opacity(0.30)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .bg(if is_current {
            theme::c(theme::LIVE_GREEN).opacity(0.08)
        } else {
            theme::c(theme::BG_DARK)
        });

    let mut row = div()
        .w(px(12.))
        .flex()
        .justify_center()
        .child(if is_current {
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::LIVE_GREEN))
                .child("▸")
        } else {
            div()
        });

    row = row
        .child(
            div()
                .py(px(2.))
                .px(px(5.))
                .rounded(px(3.))
                .text_size(px(9.))
                .text_color(gpui_kit::white())
                .bg(badge_color)
                .child(badge_label),
        )
        .child(div().w(px(8.)))
        .child(name_el)
        .child(div().flex_1());

    // The Go button jumps the live plan, so it only means something on the
    // plan that is running. On another plan it would jump the wrong one.
    if is_live && !is_current {
        row = row.child(tiny_btn(
            SharedString::from(format!("plan-jump-{i}")),
            "Go",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::JumpTo(i)), window, cx);
            }),
        ));
    }
    if i > 0 {
        row = row.child(tiny_btn(
            SharedString::from(format!("plan-up-{i}")),
            "↑",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::MoveItemUp(i)), window, cx);
            }),
        ));
    }
    if i + 1 < total {
        row = row.child(tiny_btn(
            SharedString::from(format!("plan-down-{i}")),
            "↓",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::MoveItemDown(i)), window, cx);
            }),
        ));
    }
    row = row
        .child(tiny_btn(
            SharedString::from(format!("plan-dup-{i}")),
            "⧉",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::DuplicateItem(i)), window, cx);
            }),
        ))
        .child(tiny_btn(
            SharedString::from(format!("plan-del-{i}")),
            "×",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Playlist(Message::RemoveItem(i)), window, cx);
            }),
        ));

    card.child(row)
}

/// Compact rectangular text button used across the rundown cards.
fn tiny_btn(
    id: SharedString,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .justify_center()
        .py(px(3.))
        .px(px(7.))
        .rounded(px(3.))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .bg(theme::c(theme::BG_HOVER))
        .hover(|s| {
            s.bg(theme::c(theme::BG_ACTIVE))
                .text_color(theme::c(theme::TEXT_PRIMARY))
        })
        .on_click(on_click)
        .child(label)
}

/// Small pill button with an accent text color (Save / Start / End / Delete).
fn small_button(
    id: &'static str,
    label: &'static str,
    accent: u32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .py(px(6.))
        .px(px(14.))
        .rounded(px(4.))
        .text_size(px(12.))
        .text_color(theme::c(accent))
        .border_1()
        .border_color(theme::c(accent).opacity(0.6))
        .hover(|s| s.bg(theme::c(accent).opacity(0.14)))
        .on_click(on_click)
        .child(label)
}
