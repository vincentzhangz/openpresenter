use crate::domain::{Presentation, Slide, Song, Transition, Verse};
use crate::ui::components::{
    add_button, divider, field_col, search_input, section_header, tab_bar, tab_btn,
};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{Column, Row, Space, button, column, container, row, scrollable, text, text_input},
};
use uuid::Uuid;

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

/// Messages owned by the Songs feature module (see `AGENTS.md`).
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

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Songs(msg)
}

/// Render the songs panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    songs_panel(
        &w.song.songs,
        &w.song.search,
        w.song.editing.as_ref(),
        &w.song.edit_title,
        &w.song.edit_artist,
        &w.song.edit_copyright,
        &w.song.edit_ccli,
        &w.song.edit_key,
        &w.song.edit_bpm,
        w.shell.sidebar_tab,
    )
}

/// Dispatch a songs message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::SearchChanged(q) => search_changed(w, q),
        Message::New => new_song(w),
        Message::Open(id) => open_song(w, id),
        Message::Save => save_song(w),
        Message::DeleteClicked(id) => delete_clicked(w, id),
        Message::ConfirmDelete => confirm_delete(w),
        Message::CancelDelete => cancel_delete(w),
        Message::TitleChanged(v) => title_changed(w, v),
        Message::ArtistChanged(v) => artist_changed(w, v),
        Message::CopyrightChanged(v) => copyright_changed(w, v),
        Message::CcliChanged(v) => ccli_changed(w, v),
        Message::KeyChanged(v) => key_changed(w, v),
        Message::BpmChanged(v) => bpm_changed(w, v),
        Message::AddVerse => add_verse(w),
        Message::DeleteVerse(i) => delete_verse(w, i),
        Message::MoveVerseUp(i) => move_verse_up(w, i),
        Message::MoveVerseDown(i) => move_verse_down(w, i),
        Message::VerseContentChanged(i, v) => verse_content_changed(w, i, v),
        Message::VerseLabelChanged(i, v) => verse_label_changed(w, i, v),
        Message::ToPresentation => song_to_presentation(w),
    }
}

pub(crate) fn search_changed(w: &mut MainWindow, query: String) -> Task<RootMessage> {
    w.song.search = query.clone();
    match w.song.repo.search_songs(&query) {
        Ok(songs) => w.song.songs = songs,
        Err(e) => w.set_error(format!("song search error: {e}")),
    }
    Task::none()
}

pub(crate) fn new_song(w: &mut MainWindow) -> Task<RootMessage> {
    let song = crate::domain::Song::new("New Song".to_string());
    w.load_song_for_editing(song);
    Task::none()
}

pub(crate) fn open_song(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    match w.song.repo.get_song(&id) {
        Ok(song) => w.load_song_for_editing(song),
        Err(e) => w.set_error(format!("open song error: {e}")),
    }
    Task::none()
}

pub(crate) fn save_song(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(mut song) = w.song.editing.clone() else {
        return Task::none();
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

    if let Err(e) = w.song.repo.save_song(&song) {
        w.set_error(format!("save song error: {e}"));
        return Task::none();
    }
    if let Err(e) = w.song.repo.replace_verses(&song.id, &song.verses) {
        w.set_error(format!("save verses error: {e}"));
        return Task::none();
    }

    w.song.editing = Some(song);
    w.load_songs();
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    crate::ui::editor::delete_confirm::open_delete_dialog(
        w,
        crate::ui::editor::delete_confirm::DeleteTarget::Song,
        id,
    )
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.song.to_delete.take() {
        if let Err(e) = w.song.repo.delete_song(&id) {
            w.set_error(format!("delete song error: {e}"));
        }
        if w.song.editing.as_ref().is_some_and(|s| s.id == id) {
            w.song.editing = None;
        }
        w.load_songs();
    }
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<RootMessage> {
    w.song.to_delete = None;
    crate::ui::editor::delete_confirm::close_delete_dialog(w)
}

pub(crate) fn title_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_title = v;
    Task::none()
}
pub(crate) fn artist_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_artist = v;
    Task::none()
}
pub(crate) fn copyright_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_copyright = v;
    Task::none()
}
pub(crate) fn ccli_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_ccli = v;
    Task::none()
}
pub(crate) fn key_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_key = v;
    Task::none()
}
pub(crate) fn bpm_changed(w: &mut MainWindow, v: String) -> Task<RootMessage> {
    w.song.edit_bpm = v;
    Task::none()
}

