//! Bible panel: translation list + JSON import/delete (left rail) and the
//! scripture verse browser (book/chapter navigation, verse multi-select,
//! search, verses-per-slide chunking, Send to Presentation).
//!
//! Ported from the iced-era `ui/bible/mod.rs`. The in-browser search field is
//! a GPUI [`gpui_kit::component::input::Input`] entity owned by
//! [`crate::ui::main_window::MainWindow`]; its subscription mirrors typed
//! text into the query (which reloads either the chapter or the search
//! results). Verse selection toggles on row click; the checkbox square is a
//! visual indicator of [`BibleState::selected_verse_indices`].

use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, Context, Div, IntoElement, ParentElement, SharedString, Styled, Window, div,
    px,
};

use crate::domain::{BibleImportFile, BibleTranslation, Presentation, Slide, Transition};
use crate::ui::components::{self, truncate};
use crate::ui::import_export::Done;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab, ViewMode};
use crate::ui::theme;

/// Mirror the search state into the shared search input.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let query = w.bible.search.clone();
    crate::ui::components::sync_text_field(&w.bible_search_input, &query, window, cx);
}

/// Rail wrapper for [`list_view`].
pub(crate) fn rail(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> gpui_kit::AnyElement {
    list_view(w, cx).into_any_element()
}

/// Messages owned by the Bible feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Bible(..)`.
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

/// Dispatch a bible message.
pub(crate) fn update(
    w: &mut MainWindow,
    msg: Message,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match msg {
        Message::TranslationSelected(id) => translation_selected(w, id, window, cx),
        Message::BookSelected(book) => book_selected(w, book, window, cx),
        Message::ChapterSelected(ch) => chapter_selected(w, ch, window, cx),
        Message::VerseToggled(idx) => verse_toggled(w, idx, cx),
        Message::SearchChanged(q) => search_changed(w, q, cx),
        Message::VersesPerSlideChanged(n) => verses_per_slide_changed(w, n, cx),
        Message::SendToPresentation => send_to_presentation(w, window, cx),
        Message::ImportFile => import_file(w, cx),
        Message::DeleteTranslationClicked(id) => delete_clicked(w, id, cx),
        Message::ConfirmDeleteTranslation => confirm_delete(w, cx),
        Message::CancelDeleteTranslation => cancel_delete(w, cx),
        Message::SelectAll => select_all(w, cx),
        Message::ClearSelection => clear_selection(w, cx),
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

pub(crate) fn translation_selected(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.bible.selected_translation = Some(id.clone());
    w.bible.selected_book = None;
    w.bible.selected_chapter = None;
    w.bible.books.clear();
    w.bible.chapters.clear();
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();
    clear_search_input(w, window, cx);

    match w.bible.service.list_books(&id) {
        Ok(books) => w.bible.books = books,
        Err(e) => w.set_error(format!("bible list_books: {e}")),
    }
    cx.notify();
}

pub(crate) fn book_selected(
    w: &mut MainWindow,
    book: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.bible.selected_book = Some(book.clone());
    w.bible.selected_chapter = None;
    w.bible.chapters.clear();
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();
    clear_search_input(w, window, cx);

    if let Some(tid) = w.bible.selected_translation.clone() {
        match w.bible.service.list_chapters(&tid, &book) {
            Ok(chs) => {
                w.bible.chapters = chs.clone();
                if let Some(&first) = chs.first() {
                    chapter_selected(w, first, window, cx);
                    return;
                }
            }
            Err(e) => w.set_error(format!("bible list_chapters: {e}")),
        }
    }
    cx.notify();
}

pub(crate) fn chapter_selected(
    w: &mut MainWindow,
    chapter: i32,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.bible.selected_chapter = Some(chapter);
    w.bible.verses.clear();
    w.bible.selected_verse_indices.clear();
    w.bible.search.clear();
    clear_search_input(w, window, cx);

    if let (Some(tid), Some(book)) = (
        w.bible.selected_translation.clone(),
        w.bible.selected_book.clone(),
    ) {
        match w.bible.service.chapter_verses(&tid, &book, chapter) {
            Ok(vs) => w.bible.verses = vs,
            Err(e) => w.set_error(format!("bible get_chapter_verses: {e}")),
        }
    }
    cx.notify();
}

/// Clear the browser search input after a navigation change reset the query
/// (`set_value` emits no `Change`, so this cannot re-enter `search_changed`).
fn clear_search_input(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    w.bible_search_input.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
}

/// Search within the selected chapter (empty query restores the chapter) or
/// across the whole translation. Called from the search input's subscription,
/// so no redraw is triggered from here beyond the notify.
pub(crate) fn search_changed(w: &mut MainWindow, query: String, cx: &mut Context<MainWindow>) {
    w.bible.search = query.clone();
    w.bible.selected_verse_indices.clear();

    if query.trim().is_empty() {
        if let (Some(tid), Some(book), Some(ch)) = (
            w.bible.selected_translation.clone(),
            w.bible.selected_book.clone(),
            w.bible.selected_chapter,
        ) {
            match w.bible.service.chapter_verses(&tid, &book, ch) {
                Ok(vs) => w.bible.verses = vs,
                Err(e) => w.set_error(format!("bible restore chapter: {e}")),
            }
        } else {
            w.bible.verses.clear();
        }
    } else if let Some(tid) = w.bible.selected_translation.clone() {
        match w.bible.service.search_verses(&tid, &query) {
            Ok(result) => {
                w.bible.verses = result.verses;
                // The operator is told that the list is cut short: a search
                // that quietly drops the rest of the matches tells them a
                // lie about what is there.
                w.bible.search_truncated = result.truncated;
            }
            Err(e) => w.set_error(format!("bible search: {e}")),
        }
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

pub(crate) fn verse_toggled(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
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
    cx.notify();
}

pub(crate) fn select_all(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.bible.selected_verse_indices = (0..w.bible.verses.len()).collect();
    cx.notify();
}

pub(crate) fn clear_selection(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.bible.selected_verse_indices.clear();
    cx.notify();
}

pub(crate) fn verses_per_slide_changed(w: &mut MainWindow, n: usize, cx: &mut Context<MainWindow>) {
    w.bible.verses_per_slide = n.clamp(1, 10);
    cx.notify();
}

// ---------------------------------------------------------------------------
// Send to presentation
// ---------------------------------------------------------------------------

/// Build one presentation from the selected verses: they are chunked
/// `verses_per_slide` at a time, each chunk becomes one grouped text slide
/// (`"Book C:V-V"` group labels, `"{verse}  {text}"` lines joined by blank
/// lines) with a 0.5s fade.
pub(crate) fn send_to_presentation(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if w.bible.selected_verse_indices.is_empty() {
        return;
    }

    let selected: Vec<_> = w
        .bible
        .selected_verse_indices
        .iter()
        .filter_map(|&i| w.bible.verses.get(i))
        .cloned()
        .collect();

    if selected.is_empty() {
        return;
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

    // The presentation and its slides are one thing, so they are written
    // together: a presentation whose slides failed to save is an empty entry
    // in the rail while the editor shows the verses.
    match w
        .services
        .presentations
        .create_with_slides(&pres_name, &slides)
    {
        Ok(new_pres) => {
            w.load_presentations();
            crate::ui::navigation::set_editing(
                w,
                Some(Presentation {
                    timeline: crate::domain::Timeline::default(),
                    id: new_pres.id.clone(),
                    name: new_pres.name,
                    slides,
                    created_at: new_pres.created_at,
                    updated_at: new_pres.updated_at,
                }),
            );
            w.editor.selected_slide_index = Some(0);
            w.shell.current_mode = ViewMode::Edit;
            w.shell.sidebar_tab = SidebarTab::Presentations;
            crate::ui::navigation::load_slide_for_editing(w);
            // The Edit-mode top bar's rename field must show the new
            // presentation, not the previously opened one.
            w.editor.editing_name = pres_name.clone();
            crate::ui::components::sync_text_field(&w.sync_rename_input, &pres_name, window, cx);
        }
        Err(e) => w.set_error(format!("bible send_to_presentation create: {e}")),
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Import / delete translations
// ---------------------------------------------------------------------------

pub(crate) fn import_file(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_title("Import Bible Translation (JSON)")
        .pick_file()
    else {
        return;
    };

    // The import runs off the UI thread: a translation holds tens of
    // thousands of verses, and reading, parsing and storing them on the
    // window's thread leaves the app unresponsive for seconds.
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The import could not be started.".to_string());
        cx.notify();
        return;
    };
    let path = path.to_string_lossy().into_owned();
    let service = w.bible.service.clone();
    std::thread::spawn(move || {
        let done = match std::fs::read_to_string(&path) {
            Err(e) => Done::Failed(format!("bible import read: {e}")),
            Ok(raw) => match serde_json::from_str::<BibleImportFile>(&raw) {
                Err(e) => Done::Failed(format!("bible import parse: {e}")),
                Ok(file) => match service.import_translation(file) {
                    Ok(_) => Done::BibleImported,
                    Err(e) => Done::Failed(format!("bible import save: {e}")),
                },
            },
        };
        let _ = tx.blocking_send(done);
    });
    cx.notify();
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.shell.rail_context_target = None;
    w.bible.translation_to_delete = Some(id);
    w.editor.show_delete_confirmation = true;
    cx.notify();
}

pub(crate) fn confirm_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(id) = w.bible.translation_to_delete.take() {
        if let Err(e) = w.bible.service.delete_translation(&id) {
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
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

pub(crate) fn cancel_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.bible.translation_to_delete = None;
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Views: rail translation list
// ---------------------------------------------------------------------------

/// Render the bible translation list for the left rail sidebar: translation
/// rows (abbr + name), the JSON import entry, and Delete Translation when one
/// is selected.
pub fn list_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let translations = w.bible.translations.clone();
    let selected_id = w.bible.selected_translation.clone();

    let mut list = div().flex().flex_col().gap(px(2.)).py(px(4.)).px(px(6.));
    if translations.is_empty() {
        list = list.child(
            div()
                .py(px(20.))
                .px(px(14.))
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No translations.\nClick Import to add one."),
        );
    }
    for tr in &translations {
        let is_active = selected_id.as_deref() == Some(tr.id.as_str());
        list = list.child(translation_row(tr, is_active, cx));
    }

    let import_btn = div()
        .id("bible-import")
        .w_full()
        .py(px(8.))
        .px(px(12.))
        .rounded(px(4.))
        .text_size(px(12.))
        .text_color(gpui_kit::white())
        .bg(theme::c(theme::ACCENT_ORANGE))
        .hover(|s| s.bg(theme::c(theme::ACCENT_ORANGE_HOVER)))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(RootMessage::Bible(Message::ImportFile), window, cx);
        }))
        .child("Import JSON…");

    let mut footer = div().flex().flex_col().gap(px(4.)).child(import_btn);
    if let Some(id) = selected_id {
        footer = footer.child(
            div()
                .id("bible-delete-translation")
                .w_full()
                .py(px(8.))
                .px(px(12.))
                .rounded(px(4.))
                .text_size(px(12.))
                .text_color(theme::c(theme::DANGER_RED))
                .border_1()
                .border_color(theme::c(theme::DANGER_RED).opacity(0.6))
                .hover(|s| s.bg(theme::c(theme::DANGER_RED).opacity(0.14)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Bible(Message::DeleteTranslationClicked(id.clone())),
                        window,
                        cx,
                    );
                }))
                .child("Delete Translation"),
        );
    }

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .id("bible-translations-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(list),
        )
        .child(components::divider())
        .child(footer)
}

fn translation_row(
    tr: &BibleTranslation,
    is_active: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let id = tr.id.clone();
    let abbr = tr.abbr.clone();
    let name = tr.name.clone();

    div()
        .id(SharedString::from(format!("rail-translation-{}", tr.id)))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .py(px(6.))
        .px(px(10.))
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
            w.handle_message(
                RootMessage::Bible(Message::TranslationSelected(id.clone())),
                window,
                cx,
            );
        }))
        .child(
            div()
                .w(px(50.))
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(truncate(&abbr, 8)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(truncate(&name, 26)),
        )
}

// ---------------------------------------------------------------------------
// Views: verse browser (center)
// ---------------------------------------------------------------------------

/// Render the scripture verse browser for the center workspace.
pub fn browser_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let books = w.bible.books.clone();
    if books.is_empty() {
        return empty_browser();
    }

    let selected_book = w.bible.selected_book.clone();
    let mut book_row = div().flex().flex_row().gap(px(4.)).py(px(4.)).px(px(8.));
    for book in &books {
        let name = book.clone();
        let is_sel = selected_book.as_deref() == Some(book.as_str());
        book_row = book_row.child(pill_btn(
            SharedString::from(format!("bible-book-{book}")),
            book,
            is_sel,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Bible(Message::BookSelected(name.clone())),
                    window,
                    cx,
                );
            }),
        ));
    }

    let chapters = w.bible.chapters.clone();
    let selected_chapter = w.bible.selected_chapter;
    let chapter_row: Div = if chapters.is_empty() {
        div()
    } else {
        let mut row = div().flex().flex_row().gap(px(4.)).py(px(4.)).px(px(8.));
        for &ch in &chapters {
            let is_sel = selected_chapter == Some(ch);
            row = row.child(pill_btn(
                SharedString::from(format!("bible-chapter-{ch}")),
                &ch.to_string(),
                is_sel,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(RootMessage::Bible(Message::ChapterSelected(ch)), window, cx);
                }),
            ));
        }
        row
    };

    let verses = w.bible.verses.clone();
    let selected_indices = w.bible.selected_verse_indices.clone();
    let search_active = !w.bible.search.trim().is_empty();

    let mut verse_col = div().flex().flex_col().gap(px(2.)).py(px(2.)).px(px(6.));
    if verses.is_empty() {
        let hint = if search_active {
            "No verses match your search."
        } else {
            "Select a book and chapter to browse verses."
        };
        verse_col = verse_col.child(
            div()
                .py(px(16.))
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(hint),
        );
    } else {
        verse_col = verse_col
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .py(px(2.))
                    .px(px(4.))
                    .child(small_btn(
                        "bible-select-all",
                        "All",
                        cx.listener(|w, _, window, cx| {
                            w.handle_message(RootMessage::Bible(Message::SelectAll), window, cx);
                        }),
                    ))
                    .child(small_btn(
                        "bible-select-none",
                        "None",
                        cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                RootMessage::Bible(Message::ClearSelection),
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child(if w.bible.search_truncated {
                                format!(
                                    "showing the first {} of more matches",
                                    crate::db::repository::BIBLE_SEARCH_LIMIT
                                )
                            } else {
                                format!("{} verses", verses.len())
                            }),
                    ),
            )
            .child(components::divider());

        for (idx, v) in verses.iter().enumerate() {
            let is_checked = selected_indices.contains(&idx);
            verse_col = verse_col.child(verse_row(idx, v, is_checked, search_active, cx));
        }
    }

    // Action bar: selection readout, verses-per-slide stepper, send button.
    let sel_count = selected_indices.len();
    let selected_label = if sel_count == 0 {
        String::from("No verses selected")
    } else {
        format!(
            "{sel_count} verse{} selected",
            if sel_count > 1 { "s" } else { "" }
        )
    };
    let vps = w.bible.verses_per_slide;

    let action_bar = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .py(px(8.))
        .px(px(12.))
        .bg(theme::c(theme::BG_DARK))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(selected_label),
        )
        .child(div().flex_1())
        .child(stepper_btn(
            "bible-vps-dec",
            "−",
            cx.listener(|w, _, window, cx| {
                let next = w.bible.verses_per_slide.saturating_sub(1).max(1);
                w.handle_message(
                    RootMessage::Bible(Message::VersesPerSlideChanged(next)),
                    window,
                    cx,
                );
            }),
        ))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(format!("Verses/slide: {vps}")),
        )
        .child(stepper_btn(
            "bible-vps-inc",
            "+",
            cx.listener(|w, _, window, cx| {
                let next = (w.bible.verses_per_slide + 1).min(10);
                w.handle_message(
                    RootMessage::Bible(Message::VersesPerSlideChanged(next)),
                    window,
                    cx,
                );
            }),
        ))
        .child(
            div()
                .id("bible-send")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .py(px(8.))
                .px(px(14.))
                .rounded(px(4.))
                .text_size(px(13.))
                .text_color(gpui_kit::white())
                .bg(theme::c(theme::ACCENT_ORANGE))
                .hover(|s| s.bg(theme::c(theme::ACCENT_ORANGE_HOVER)))
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Bible(Message::SendToPresentation), window, cx);
                }))
                .child("Send to Presentation"),
        );

    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARKEST))
        .child(
            div()
                .bg(theme::c(theme::BG_DARK))
                .border_b_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                // One horizontal strip of book pills (the iced era used a
                // horizontal scrollable here).
                .child(
                    div()
                        .id("bible-books")
                        .w_full()
                        .overflow_x_scroll()
                        .child(book_row),
                ),
        )
        .when(!chapters.is_empty(), |col| {
            col.child(
                div()
                    .bg(theme::c(theme::BG_DARK))
                    .border_b_1()
                    .border_color(theme::c(theme::BORDER_PANEL))
                    .child(
                        div()
                            .id("bible-chapters")
                            .w_full()
                            .overflow_x_scroll()
                            .child(chapter_row),
                    ),
            )
        })
        .child(
            div()
                .py(px(6.))
                .px(px(8.))
                .border_b_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .child(Input::new(&w.bible_search_input.clone())),
        )
        .child(
            div()
                .id("bible-verses-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(verse_col),
        )
        .child(action_bar)
}

