use crate::slides::{Presentation, Slide, Transition, Verse};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Size, Task, window};
use uuid::Uuid;

pub(crate) fn search_changed(w: &mut MainWindow, query: String) -> Task<Message> {
    w.song.search = query.clone();
    match w.song.repo.search_songs(&query) {
        Ok(songs) => w.song.songs = songs,
        Err(e) => eprintln!("song search error: {e}"),
    }
    Task::none()
}

pub(crate) fn new_song(w: &mut MainWindow) -> Task<Message> {
    let song = crate::slides::Song::new("New Song".to_string());
    w.load_song_for_editing(song);
    Task::none()
}

pub(crate) fn open_song(w: &mut MainWindow, id: String) -> Task<Message> {
    match w.song.repo.get_song(&id) {
        Ok(song) => w.load_song_for_editing(song),
        Err(e) => eprintln!("open song error: {e}"),
    }
    Task::none()
}

pub(crate) fn save_song(w: &mut MainWindow) -> Task<Message> {
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
        eprintln!("save song error: {e}");
        return Task::none();
    }
    if let Err(e) = w.song.repo.replace_verses(&song.id, &song.verses) {
        eprintln!("save verses error: {e}");
        return Task::none();
    }

    w.song.editing = Some(song);
    w.load_songs();
    Task::none()
}

pub(crate) fn delete_clicked(w: &mut MainWindow, id: String) -> Task<Message> {
    if let Some(existing) = w.delete_confirm_window_id {
        return window::gain_focus(existing);
    }
    w.song.to_delete = Some(id);
    let (win_id, open_task) = window::open(window::Settings {
        size: Size::new(420.0, 260.0),
        resizable: false,
        ..Default::default()
    });
    w.delete_confirm_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

pub(crate) fn confirm_delete(w: &mut MainWindow) -> Task<Message> {
    if let Some(id) = w.song.to_delete.take() {
        if let Err(e) = w.song.repo.delete_song(&id) {
            eprintln!("delete song error: {e}");
        }
        if w.song.editing.as_ref().is_some_and(|s| s.id == id) {
            w.song.editing = None;
        }
        w.load_songs();
    }
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn cancel_delete(w: &mut MainWindow) -> Task<Message> {
    w.song.to_delete = None;
    if let Some(win_id) = w.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}

pub(crate) fn title_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_title = v;
    Task::none()
}
pub(crate) fn artist_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_artist = v;
    Task::none()
}
pub(crate) fn copyright_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_copyright = v;
    Task::none()
}
pub(crate) fn ccli_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_ccli = v;
    Task::none()
}
pub(crate) fn key_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_key = v;
    Task::none()
}
pub(crate) fn bpm_changed(w: &mut MainWindow, v: String) -> Task<Message> {
    w.song.edit_bpm = v;
    Task::none()
}

pub(crate) fn add_verse(w: &mut MainWindow) -> Task<Message> {
    let Some(song) = w.song.editing.as_mut() else {
        return Task::none();
    };
    let idx = song.verses.len();
    let label = next_verse_label(idx);
    song.verses.push(Verse::new(label, String::new(), idx));
    Task::none()
}

pub(crate) fn delete_verse(w: &mut MainWindow, index: usize) -> Task<Message> {
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

pub(crate) fn move_verse_up(w: &mut MainWindow, index: usize) -> Task<Message> {
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

pub(crate) fn move_verse_down(w: &mut MainWindow, index: usize) -> Task<Message> {
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
) -> Task<Message> {
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
) -> Task<Message> {
    if let Some(song) = w.song.editing.as_mut()
        && let Some(v) = song.verses.get_mut(index)
    {
        v.label = label;
    }
    Task::none()
}

pub(crate) fn song_to_presentation(w: &mut MainWindow) -> Task<Message> {
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

    match w.repo.create_presentation(&pres_name) {
        Ok(_) => {
            w.load_presentations();
            if let Some(new_pres) = w.presentations.first().cloned() {
                if let Err(e) = w
                    .repo
                    .replace_presentation_slides(&new_pres.id, &pres.slides)
                {
                    eprintln!("song_to_presentation replace slides: {e}");
                }
                w.editing_presentation = Some(Presentation {
                    id: new_pres.id,
                    name: new_pres.name,
                    slides: pres.slides,
                    created_at: new_pres.created_at,
                    updated_at: new_pres.updated_at,
                });
                w.selected_slide_index = if w
                    .editing_presentation
                    .as_ref()
                    .map_or(0, |p| p.slides.len())
                    > 0
                {
                    Some(0)
                } else {
                    None
                };
                w.current_mode = crate::ui::messages::ViewMode::Edit;
                w.sidebar_tab = crate::ui::messages::SidebarTab::Presentations;
                w.load_slide_for_editing();
            }
        }
        Err(e) => eprintln!("song_to_presentation create: {e}"),
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
