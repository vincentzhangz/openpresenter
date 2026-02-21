use crate::slides::{BibleImportFile, Presentation, Slide, Transition};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, SidebarTab, ViewMode};
use iced::Task;
use uuid::Uuid;

pub(crate) fn translation_selected(w: &mut MainWindow, id: String) -> Task<Message> {
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
        Err(e) => eprintln!("bible list_books: {e}"),
    }
    Task::none()
}

pub(crate) fn book_selected(w: &mut MainWindow, book: String) -> Task<Message> {
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
            Err(e) => eprintln!("bible list_chapters: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn chapter_selected(w: &mut MainWindow, chapter: i32) -> Task<Message> {
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
            Err(e) => eprintln!("bible get_chapter_verses: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn search_changed(w: &mut MainWindow, query: String) -> Task<Message> {
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
                Err(e) => eprintln!("bible restore chapter: {e}"),
            }
        } else {
            w.bible.verses.clear();
        }
    } else if let Some(tid) = w.bible.selected_translation.clone() {
        match w.bible.repo.search_verses(&tid, &query) {
            Ok(vs) => w.bible.verses = vs,
            Err(e) => eprintln!("bible search: {e}"),
        }
    }
    Task::none()
}

pub(crate) fn verse_toggled(w: &mut MainWindow, idx: usize) -> Task<Message> {
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

pub(crate) fn select_all(w: &mut MainWindow) -> Task<Message> {
    w.bible.selected_verse_indices = (0..w.bible.verses.len()).collect();
    Task::none()
}

pub(crate) fn clear_selection(w: &mut MainWindow) -> Task<Message> {
    w.bible.selected_verse_indices.clear();
    Task::none()
}

pub(crate) fn verses_per_slide_changed(w: &mut MainWindow, n: usize) -> Task<Message> {
    w.bible.verses_per_slide = n.max(1).min(10);
    Task::none()
}

pub(crate) fn send_to_presentation(w: &mut MainWindow) -> Task<Message> {
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

    match w.repo.create_presentation(&pres_name) {
        Ok(_) => {
            w.load_presentations();
            if let Some(new_pres) = w.presentations.first().cloned() {
                if let Err(e) = w
                    .repo
                    .replace_presentation_slides(&new_pres.id, &pres.slides)
                {
                    eprintln!("bible send_to_presentation replace: {e}");
                }
                w.editing_presentation = Some(Presentation {
                    id: new_pres.id.clone(),
                    name: new_pres.name,
                    slides: pres.slides,
                    created_at: new_pres.created_at,
                    updated_at: new_pres.updated_at,
                });
                w.selected_slide_index = Some(0);
                w.current_mode = ViewMode::Edit;
                w.sidebar_tab = SidebarTab::Presentations;
                w.load_slide_for_editing();
            }
        }
        Err(e) => eprintln!("bible send_to_presentation create: {e}"),
    }
    Task::none()
}

pub(crate) fn import_file(w: &mut MainWindow) -> Task<Message> {
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
            eprintln!("bible import read: {e}");
            return Task::none();
        }
    };

    let file: BibleImportFile = match serde_json::from_str(&raw) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bible import parse: {e}");
            return Task::none();
        }
    };

    let abbr = file.abbr.clone();
    match w.bible.repo.import_translation(file) {
        Ok(tr) => {
            eprintln!("Imported Bible translation: {} ({})", tr.name, abbr);
            w.load_bible_translations();
        }
        Err(e) => eprintln!("bible import save: {e}"),
    }
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<Message> {
    w.bible.translation_to_delete = Some(id);
    Task::none()
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.bible.translation_to_delete.take() {
        if let Err(e) = w.bible.repo.delete_translation(&id) {
            eprintln!("bible delete: {e}");
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

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<Message> {
    w.bible.translation_to_delete = None;
    Task::none()
}