/// Empty state shown before a translation with books is selected.
fn empty_browser() -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::c(theme::BG_DARKEST))
        .child(
            div()
                .text_size(px(14.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Select a translation on the left,\nor import one from a JSON file."),
        )
}

/// One verse row: checkbox indicator + reference + text; clicking the row
/// toggles the selection (the iced era used a `checkbox` with the same
/// toggle semantics).
fn verse_row(
    idx: usize,
    v: &crate::domain::BibleVerse,
    checked: bool,
    search_active: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let label = if search_active {
        format!("{} {}:{}", v.book, v.chapter, v.verse)
    } else {
        format!("v{}", v.verse)
    };
    let text = v.text.clone();

    div()
        .id(SharedString::from(format!("bible-verse-{idx}")))
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(8.))
        .py(px(3.))
        .px(px(4.))
        .rounded(px(3.))
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(RootMessage::Bible(Message::VerseToggled(idx)), window, cx);
        }))
        .child(
            // Checkbox indicator (visual only; the row click toggles).
            div()
                .mt(px(2.))
                .w(px(13.))
                .h(px(13.))
                .flex_shrink_0()
                .rounded(px(3.))
                .border_1()
                .border_color(if checked {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BORDER_STRONG)
                })
                .bg(if checked {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BG_DARK)
                })
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.))
                .text_color(gpui_kit::white())
                .when(checked, |box_el| box_el.child("✓")),
        )
        .child(
            div()
                .w(px(60.))
                .flex_shrink_0()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(text),
        )
}

