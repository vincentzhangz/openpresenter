use crate::slides::{BibleTranslation, BibleVerse};
use crate::ui::components::{divider, section_header, tab_bar, tab_btn};
use crate::ui::messages::{Message, SidebarTab};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Row, Space, button, checkbox, column, container, row, scrollable, text, text_input,
    },
};
use iced_font_awesome::fa_icon_solid;

#[allow(clippy::too_many_arguments)]
pub fn bible_panel<'a>(
    translations: &'a [BibleTranslation],
    selected_translation: Option<&'a str>,
    books: &'a [String],
    selected_book: Option<&'a str>,
    chapters: &'a [i32],
    selected_chapter: Option<i32>,
    verses: &'a [BibleVerse],
    selected_verse_indices: &'a [usize],
    search: &'a str,
    verses_per_slide: usize,
    sidebar_tab: SidebarTab,
) -> Element<'a, Message> {
    let left = translation_list(translations, selected_translation, sidebar_tab);
    let right = verse_browser(
        books,
        selected_book,
        chapters,
        selected_chapter,
        verses,
        selected_verse_indices,
        search,
        verses_per_slide,
    );
    row![left, right]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn translation_list<'a>(
    translations: &'a [BibleTranslation],
    selected_id: Option<&'a str>,
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

    let header = section_header("TRANSLATIONS");

    let mut list = Column::new().spacing(2).padding([4u16, 0u16]);
    if translations.is_empty() {
        list = list.push(
            container(
                text("No translations.\nClick Import to add one.")
                    .size(12)
                    .color(theme::TEXT_MUTED),
            )
            .padding([20u16, 14u16]),
        );
    } else {
        for tr in translations {
            let is_active = selected_id == Some(tr.id.as_str());
            let row_content = row![
                text(tr.abbr.as_str()).size(13).width(50),
                text(tr.name.as_str()).size(12).color(theme::TEXT_MUTED),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let btn = button(row_content)
                .on_press(Message::BibleTranslationSelected(tr.id.clone()))
                .width(Length::Fill)
                .padding([6u16, 10u16])
                .style(if is_active {
                    theme::primary_button
                } else {
                    theme::ghost_button
                });
            list = list.push(btn);
        }
    }

    let scrollable_list = scrollable(list).height(Length::Fill);

    let import_btn = button(text("Import JSON…").size(12))
        .on_press(Message::BibleImportFile)
        .width(Length::Fill)
        .padding([8u16, 12u16])
        .style(theme::primary_button);

    let delete_btn: Element<'a, Message> = if let Some(id) = selected_id {
        button(text("Delete Translation").size(12))
            .on_press(Message::BibleDeleteTranslationClicked(id.to_string()))
            .width(Length::Fill)
            .padding([8u16, 12u16])
            .style(theme::danger_button)
            .into()
    } else {
        Space::new().width(Length::Fill).height(0).into()
    };

    column![
        tabs,
        header,
        scrollable_list,
        divider(),
        import_btn,
        delete_btn
    ]
    .width(240)
    .height(Length::Fill)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn verse_browser<'a>(
    books: &'a [String],
    selected_book: Option<&'a str>,
    chapters: &'a [i32],
    selected_chapter: Option<i32>,
    verses: &'a [BibleVerse],
    selected_verse_indices: &'a [usize],
    search: &'a str,
    verses_per_slide: usize,
) -> Element<'a, Message> {
    if books.is_empty() {
        return container(
            text("Select a translation on the left,\nor import one from a JSON file.")
                .size(14)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(theme::canvas_bg_style)
        .into();
    }

    let mut book_row = Row::new().spacing(4).padding([4u16, 8u16]);
    for book in books {
        let is_sel = selected_book == Some(book.as_str());
        book_row = book_row.push(
            button(text(book.as_str()).size(11))
                .on_press(Message::BibleBookSelected(book.clone()))
                .padding([4u16, 8u16])
                .style(if is_sel {
                    theme::primary_button
                } else {
                    theme::ghost_button
                }),
        );
    }
    let book_scroll = container(
        scrollable(book_row).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default(),
        )),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style);

    let chapter_row: Element<'a, Message> = if !chapters.is_empty() {
        let mut ch_row = Row::new().spacing(4).padding([4u16, 8u16]);
        for &ch in chapters {
            let is_sel = selected_chapter == Some(ch);
            ch_row = ch_row.push(
                button(text(ch.to_string()).size(11))
                    .on_press(Message::BibleChapterSelected(ch))
                    .padding([4u16, 6u16])
                    .style(if is_sel {
                        theme::primary_button
                    } else {
                        theme::ghost_button
                    }),
            );
        }
        container(
            scrollable(ch_row).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            )),
        )
        .width(Length::Fill)
        .style(theme::dark_panel_style)
        .into()
    } else {
        Space::new().width(Length::Fill).height(0).into()
    };

    let search_bar = container(
        text_input("Search verses…", search)
            .on_input(Message::BibleSearchChanged)
            .padding([6u16, 10u16])
            .size(13),
    )
    .padding([6u16, 8u16])
    .width(Length::Fill);

    let show_search_results = !search.trim().is_empty();
    let mut verse_col = Column::new().spacing(2).padding([2u16, 6u16]);

    if verses.is_empty() {
        let hint = if show_search_results {
            "No verses match your search."
        } else {
            "Select a book and chapter to browse verses."
        };
        verse_col = verse_col
            .push(container(text(hint).size(12).color(theme::TEXT_MUTED)).padding([16u16, 0u16]));
    } else {
        let sel_bar = row![
            button(text("All").size(11))
                .on_press(Message::BibleSelectAll)
                .padding([3u16, 8u16])
                .style(theme::ghost_button),
            button(text("None").size(11))
                .on_press(Message::BibleClearSelection)
                .padding([3u16, 8u16])
                .style(theme::ghost_button),
            Space::new().width(Length::Fill),
            text(format!("{} verses", verses.len()))
                .size(11)
                .color(theme::TEXT_MUTED),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([2u16, 4u16]);
        verse_col = verse_col.push(sel_bar);
        verse_col = verse_col.push(divider());

        for (idx, v) in verses.iter().enumerate() {
            let is_checked = selected_verse_indices.contains(&idx);
            let label = if show_search_results {
                format!("{} {}:{}", v.book, v.chapter, v.verse)
            } else {
                format!("v{}", v.verse)
            };
            let verse_ref = text(label).size(11).color(theme::TEXT_MUTED).width(60);
            let verse_text = text(v.text.as_str()).size(12);
            let cb = checkbox(is_checked)
                .on_toggle(move |_| Message::BibleVerseToggled(idx))
                .spacing(4);
            let row_elem = row![cb, verse_ref, verse_text]
                .spacing(8)
                .align_y(Alignment::Start)
                .padding([3u16, 4u16]);
            verse_col = verse_col.push(row_elem);
        }
    }

    let verse_scroll = scrollable(verse_col).height(Length::Fill);

    let sel_count = selected_verse_indices.len();
    let selected_label = if sel_count == 0 {
        "No verses selected".to_string()
    } else {
        format!(
            "{sel_count} verse{} selected",
            if sel_count > 1 { "s" } else { "" }
        )
    };

    let vps_dec = button(text("−").size(13))
        .on_press(Message::BibleVersesPerSlideChanged(
            verses_per_slide.saturating_sub(1).max(1),
        ))
        .padding([4u16, 8u16])
        .style(theme::ghost_button);

    let vps_inc = button(text("+").size(13))
        .on_press(Message::BibleVersesPerSlideChanged(
            (verses_per_slide + 1).min(10),
        ))
        .padding([4u16, 8u16])
        .style(theme::ghost_button);

    let vps_label = text(format!("Verses/slide: {verses_per_slide}")).size(12);

    let send_btn = button(
        row![
            fa_icon_solid("share").size(13.0),
            text(" Send to Presentation").size(13),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(Message::BibleSendToPresentation)
    .padding([8u16, 14u16])
    .style(theme::primary_button);

    let action_bar = container(
        row![
            text(selected_label).size(12).color(theme::TEXT_MUTED),
            Space::new().width(Length::Fill),
            vps_dec,
            vps_label,
            vps_inc,
            send_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([8u16, 12u16]),
    )
    .width(Length::Fill)
    .style(theme::dark_panel_style);

    column![
        book_scroll,
        chapter_row,
        search_bar,
        divider(),
        verse_scroll,
        divider(),
        action_bar,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