pub(crate) fn add_verse(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(song) = w.song.editing.as_mut() else {
        return Task::none();
    };
    let idx = song.verses.len();
    let label = next_verse_label(idx);
    song.verses.push(Verse::new(label, String::new(), idx));
    Task::none()
}

pub(crate) fn delete_verse(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    let Some(song) = w.song.editing.as_mut() else {
        return Task::none();
    };
    if index < song.verses.len() {
        song.verses.remove(index);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    Task::none()
}

pub(crate) fn move_verse_up(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    let Some(song) = w.song.editing.as_mut() else {
        return Task::none();
    };
    if index > 0 && index < song.verses.len() {
        song.verses.swap(index - 1, index);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    Task::none()
}

pub(crate) fn move_verse_down(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    let Some(song) = w.song.editing.as_mut() else {
        return Task::none();
    };
    if index + 1 < song.verses.len() {
        song.verses.swap(index, index + 1);
        for (i, v) in song.verses.iter_mut().enumerate() {
            v.order_index = i;
        }
    }
    Task::none()
}

pub(crate) fn verse_content_changed(
    w: &mut MainWindow,
    index: usize,
    content: String,
) -> Task<RootMessage> {
    if let Some(song) = w.song.editing.as_mut()
        && let Some(v) = song.verses.get_mut(index)
    {
        v.content = content;
    }
    Task::none()
}

pub(crate) fn verse_label_changed(
    w: &mut MainWindow,
    index: usize,
    label: String,
) -> Task<RootMessage> {
    if let Some(song) = w.song.editing.as_mut()
        && let Some(v) = song.verses.get_mut(index)
    {
        v.label = label;
    }
    Task::none()
}

pub(crate) fn song_to_presentation(w: &mut MainWindow) -> Task<RootMessage> {
    let _ = save_song(w);

    let Some(song) = w.song.editing.clone() else {
        return Task::none();
    };

    let pres_name = song.title.clone();
    let slides: Vec<Slide> = song
        .verses
        .iter()
        .map(|verse| {
            let mut slide = Slide::new_text_in_group(verse.content.clone(), verse.label.clone());
            slide.transition = Transition::Fade { duration_ms: 500 };
            slide
        })
        .collect();

    let pres = Presentation {
        id: Uuid::new_v4().to_string(),
        name: pres_name.clone(),
        slides,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    match w.services.presentations.create(&pres_name) {
        Ok(_) => {
            w.load_presentations();
            if let Some(new_pres) = w.editor.presentations.first().cloned() {
                if let Err(e) = w
                    .services
                    .presentations
                    .replace_slides(&new_pres.id, &pres.slides)
                {
                    w.set_error(format!("song_to_presentation replace slides: {e}"));
                }
                w.editor.editing = Some(Presentation {
                    id: new_pres.id,
                    name: new_pres.name,
                    slides: pres.slides,
                    created_at: new_pres.created_at,
                    updated_at: new_pres.updated_at,
                });
                w.editor.selected_slide_index =
                    if w.editor.editing.as_ref().map_or(0, |p| p.slides.len()) > 0 {
                        Some(0)
                    } else {
                        None
                    };
                w.shell.current_mode = crate::ui::messages::ViewMode::Edit;
                w.shell.sidebar_tab = crate::ui::messages::SidebarTab::Presentations;
                w.load_slide_for_editing();
            }
        }
        Err(e) => w.set_error(format!("song_to_presentation create: {e}")),
    }

    Task::none()
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
        0 => "Verse 1".to_string(),
        1 => "Chorus 1".to_string(),
        2 => "Verse 2".to_string(),
        3 => "Chorus 2".to_string(),
        4 => "Bridge".to_string(),
        n => format!("Verse {}", n),
    }
}

#[allow(clippy::too_many_arguments)]
fn songs_panel<'a>(
    songs: &'a [Song],
    search: &'a str,
    editing: Option<&'a Song>,
    edit_title: &'a str,
    edit_artist: &'a str,
    edit_copyright: &'a str,
    edit_ccli: &'a str,
    edit_key: &'a str,
    edit_bpm: &'a str,
    sidebar_tab: SidebarTab,
) -> Element<'a, RootMessage> {
    let list = song_list(songs, search, editing.map(|s| s.id.as_str()), sidebar_tab);

    let editor: Element<'a, RootMessage> = if let Some(song) = editing {
        song_editor(
            song,
            edit_title,
            edit_artist,
            edit_copyright,
            edit_ccli,
            edit_key,
            edit_bpm,
        )
    } else {
        container(
            text("Select a song or create a new one")
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

fn song_list<'a>(
    songs: &'a [Song],
    search: &'a str,
    active_id: Option<&'a str>,
    sidebar_tab: SidebarTab,
) -> Element<'a, RootMessage> {
    let tabs = tab_bar(vec![
        tab_btn(
            "Slides",
            sidebar_tab == SidebarTab::Presentations,
            RootMessage::SwitchSidebarTab(SidebarTab::Presentations),
        ),
        tab_btn(
            "Library",
            sidebar_tab == SidebarTab::Library,
            RootMessage::SwitchSidebarTab(SidebarTab::Library),
        ),
        tab_btn(
            "Songs",
            sidebar_tab == SidebarTab::Songs,
            RootMessage::SwitchSidebarTab(SidebarTab::Songs),
        ),
        tab_btn(
            "Bible",
            sidebar_tab == SidebarTab::Bible,
            RootMessage::SwitchSidebarTab(SidebarTab::Bible),
        ),
    ]);

    let header = section_header("SONGS");

    let search_bar = search_input("Search songs...", search, move |v| {
        wrap(Message::SearchChanged(v))
    });

    let mut list_col = Column::new().spacing(2).padding([4u16, 0u16]);

    if songs.is_empty() {
        list_col = list_col.push(crate::ui::components::empty_state("No songs yet"));
    } else {
        for song in songs {
            let is_active = active_id == Some(song.id.as_str());
            let card_style = if is_active {
                theme::primary_button
            } else {
                theme::ghost_button
            };

            let artist_el: Element<'_, RootMessage> = match &song.artist {
                Some(a) => text(a.clone()).size(11).color(theme::TEXT_MUTED).into(),
                None => Space::new().height(0).into(),
            };

            let card_content = column![text(song.title.clone()).size(13), artist_el].spacing(2);

            list_col = list_col.push(
                button(card_content)
                    .on_press(wrap(Message::Open(song.id.clone())))
                    .padding([8u16, 12u16])
                    .width(Length::Fill)
                    .style(card_style),
            );
        }
    }

    container(
        column![
            tabs,
            header,
            search_bar,
            scrollable(list_col)
                .height(Length::Fill)
                .width(Length::Fill),
            add_button("New Song", wrap(Message::New)),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(240)
    .height(Length::Fill)
    .style(theme::panel_style)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn song_editor<'a>(
    song: &'a Song,
    edit_title: &'a str,
    edit_artist: &'a str,
    edit_copyright: &'a str,
    edit_ccli: &'a str,
    edit_key: &'a str,
    edit_bpm: &'a str,
) -> Element<'a, RootMessage> {
    let metadata_row = row![
        field_col(
            "CCLI #",
            text_input("000000", edit_ccli).on_input(move |v| wrap(Message::CcliChanged(v)))
        ),
        field_col(
            "Key",
            text_input("G", edit_key).on_input(move |v| wrap(Message::KeyChanged(v)))
        ),
        field_col(
            "BPM",
            text_input("120", edit_bpm).on_input(move |v| wrap(Message::BpmChanged(v)))
        ),
    ]
    .spacing(8);

    let header = column![
        field_col(
            "Title",
            text_input("Song title", edit_title).on_input(move |v| wrap(Message::TitleChanged(v))),
        ),
        field_col(
            "Artist / Author",
            text_input("Artist or author", edit_artist)
                .on_input(move |v| wrap(Message::ArtistChanged(v))),
        ),
        field_col(
            "Copyright",
            text_input("(c) Year Author", edit_copyright)
                .on_input(move |v| wrap(Message::CopyrightChanged(v))),
        ),
        metadata_row,
    ]
    .spacing(6)
    .padding([12u16, 16u16]);

    let actions = row![
        button(text("Save").size(13))
            .on_press(wrap(Message::Save))
            .style(theme::primary_button),
        button(text("+ Verse").size(13))
            .on_press(wrap(Message::AddVerse))
            .style(theme::secondary_button),
        button(text("Send to Presentation").size(13))
            .on_press(wrap(Message::ToPresentation))
            .style(theme::secondary_button),
        Space::new().width(Length::Fill),
        button(text("Delete Song").size(12))
            .on_press(wrap(Message::DeleteClicked(song.id.clone())))
            .style(theme::danger_button),
    ]
    .spacing(8)
    .padding(Padding {
        top: 0.0,
        right: 16.0,
        bottom: 10.0,
        left: 16.0,
    })
    .align_y(Alignment::Center);

    let total = song.verses.len();
    let mut verses_col = Column::new().spacing(12).padding(Padding {
        top: 0.0,
        right: 16.0,
        bottom: 16.0,
        left: 16.0,
    });

    if song.verses.is_empty() {
        verses_col = verses_col.push(
            container(
                text("No verses yet -- click \"+ Verse\" to add one")
                    .size(13)
                    .color(theme::TEXT_MUTED),
            )
            .padding([20u16, 0u16]),
        );
    } else {
        for (i, verse) in song.verses.iter().enumerate() {
            verses_col = verses_col.push(verse_card(i, verse, total));
        }
    }

    container(
        column![
            header,
            divider(),
            actions,
            divider(),
            scrollable(verses_col).height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::canvas_bg_style)
    .into()
}

fn verse_card(i: usize, verse: &Verse, total: usize) -> Element<'_, RootMessage> {
    let core: &[&str] = &[
        "Verse 1", "Verse 2", "Chorus 1", "Chorus 2", "Bridge", "Intro", "Outro", "Tag",
    ];
    let preset_buttons: Vec<Element<'_, RootMessage>> = core
        .iter()
        .map(|&lbl| {
            let active = verse.label == lbl;
            let sty = if active {
                theme::primary_button
            } else {
                theme::secondary_button
            };
            button(text(lbl).size(10))
                .on_press(wrap(Message::VerseLabelChanged(i, lbl.to_string())))
                .padding([3u16, 7u16])
                .style(sty)
                .into()
        })
        .collect();
    let preset_row = Row::with_children(preset_buttons).spacing(4).wrap();

    let label_input = text_input("Label", &verse.label)
        .on_input(move |v| wrap(Message::VerseLabelChanged(i, v)))
        .padding([5u16, 8u16])
        .size(13);

    let content_input = text_input("Verse text...", &verse.content)
        .on_input(move |v| wrap(Message::VerseContentChanged(i, v)))
        .padding([8u16, 10u16])
        .size(14);

    let up_el: Element<'_, RootMessage> = if i > 0 {
        button(text("^").size(11))
            .on_press(wrap(Message::MoveVerseUp(i)))
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let down_el: Element<'_, RootMessage> = if i + 1 < total {
        button(text("v").size(11))
            .on_press(wrap(Message::MoveVerseDown(i)))
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let del = button(text("x").size(11))
        .on_press(wrap(Message::DeleteVerse(i)))
        .style(theme::danger_button)
        .padding([4u16, 8u16]);

    let toolbar = row![
        label_input,
        Space::new().width(Length::Fill),
        up_el,
        down_el,
        del
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    container(column![preset_row, toolbar, content_input].spacing(6))
        .padding([10u16, 10u16])
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
}