// ---------------------------------------------------------------------------
// Small shared builders
// ---------------------------------------------------------------------------

/// Small pill button used for book/chapter navigation.
fn pill_btn(
    id: SharedString,
    label: &str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex_shrink_0()
        .py(px(4.))
        .px(px(8.))
        .rounded(px(3.))
        .text_size(px(11.))
        .text_color(if active {
            gpui_kit::white()
        } else {
            theme::c(theme::TEXT_SECONDARY)
        })
        .bg(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            gpui_kit::transparent_black()
        })
        .border_1()
        .border_color(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(on_click)
        .child(label.to_string())
}

/// Compact ghost button (All / None selection shortcuts).
fn small_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .py(px(3.))
        .px(px(8.))
        .rounded(px(3.))
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| {
            s.bg(theme::c(theme::BG_HOVER))
                .text_color(theme::c(theme::TEXT_PRIMARY))
        })
        .on_click(on_click)
        .child(label)
}

/// Round stepper button for the verses-per-slide control.
fn stepper_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .justify_center()
        .py(px(2.))
        .px(px(8.))
        .rounded(px(3.))
        .text_size(px(13.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| {
            s.bg(theme::c(theme::BG_HOVER))
                .text_color(theme::c(theme::TEXT_PRIMARY))
        })
        .on_click(on_click)
        .child(label)
}
