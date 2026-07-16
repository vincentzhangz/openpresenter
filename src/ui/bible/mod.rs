use crate::domain::{
    BibleImportFile, BibleTranslation, BibleVerse, Presentation, Slide, Transition,
};
use crate::ui::components::{divider, section_header, tab_bar, tab_btn};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab, ViewMode};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length, Task,
    widget::{
        Column, Row, Space, button, checkbox, column, container, row, scrollable, text, text_input,
    },
};
use iced_font_awesome::fa_icon_solid;
use uuid::Uuid;

/// Messages owned by the Bible feature module (see `AGENTS.md`).
#[derive(Debug, Clone)]
pub enum Message {
    TranslationSelected(String),
    BookSelected(String),
    ChapterSelected(i32),
    VerseToggled(usize),
    SearchChanged(String),
    VersesPerSlideChanged(usize),
    SendToPresentation,
    ImportFile,
    DeleteTranslationClicked(String),
    ConfirmDeleteTranslation,
    CancelDeleteTranslation,
    SelectAll,
    ClearSelection,
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Bible(msg)
}

/// Render the bible panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    bible_panel(
        &w.bible.translations,
        w.bible.selected_translation.as_deref(),
        &w.bible.books,
        w.bible.selected_book.as_deref(),
        &w.bible.chapters,
        w.bible.selected_chapter,
        &w.bible.verses,
        &w.bible.selected_verse_indices,
        &w.bible.search,
        w.bible.verses_per_slide,
        w.shell.sidebar_tab,
    )
}

/// Dispatch a bible message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::TranslationSelected(id) => translation_selected(w, id),
        Message::BookSelected(book) => book_selected(w, book),
        Message::ChapterSelected(ch) => chapter_selected(w, ch),
        Message::VerseToggled(idx) => verse_toggled(w, idx),
        Message::SearchChanged(q) => search_changed(w, q),
        Message::VersesPerSlideChanged(n) => verses_per_slide_changed(w, n),
        Message::SendToPresentation => send_to_presentation(w),
        Message::ImportFile => import_file(w),
        Message::DeleteTranslationClicked(id) => delete_clicked(w, id),
        Message::ConfirmDeleteTranslation => confirm_delete(w),
        Message::CancelDeleteTranslation => cancel_delete(w),
        Message::SelectAll => select_all(w),
        Message::ClearSelection => clear_selection(w),
    }
}

pub(crate) fn translation_selected(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.bible.selected_translation = Some(id.clone());
    w.bible.selected_book = None;
    w.bible.selected_chapter = None;
    w.bible.books.clear();
    w.bible.chapters.clear();
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();

    match w.bible.repo.list_books(&id) {
        Ok(books) => w.bible.books = books,
        Err(e) => w.set_error(format!("bible list_books: {e}")),
    }
    Task::none()
}

