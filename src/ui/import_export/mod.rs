use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::Task;

/// Messages owned by the Import/Export feature module (see `AGENTS.md`).
///
/// `ToggleImportExportPanel` stays as a root variant (global panel visibility
/// toggle, consistent with the other `*Panel` toggles).
///
/// NOTE: the import/export panel UI is not yet implemented; only the action
/// messages and their handlers exist.
#[derive(Debug, Clone)]
pub enum Message {
    ExportOpp,
    ImportOpp,
    ImportOppChosen(String),
    ExportOpenLyrics,
    ImportOpenLyrics,
    ImportOpenLyricsChosen(String),
}

/// Dispatch an import/export message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::ExportOpp => export_opp(w),
        Message::ImportOpp => import_opp_pick(w),
        Message::ImportOppChosen(path) => import_opp_load(w, path),
        Message::ExportOpenLyrics => export_openlyrics(w),
        Message::ImportOpenLyrics => import_openlyrics_pick(w),
        Message::ImportOpenLyricsChosen(path) => import_openlyrics_load(w, path),
    }
}

pub(crate) fn export_opp(w: &mut MainWindow) -> Task<RootMessage> {
    let pres = match w.editor.editing.clone() {
        Some(p) => p,
        None => return Task::none(),
    };

    let dest = rfd::FileDialog::new()
        .set_title("Export Presentation Package")
        .add_filter("OpenPresenter Package", &["opp"])
        .set_file_name(format!("{}.opp", pres.name))
        .save_file();

    if let Some(path) = dest {
        match crate::import::opp::export(&pres, &path) {
            Ok(n) => println!("[export .opp] wrote {n} media files to {}", path.display()),
            Err(e) => w.set_error(format!("[export .opp] error: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn import_opp_pick(w: &mut MainWindow) -> Task<RootMessage> {
    let path = rfd::FileDialog::new()
        .set_title("Import Presentation Package")
        .add_filter("OpenPresenter Package", &["opp"])
        .pick_file();

    match path {
        Some(p) => import_opp_load(w, p.to_string_lossy().into_owned()),
        None => Task::none(),
    }
}

pub(crate) fn import_opp_load(w: &mut MainWindow, path: String) -> Task<RootMessage> {
    let media_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openpresenter")
        .join("media");

    match crate::import::opp::import(std::path::Path::new(&path), &media_dir) {
        Ok(pres) => match w.services.presentations.create(&pres.name) {
            Ok(created) => {
                if let Err(e) = w
                    .services
                    .presentations
                    .replace_slides(&created.id, &pres.slides)
                {
                    w.set_error(format!("[import .opp] slide save error: {e}"));
                }
                w.load_presentations();
            }
            Err(e) => w.set_error(format!("[import .opp] DB create error: {e}")),
        },
        Err(e) => w.set_error(format!("[import .opp] error: {e}")),
    }
    Task::none()
}

pub(crate) fn import_openlyrics_pick(w: &mut MainWindow) -> Task<RootMessage> {
    let path = rfd::FileDialog::new()
        .set_title("Import OpenLyrics Song")
        .add_filter("OpenLyrics XML", &["xml"])
        .pick_file();

    match path {
        Some(p) => import_openlyrics_load(w, p.to_string_lossy().into_owned()),
        None => Task::none(),
    }
}

pub(crate) fn import_openlyrics_load(w: &mut MainWindow, path: String) -> Task<RootMessage> {
    match crate::import::openlyrics::import_song(std::path::Path::new(&path)) {
        Ok(song) => {
            if let Err(e) = w.song.repo.save_song(&song) {
                w.set_error(format!("[import OpenLyrics] DB save error: {e}"));
            } else {
                w.load_songs();
            }
        }
        Err(e) => w.set_error(format!("[import OpenLyrics] error: {e}")),
    }
    Task::none()
}

pub(crate) fn export_openlyrics(w: &mut MainWindow) -> Task<RootMessage> {
    let song = match w.song.editing.clone() {
        Some(s) => s,
        None => return Task::none(),
    };

    let dest = rfd::FileDialog::new()
        .set_title("Export Song as OpenLyrics")
        .add_filter("OpenLyrics XML", &["xml"])
        .set_file_name(format!("{}.xml", song.title))
        .save_file();

    if let Some(path) = dest
        && let Err(e) = crate::import::openlyrics::export_song(&song, &path)
    {
        w.set_error(format!("[export OpenLyrics] error: {e}"));
    }
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.import_export.panel_open = !w.import_export.panel_open;
    Task::none()
}
