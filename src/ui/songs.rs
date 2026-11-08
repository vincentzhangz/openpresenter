//! Song library panel: song search + list (left rail) and the song editor
//! (title/artist/copyright/CCLI/key/BPM fields, verse list editing, New/
//! Save/Delete with confirm, Send to Presentation).
//!
//! Ported from the iced-era `ui/songs/mod.rs`. The editor's text fields are
//! GPUI [`gpui_kit::component::input::Input`] entities: the six metadata
//! fields are owned by [`crate::ui::main_window::MainWindow`], while every
//! verse gets a label/content pair kept in [`crate::ui::state::SongState`]
//! and rebuilt whenever the verse list changes structurally (add, delete,
//! reorder, open). Input subscriptions mirror typed text into the song being
//! edited.

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, Context, Div, ParentElement, SharedString, Styled, Window, div, px,
};

use crate::domain::Song;
use crate::ui::components::{self, truncate};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// Mirror song metadata state into the shared inputs.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    sync_metadata_inputs(w, window, cx);
}

/// Rail wrapper for [`list_view`].
pub(crate) fn rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> gpui_kit::AnyElement {
    list_view(w, cx).into_any_element()
}

/// Preset labels offered as one-click chips on every verse card.
pub const VERSE_LABEL_PRESETS: &[&str] = &[
    "Verse 1",
    "Verse 2",
    "Verse 3",
    "Verse 4",
    "Chorus 1",
    "Chorus 2",
    "Chorus 3",
    "Bridge",
    "Pre-Chorus",
    "Intro",
    "Outro",
    "Tag",
    "Ending",
];

/// The compact preset subset shown on each verse card (the iced era showed
/// these eight; the full preset list stays available through the label input).
const VERSE_CARD_PRESETS: [&str; 8] = [
    "Verse 1", "Verse 2", "Chorus 1", "Chorus 2", "Bridge", "Intro", "Outro", "Tag",
];

/// Messages owned by the Songs feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Songs(..)`.
#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    New,
    Open(String),
    Save,
    DeleteClicked(String),
    ConfirmDelete,
    CancelDelete,
    TitleChanged(String),
    ArtistChanged(String),
    CopyrightChanged(String),
    CcliChanged(String),
    KeyChanged(String),
    BpmChanged(String),
    AddVerse,
    DeleteVerse(usize),
    MoveVerseUp(usize),
    MoveVerseDown(usize),
    VerseContentChanged(usize, String),
    VerseLabelChanged(usize, String),
    ToPresentation,
}