pub(crate) fn book_selected(w: &mut MainWindow, book: String) -> Task<RootMessage> {
    w.bible.selected_book = Some(book.clone());
    w.bible.selected_chapter = None;
    w.bible.chapters.clear();
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();

    if let Some(tid) = w.bible.selected_translation.clone() {
        match w.bible.repo.list_chapters(&tid, &book) {
            Ok(chs) => {
                w.bible.chapters = chs.clone();
                if let Some(&first) = chs.first() {
                    return chapter_selected(w, first);
                }
            }
            Err(e) => w.set_error(format!("bible list_chapters: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn chapter_selected(w: &mut MainWindow, chapter: i32) -> Task<RootMessage> {
    w.bible.selected_chapter = Some(chapter);
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();

    if let (Some(tid), Some(book)) = (
        w.bible.selected_translation.clone(),
        w.bible.selected_book.clone(),
    ) {
        match w.bible.repo.get_chapter_verses(&tid, &book, chapter) {
            Ok(vs) => w.bible.verses = vs,
            Err(e) => w.set_error(format!("bible get_chapter_verses: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn search_changed(w: &mut MainWindow, query: String) -> Task<RootMessage> {
    w.bible.search = query.clone();
    w.bible.selected_verse_indices.clear();

    if query.trim().is_empty() {
        if let (Some(tid), Some(book), Some(ch)) = (
            w.bible.selected_translation.clone(),
            w.bible.selected_book.clone(),
            w.bible.selected_chapter,
        ) {
            match w.bible.repo.get_chapter_verses(&tid, &book, ch) {
                Ok(vs) => w.bible.verses = vs,
                Err(e) => w.set_error(format!("bible restore chapter: {e}")),
            }
        } else {
            w.bible.verses.clear();
        }
    } else if let Some(tid) = w.bible.selected_translation.clone() {
        match w.bible.repo.search_verses(&tid, &query) {
            Ok(vs) => w.bible.verses = vs,
            Err(e) => w.set_error(format!("bible search: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn verse_toggled(w: &mut MainWindow, idx: usize) -> Task<RootMessage> {
    if let Some(pos) = w
        .bible
        .selected_verse_indices
        .iter()
        .position(|&i| i == idx)
    {
        w.bible.selected_verse_indices.remove(pos);
    } else {
        w.bible.selected_verse_indices.push(idx);
        w.bible.selected_verse_indices.sort_unstable();
    }
    Task::none()
}

pub(crate) fn select_all(w: &mut MainWindow) -> Task<RootMessage> {
    w.bible.selected_verse_indices = (0..w.bible.verses.len()).collect();
    Task::none()
}

pub(crate) fn clear_selection(w: &mut MainWindow) -> Task<RootMessage> {
    w.bible.selected_verse_indices.clear();
    Task::none()
}

pub(crate) fn verses_per_slide_changed(w: &mut MainWindow, n: usize) -> Task<RootMessage> {
    w.bible.verses_per_slide = n.clamp(1, 10);
    Task::none()
}

pub(crate) fn send_to_presentation(w: &mut MainWindow) -> Task<RootMessage> {
    if w.bible.selected_verse_indices.is_empty() {
        return Task::none();
    }

    let selected: Vec<_> = w
        .bible
        .selected_verse_indices
        .iter()
        .filter_map(|&i| w.bible.verses.get(i))
        .cloned()
        .collect();

    if selected.is_empty() {
        return Task::none();
    }

    let first = &selected[0];
    let last = &selected[selected.len() - 1];
    let pres_name = if first.book == last.book && first.chapter == last.chapter {
        format!(
            "{} {}:{}-{}",
            first.book, first.chapter, first.verse, last.verse
        )
    } else {
        format!(
            "{} {}:{} – {} {}:{}",
            first.book, first.chapter, first.verse, last.book, last.chapter, last.verse
        )
    };

    let vps = w.bible.verses_per_slide;

    let chunks: Vec<Vec<_>> = selected.chunks(vps).map(|c| c.to_vec()).collect();

    let slides: Vec<Slide> = chunks
        .iter()
        .map(|chunk| {
            let verse_lines: Vec<String> = chunk
                .iter()
                .map(|v| format!("{}  {}", v.verse, v.text))
                .collect();
            let text = verse_lines.join("\n\n");

            let f = &chunk[0];
            let l = &chunk[chunk.len() - 1];
            let group = if f.verse == l.verse {
                format!("{} {}:{}", f.book, f.chapter, f.verse)
            } else {
                format!("{} {}:{}-{}", f.book, f.chapter, f.verse, l.verse)
            };

            let mut slide = Slide::new_text_in_group(text, group);
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
                    w.set_error(format!("bible send_to_presentation replace: {e}"));
                }
                w.editor.editing = Some(Presentation {
                    id: new_pres.id.clone(),
                    name: new_pres.name,
                    slides: pres.slides,
                    created_at: new_pres.created_at,
                    updated_at: new_pres.updated_at,
                });
                w.editor.selected_slide_index = Some(0);
                w.shell.current_mode = ViewMode::Edit;
                w.shell.sidebar_tab = SidebarTab::Presentations;
                w.load_slide_for_editing();
            }
        }
        Err(e) => w.set_error(format!("bible send_to_presentation create: {e}")),
    }
    Task::none()
}

pub(crate) fn import_file(w: &mut MainWindow) -> Task<RootMessage> {
    let path = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Import Bible Translation (JSON)")
        .pick_file();

    let Some(path) = path else {
        return Task::none();
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            w.set_error(format!("bible import read: {e}"));
            return Task::none();
        }
    };

    let file: BibleImportFile = match serde_json::from_str(&raw) {
        Ok(f) => f,
        Err(e) => {
            w.set_error(format!("bible import parse: {e}"));
            return Task::none();
        }
    };

    let abbr = file.abbr.clone();
    match w.bible.repo.import_translation(file) {
        Ok(tr) => {
            eprintln!("Imported Bible translation: {} ({})", tr.name, abbr);
            w.load_bible_translations();
        }
        Err(e) => w.set_error(format!("bible import save: {e}")),
    }
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<RootMessage> {
    w.bible.translation_to_delete = Some(id);
    Task::none()
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.bible.translation_to_delete.take() {
        if let Err(e) = w.bible.repo.delete_translation(&id) {
            w.set_error(format!("bible delete: {e}"));
        }
        if w.bible.selected_translation.as_deref() == Some(&id) {
            w.bible.selected_translation = None;
            w.bible.books.clear();
            w.bible.chapters.clear();
            w.bible.verses.clear();
            w.bible.selected_verse_indices.clear();
        }
        w.load_bible_translations();
    }
    Task::none()
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<RootMessage> {
    w.bible.translation_to_delete = None;
    Task::none()
}

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
) -> Element<'a, RootMessage> {
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
                .on_press(wrap(Message::TranslationSelected(tr.id.clone())))
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
        .on_press(wrap(Message::ImportFile))
        .width(Length::Fill)
        .padding([8u16, 12u16])
        .style(theme::primary_button);

    let delete_btn: Element<'a, RootMessage> = if let Some(id) = selected_id {
        button(text("Delete Translation").size(12))
            .on_press(wrap(Message::DeleteTranslationClicked(id.to_string())))
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
) -> Element<'a, RootMessage> {
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
                .on_press(wrap(Message::BookSelected(book.clone())))
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

    let chapter_row: Element<'a, RootMessage> = if !chapters.is_empty() {
        let mut ch_row = Row::new().spacing(4).padding([4u16, 8u16]);
        for &ch in chapters {
            let is_sel = selected_chapter == Some(ch);
            ch_row = ch_row.push(
                button(text(ch.to_string()).size(11))
                    .on_press(wrap(Message::ChapterSelected(ch)))
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
            .on_input(move |v| wrap(Message::SearchChanged(v)))
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
                .on_press(wrap(Message::SelectAll))
                .padding([3u16, 8u16])
                .style(theme::ghost_button),
            button(text("None").size(11))
                .on_press(wrap(Message::ClearSelection))
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
                .on_toggle(move |_| wrap(Message::VerseToggled(idx)))
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
        .on_press(wrap(Message::VersesPerSlideChanged(
            verses_per_slide.saturating_sub(1).max(1),
        )))
        .padding([4u16, 8u16])
        .style(theme::ghost_button);

    let vps_inc = button(text("+").size(13))
        .on_press(wrap(Message::VersesPerSlideChanged(
            (verses_per_slide + 1).min(10),
        )))
        .padding([4u16, 8u16])
        .style(theme::ghost_button);

    let vps_label = text(format!("Verses/slide: {verses_per_slide}")).size(12);

    let send_btn = button(
        row![
            fa_icon_solid("share").size(13.0_f32),
            text(" Send to Presentation").size(13),
        ]
        .align_y(Alignment::Center),
    )
    .on_press(wrap(Message::SendToPresentation))
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
