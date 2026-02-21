use crate::slides::{Song, Verse};
use crate::ui::components::{
    add_button, divider, field_col, search_input, section_header, tab_bar, tab_btn,
};
use crate::ui::messages::{Message, SidebarTab};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Column, Row, Space, button, column, container, row, scrollable, text, text_input},
};

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

#[allow(clippy::too_many_arguments)]
pub fn songs_panel<'a>(
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
) -> Element<'a, Message> {
    let list = song_list(songs, search, editing.map(|s| s.id.as_str()), sidebar_tab);

    let editor: Element<'a, Message> = if let Some(song) = editing {
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
) -> Element<'a, Message> {
    let tabs = tab_bar(vec![
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

    let header = section_header("SONGS");

    let search_bar = search_input("Search songs...", search, Message::SongSearchChanged);

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

            let artist_el: Element<'_, Message> = match &song.artist {
                Some(a) => text(a.clone()).size(11).color(theme::TEXT_MUTED).into(),
                None => Space::new().height(0).into(),
            };

            let card_content = column![text(song.title.clone()).size(13), artist_el].spacing(2);

            list_col = list_col.push(
                button(card_content)
                    .on_press(Message::OpenSong(song.id.clone()))
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
            add_button("New Song", Message::NewSong),
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
) -> Element<'a, Message> {
    let metadata_row = row![
        field_col(
            "CCLI #",
            text_input("000000", edit_ccli).on_input(Message::SongCcliChanged)
        ),
        field_col(
            "Key",
            text_input("G", edit_key).on_input(Message::SongKeyChanged)
        ),
        field_col(
            "BPM",
            text_input("120", edit_bpm).on_input(Message::SongBpmChanged)
        ),
    ]
    .spacing(8);

    let header = column![
        field_col(
            "Title",
            text_input("Song title", edit_title).on_input(Message::SongTitleChanged),
        ),
        field_col(
            "Artist / Author",
            text_input("Artist or author", edit_artist).on_input(Message::SongArtistChanged),
        ),
        field_col(
            "Copyright",
            text_input("(c) Year Author", edit_copyright).on_input(Message::SongCopyrightChanged),
        ),
        metadata_row,
    ]
    .spacing(6)
    .padding([12u16, 16u16]);

    let actions = row![
        button(text("Save").size(13))
            .on_press(Message::SaveSong)
            .style(theme::primary_button),
        button(text("+ Verse").size(13))
            .on_press(Message::AddVerse)
            .style(theme::secondary_button),
        button(text("Send to Presentation").size(13))
            .on_press(Message::SongToPresentation)
            .style(theme::secondary_button),
        Space::new().width(Length::Fill),
        button(text("Delete Song").size(12))
            .on_press(Message::DeleteSongClicked(song.id.clone()))
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

fn verse_card(i: usize, verse: &Verse, total: usize) -> Element<'_, Message> {
    let core: &[&str] = &[
        "Verse 1", "Verse 2", "Chorus 1", "Chorus 2", "Bridge", "Intro", "Outro", "Tag",
    ];
    let preset_buttons: Vec<Element<'_, Message>> = core
        .iter()
        .map(|&lbl| {
            let active = verse.label == lbl;
            let sty = if active {
                theme::primary_button
            } else {
                theme::secondary_button
            };
            button(text(lbl).size(10))
                .on_press(Message::VerseLabelChanged(i, lbl.to_string()))
                .padding([3u16, 7u16])
                .style(sty)
                .into()
        })
        .collect();
    let preset_row = Row::with_children(preset_buttons).spacing(4).wrap();

    let label_input = text_input("Label", &verse.label)
        .on_input(move |v| Message::VerseLabelChanged(i, v))
        .padding([5u16, 8u16])
        .size(13);

    let content_input = text_input("Verse text...", &verse.content)
        .on_input(move |v| Message::VerseContentChanged(i, v))
        .padding([8u16, 10u16])
        .size(14);

    let up_el: Element<'_, Message> = if i > 0 {
        button(text("^").size(11))
            .on_press(Message::MoveVerseUp(i))
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let down_el: Element<'_, Message> = if i + 1 < total {
        button(text("v").size(11))
            .on_press(Message::MoveVerseDown(i))
            .style(theme::secondary_button)
            .padding([4u16, 8u16])
            .into()
    } else {
        Space::new().width(32).into()
    };

    let del = button(text("x").size(11))
        .on_press(Message::DeleteVerse(i))
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