/// Dispatch a songs message.
pub(crate) fn update(
    w: &mut MainWindow,
    msg: Message,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match msg {
        Message::SearchChanged(q) => search_changed(w, q, cx),
        Message::New => new_song(w, window, cx),
        Message::Open(id) => open_song(w, id, window, cx),
        Message::Save => save_song(w, cx),
        Message::DeleteClicked(id) => delete_clicked(w, id, cx),
        Message::ConfirmDelete => confirm_delete(w, cx),
        Message::CancelDelete => cancel_delete(w, cx),
        Message::TitleChanged(v) => title_changed(w, v, cx),
        Message::ArtistChanged(v) => artist_changed(w, v, cx),
        Message::CopyrightChanged(v) => copyright_changed(w, v, cx),
        Message::CcliChanged(v) => ccli_changed(w, v, cx),
        Message::KeyChanged(v) => key_changed(w, v, cx),
        Message::BpmChanged(v) => bpm_changed(w, v, cx),
        Message::AddVerse => add_verse(w, window, cx),
        Message::DeleteVerse(i) => delete_verse(w, i, window, cx),
        Message::MoveVerseUp(i) => move_verse_up(w, i, window, cx),
        Message::MoveVerseDown(i) => move_verse_down(w, i, window, cx),
        Message::VerseContentChanged(i, v) => verse_content_changed(w, i, v, cx),
        Message::VerseLabelChanged(i, v) => {
            verse_label_changed(w, i, v.clone(), cx);
            // Preset chips change the label outside the input; keep the input
            // text in sync (`set_value` emits no `Change`, so no loop).
            if let Some(inputs) = w.song.verse_inputs.get(i) {
                let label = inputs.label.clone();
                label.update(cx, |state, input_cx| {
                    state.set_value(&v, window, input_cx);
                });
            }
        }
        Message::ToPresentation => to_presentation(w, window, cx),
    }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

pub(crate) fn search_changed(w: &mut MainWindow, query: String, cx: &mut Context<MainWindow>) {
    w.song.search = query.clone();
    // An empty query is not a search: it would be a query error.
    if query.trim().is_empty() {
        w.song.songs = w.song.service.list().unwrap_or_default();
        cx.notify();
        return;
    }
    match w.song.service.search(&query) {
        Ok(songs) => w.song.songs = songs,
        Err(e) => w.set_error(format!("song search error: {e}")),
    }
    cx.notify();
}

pub(crate) fn new_song(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let song = Song::new(String::from("New Song"));
    load_song_for_editing(w, song, window, cx);
}

pub(crate) fn open_song(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    // The rail's context menu closes: its backdrop covers the window, so a
    // menu that stayed up would swallow the operator's next click.
    w.shell.rail_context_target = None;
    match w.song.service.get(&id) {
        Ok(song) => {
            // The song is edited in the song editor, which the centre shows
            // in Show mode: without the switch, opening a song from the
            // search would appear to do nothing.
            w.shell.current_mode = crate::ui::messages::ViewMode::Show;
            w.shell.sidebar_tab = crate::ui::messages::SidebarTab::Songs;
            load_song_for_editing(w, song, window, cx);
        }
        Err(e) => {
            w.set_error(format!("open song error: {e}"));
            cx.notify();
        }
    }
}

/// Put a song into the editor: fill the metadata strings and inputs, then
/// rebuild the per-verse input pairs.
pub(crate) fn load_song_for_editing(
    w: &mut MainWindow,
    song: Song,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.song.edit_title = song.title.clone();
    w.song.edit_artist = song.artist.clone().unwrap_or_default();
    w.song.edit_copyright = song.copyright.clone().unwrap_or_default();
    w.song.edit_ccli = song.ccli_number.clone().unwrap_or_default();
    w.song.edit_key = song.key_signature.clone().unwrap_or_default();
    w.song.edit_bpm = song.bpm.map(|v| v.to_string()).unwrap_or_default();
    w.song.editing = Some(song);
    sync_metadata_inputs(w, window, cx);
    sync_verse_inputs(w, window, cx);
    cx.notify();
}

/// Prefill the six metadata inputs from the `song.edit_*` mirrors
/// (`set_value` emits no `Change`).
fn sync_metadata_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let values = [
        w.song.edit_title.clone(),
        w.song.edit_artist.clone(),
        w.song.edit_copyright.clone(),
        w.song.edit_ccli.clone(),
        w.song.edit_key.clone(),
        w.song.edit_bpm.clone(),
    ];
    let inputs = [
        w.song_title_input.clone(),
        w.song_artist_input.clone(),
        w.song_copyright_input.clone(),
        w.song_ccli_input.clone(),
        w.song_key_input.clone(),
        w.song_bpm_input.clone(),
    ];
    for (value, input) in values.into_iter().zip(inputs) {
        input.update(cx, |state, input_cx| {
            state.set_value(&value, window, input_cx);
        });
    }
}

/// Rebuild the per-verse label/content input pairs (and their subscriptions)
/// from the verse list of the song being edited. Called after every
/// structural change so the captured verse indices stay valid.
fn sync_verse_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let verses = w
        .song
        .editing
        .as_ref()
        .map(|s| s.verses.clone())
        .unwrap_or_default();

    let mut inputs = Vec::with_capacity(verses.len());
    for (i, verse) in verses.iter().enumerate() {
        let label = cx.new(|input_cx| {
            InputState::new(window, input_cx).placeholder(SharedString::from("Label"))
        });
        label.update(cx, |state, input_cx| {
            state.set_value(verse.label.as_str(), window, input_cx);
        });
        let content = cx.new(|input_cx| {
            InputState::new(window, input_cx).placeholder(SharedString::from("Verse text…"))
        });
        content.update(cx, |state, input_cx| {
            state.set_value(verse.content.as_str(), window, input_cx);
        });

        // Mirror typing straight into the song being edited. The closures
        // own the verse index; the pairs are rebuilt on any structural
        // change, so indices stay fresh.
        let label_entity = label.clone();
        cx.subscribe(&label, move |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let value = label_entity.read(cx).value().to_string();
                verse_label_changed(this, i, value, cx);
            }
        })
        .detach();
        let content_entity = content.clone();
        cx.subscribe(&content, move |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let value = content_entity.read(cx).value().to_string();
                verse_content_changed(this, i, value, cx);
            }
        })
        .detach();

        inputs.push(super::state::VerseInputs { label, content });
    }
    w.song.verse_inputs = inputs;
}

pub(crate) fn save_song(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(mut song) = w.song.editing.clone() else {
        return;
    };

    song.title = w.song.edit_title.clone();
    song.artist = non_empty(&w.song.edit_artist);
    song.copyright = non_empty(&w.song.edit_copyright);
    song.ccli_number = non_empty(&w.song.edit_ccli);
    song.key_signature = non_empty(&w.song.edit_key);
    song.bpm = w.song.edit_bpm.trim().parse::<u32>().ok();
    song.updated_at = chrono::Utc::now();

    for (i, v) in song.verses.iter_mut().enumerate() {
        v.order_index = i;
    }

    // The service saves the song and its verses in one transaction, so the
    // lyrics cannot be left behind.
    if let Err(e) = w.song.service.save(&song) {
        w.set_error(format!("save song error: {e}"));
        cx.notify();
        return;
    }

    w.song.editing = Some(song);
    w.load_songs();
    cx.notify();
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.shell.rail_context_target = None;
    w.song.to_delete = Some(id);
    w.editor.show_delete_confirmation = true;
    cx.notify();
}

pub(crate) fn confirm_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(id) = w.song.to_delete.take() {
        if let Err(e) = w.song.service.delete(&id) {
            w.set_error(format!("delete song error: {e}"));
        }
        if w.song.editing.as_ref().is_some_and(|s| s.id == id) {
            w.song.editing = None;
            w.song.verse_inputs.clear();
        }
        w.load_songs();
    }
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

pub(crate) fn cancel_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.song.to_delete = None;
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Metadata fields
// ---------------------------------------------------------------------------

pub(crate) fn title_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_title = v;
    cx.notify();
}

pub(crate) fn artist_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_artist = v;
    cx.notify();
}

pub(crate) fn copyright_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_copyright = v;
    cx.notify();
}

pub(crate) fn ccli_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_ccli = v;
    cx.notify();
}

pub(crate) fn key_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_key = v;
    cx.notify();
}

pub(crate) fn bpm_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    w.song.edit_bpm = v;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Verses
// ---------------------------------------------------------------------------

pub(crate) fn add_verse(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let Some(song) = w.song.editing.as_mut() else {
        return;
    };
    let idx = song.verses.len();
    let label = next_verse_label(idx);
    song.verses
        .push(crate::domain::Verse::new(label, String::new(), idx));
    sync_verse_inputs(w, window, cx);
    cx.notify();
}

pub(crate) fn delete_verse(
    w: &mut MainWindow,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if let Some(song) = w.song.editing.as_mut()
        && index < song.verses.len()
    {
        song.verses.remove(index);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    sync_verse_inputs(w, window, cx);
    cx.notify();
}

pub(crate) fn move_verse_up(
    w: &mut MainWindow,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if let Some(song) = w.song.editing.as_mut()
        && index > 0
        && index < song.verses.len()
    {
        song.verses.swap(index - 1, index);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    sync_verse_inputs(w, window, cx);
    cx.notify();
}

pub(crate) fn move_verse_down(
    w: &mut MainWindow,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if let Some(song) = w.song.editing.as_mut()
        && index + 1 < song.verses.len()
    {
        song.verses.swap(index, index + 1);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    sync_verse_inputs(w, window, cx);
    cx.notify();
}

/// Write `content` into verse `index` of the song being edited. Called from
/// the per-verse content input's subscription, so no redraw is triggered.
pub(crate) fn verse_content_changed(
    w: &mut MainWindow,
    index: usize,
    content: String,
    cx: &mut Context<MainWindow>,
) {
    if let Some(song) = w.song.editing.as_mut()
        && let Some(v) = song.verses.get_mut(index)
    {
        v.content = content;
    }
    cx.notify();
}

/// Write `label` into verse `index` of the song being edited. Called from the
/// per-verse label input's subscription and the preset-chip handlers.
pub(crate) fn verse_label_changed(
    w: &mut MainWindow,
    index: usize,
    label: String,
    cx: &mut Context<MainWindow>,
) {
    if let Some(song) = w.song.editing.as_mut()
        && let Some(v) = song.verses.get_mut(index)
    {
        v.label = label;
    }
    cx.notify();
}

/// Convert the song being edited into a presentation: save first (matching
/// the iced-era flow, so unsaved edits are included), then reuse the shared
/// root `SongToPresentation` conversion.
fn to_presentation(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    save_song(w, cx);
    let Some(song) = w.song.editing.clone() else {
        return;
    };
    w.handle_message(RootMessage::SongToPresentation(song.id), window, cx);
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn next_verse_label(existing: usize) -> String {
    match existing {
        0 => String::from("Verse 1"),
        1 => String::from("Chorus 1"),
        2 => String::from("Verse 2"),
        3 => String::from("Chorus 2"),
        4 => String::from("Bridge"),
        n => format!("Verse {n}"),
    }
}

// ---------------------------------------------------------------------------
// Views: rail song list
// ---------------------------------------------------------------------------

/// Render the song list for the left rail sidebar: search field, song rows
/// (click to open, right-click for the context menu), the OpenLyrics import
/// entry, and "+ New Song".
pub fn list_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let songs = w.song.songs.clone();
    let active_id = w.song.editing.as_ref().map(|s| s.id.clone());

    let mut list = div().flex().flex_col().gap(px(2.)).py(px(4.));
    if songs.is_empty() {
        list = list.child(components::empty_state("No songs yet"));
    }
    for song in &songs {
        let id = song.id.clone();
        let id_menu = song.id.clone();
        let is_active = active_id.as_deref() == Some(song.id.as_str());
        let artist = song.artist.clone().unwrap_or_default();

        list = list.child(
            div()
                .id(SharedString::from(format!("rail-song-{}", song.id)))
                .w_full()
                .flex()
                .flex_col()
                .gap(px(2.))
                .py(px(8.))
                .px(px(12.))
                .rounded(px(5.))
                .border_1()
                .border_color(if is_active {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    gpui_kit::transparent_black()
                })
                .bg(if is_active {
                    theme::c(theme::ACCENT_ORANGE).opacity(0.14)
                } else {
                    gpui_kit::transparent_black()
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(RootMessage::Songs(Message::Open(id.clone())), window, cx);
                }))
                .on_mouse_down(
                    gpui_kit::MouseButton::Right,
                    cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::ShowRailContextMenu(
                                crate::ui::messages::RailContextTarget::Song(id_menu.clone()),
                            ),
                            window,
                            cx,
                        );
                    }),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(truncate(&song.title, 30)),
                )
                .when(!artist.is_empty(), |el| {
                    el.child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child(truncate(&artist, 30)),
                    )
                }),
        );
    }

    let import_btn = ghost_btn(
        "songs-import-openlyrics",
        "Import OpenLyrics",
        cx.listener(|w, _, window, cx| {
            w.handle_message(
                RootMessage::ImportExport(crate::ui::import_export::Message::ImportOpenLyrics),
                window,
                cx,
            );
        }),
    );
    let new_btn = components::add_button("rail-new-song", "New Song", {
        cx.listener(|w, _, window, cx| {
            w.handle_message(RootMessage::Songs(Message::New), window, cx);
        })
    });

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .child(Input::new(&w.song_search_input.clone())),
        )
        .child(
            div()
                .id("rail-songs-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(list),
        )
        .child(import_btn)
        .child(new_btn)
}

// ---------------------------------------------------------------------------
// Views: song editor (center)
// ---------------------------------------------------------------------------

/// Render the song editor for the center workspace.
pub fn editor_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let Some(song) = w.song.editing.clone() else {
        return empty_editor();
    };
    let song_id = song.id.clone();

    // Metadata fields bound to the shared per-field input entities.
    let header = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .py(px(12.))
        .px(px(16.))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.))
                .child(div().flex_1().child(components::field_col(
                    "Title",
                    Input::new(&w.song_title_input.clone()),
                )))
                .child(div().flex_1().child(components::field_col(
                    "Artist / Author",
                    Input::new(&w.song_artist_input.clone()),
                ))),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.))
                .child(div().flex_1().child(components::field_col(
                    "Copyright",
                    Input::new(&w.song_copyright_input.clone()),
                )))
                .child(div().w(px(110.)).child(components::field_col(
                    "CCLI #",
                    Input::new(&w.song_ccli_input.clone()),
                )))
                .child(div().w(px(80.)).child(components::field_col(
                    "Key",
                    Input::new(&w.song_key_input.clone()),
                )))
                .child(div().w(px(80.)).child(components::field_col(
                    "BPM",
                    Input::new(&w.song_bpm_input.clone()),
                ))),
        );

    let actions = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pb(px(10.))
        .px(px(16.))
        .child(action_btn(
            "song-save",
            "Save",
            theme::ACCENT_ORANGE,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Songs(Message::Save), window, cx);
            }),
        ))
        .child(action_btn(
            "song-add-verse",
            "+ Verse",
            theme::TEXT_SECONDARY,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Songs(Message::AddVerse), window, cx);
            }),
        ))
        .child(action_btn(
            "song-to-presentation",
            "Send to Presentation",
            theme::TEXT_SECONDARY,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::Songs(Message::ToPresentation), window, cx);
            }),
        ))
        .child(ghost_btn(
            "song-export-openlyrics",
            "Export OpenLyrics",
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    RootMessage::ImportExport(crate::ui::import_export::Message::ExportOpenLyrics),
                    window,
                    cx,
                );
            }),
        ))
        .child(div().flex_1())
        .child(action_btn(
            "song-delete",
            "Delete Song",
            theme::DANGER_RED,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Songs(Message::DeleteClicked(song_id.clone())),
                    window,
                    cx,
                );
            }),
        ));

    let total = song.verses.len();
    let mut verses_col = div().flex().flex_col().gap(px(12.)).pb(px(16.)).px(px(16.));
    if total == 0 {
        verses_col = verses_col.child(
            div()
                .py(px(20.))
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No verses yet — click \"+ Verse\" to add one"),
        );
    } else {
        for (i, verse) in song.verses.iter().enumerate() {
            verses_col = verses_col.child(verse_card(w, i, verse, total, cx));
        }
    }

    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARKEST))
        .child(header)
        .child(components::divider())
        .child(actions)
        .child(components::divider())
        .child(
            div()
                .id("song-verses-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(verses_col),
        )
}

/// One verse card: preset label chips, the label/content input pair, and the
/// move-up / move-down / delete toolbar.
fn verse_card(
    w: &mut MainWindow,
    i: usize,
    verse: &crate::domain::Verse,
    total: usize,
    cx: &mut Context<MainWindow>,
) -> Div {
    let mut preset_row = div().flex().flex_row().flex_wrap().gap(px(4.));
    for (p, preset) in VERSE_CARD_PRESETS.into_iter().enumerate() {
        let active = verse.label == preset;
        preset_row = preset_row.child(
            div()
                .id(SharedString::from(format!("song-verse-{i}-preset-{p}")))
                .py(px(3.))
                .px(px(7.))
                .rounded(px(3.))
                .text_size(px(10.))
                .text_color(if active {
                    gpui_kit::white()
                } else {
                    theme::c(theme::TEXT_SECONDARY)
                })
                .bg(if active {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BG_HOVER)
                })
                .hover(|s| s.bg(theme::c(theme::BG_ACTIVE)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Songs(Message::VerseLabelChanged(i, preset.to_string())),
                        window,
                        cx,
                    );
                }))
                .child(preset),
        );
    }

    let mut toolbar = div().w_full().flex().flex_row().items_center().gap(px(4.));
    if let Some(inputs) = w.song.verse_inputs.get(i) {
        toolbar = toolbar.child(div().w(px(120.)).child(Input::new(&inputs.label.clone())));
    } else {
        toolbar = toolbar.child(div().w(px(120.)));
    }
    toolbar = toolbar.child(div().flex_1());
    if i > 0 {
        toolbar = toolbar.child(verse_tool_btn(
            SharedString::from(format!("song-verse-{i}-up")),
            "↑",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Songs(Message::MoveVerseUp(i)), window, cx);
            }),
        ));
    }
    if i + 1 < total {
        toolbar = toolbar.child(verse_tool_btn(
            SharedString::from(format!("song-verse-{i}-down")),
            "↓",
            cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Songs(Message::MoveVerseDown(i)), window, cx);
            }),
        ));
    }
    toolbar = toolbar.child(verse_tool_btn(
        SharedString::from(format!("song-verse-{i}-del")),
        "×",
        cx.listener(move |w, _, window, cx| {
            w.handle_message(RootMessage::Songs(Message::DeleteVerse(i)), window, cx);
        }),
    ));

    let content_input: Div = match w.song.verse_inputs.get(i) {
        Some(inputs) => div().child(Input::new(&inputs.content.clone())),
        None => div(),
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(10.))
        .rounded(px(4.))
        .bg(theme::c(theme::BG_DARK))
        .child(preset_row)
        .child(toolbar)
        .child(content_input)
}

/// Empty state shown when no song is open in the editor.
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
                .child("♪"),
        )
        .child(
            div()
                .text_size(px(14.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Select a song from the sidebar, or click + New Song"),
        )
}

// ---------------------------------------------------------------------------
// Small shared builders
// ---------------------------------------------------------------------------

/// Compact ghost text button (OpenLyrics import/export entries).
fn ghost_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(5.))
        .px(px(10.))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| {
            s.bg(theme::c(theme::BG_HOVER))
                .text_color(theme::c(theme::TEXT_PRIMARY))
        })
        .on_click(on_click)
        .child(label)
}

/// Rectangular action button tinted with `accent` (Save / + Verse / Delete).
fn action_btn(
    id: &'static str,
    label: &'static str,
    accent: u32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .py(px(6.))
        .px(px(12.))
        .rounded(px(4.))
        .text_size(px(12.))
        .text_color(theme::c(accent))
        .border_1()
        .border_color(theme::c(accent).opacity(0.6))
        .hover(|s| s.bg(theme::c(accent).opacity(0.14)))
        .on_click(on_click)
        .child(label)
}

/// Micro ghost button for the verse toolbar (move up/down, delete).
fn verse_tool_btn(
    id: SharedString,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .justify_center()
        .py(px(4.))
        .px(px(8.))
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
