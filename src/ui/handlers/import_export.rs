use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::Task;

pub fn export_opp(w: &mut MainWindow) -> Task<Message> {
    let pres = match w.editing_presentation.clone() {
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
            Ok(n) => eprintln!("[export .opp] wrote {n} media files to {}", path.display()),
            Err(e) => eprintln!("[export .opp] error: {e}"),
        }
    }
    Task::none()
}

pub fn import_opp_pick(w: &mut MainWindow) -> Task<Message> {
    let path = rfd::FileDialog::new()
        .set_title("Import Presentation Package")
        .add_filter("OpenPresenter Package", &["opp"])
        .pick_file();

    match path {
        Some(p) => import_opp_load(w, p.to_string_lossy().into_owned()),
        None => Task::none(),
    }
}

pub fn import_opp_load(w: &mut MainWindow, path: String) -> Task<Message> {
    let media_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openpresenter")
        .join("media");

    match crate::import::opp::import(std::path::Path::new(&path), &media_dir) {
        Ok(pres) => match w.repo.create_presentation(&pres.name) {
            Ok(created) => {
                if let Err(e) = w
                    .repo
                    .replace_presentation_slides(&created.id, &pres.slides)
                {
                    eprintln!("[import .opp] slide save error: {e}");
                }
                w.load_presentations();
            }
            Err(e) => eprintln!("[import .opp] DB create error: {e}"),
        },
        Err(e) => eprintln!("[import .opp] error: {e}"),
    }
    Task::none()
}

pub fn import_openlyrics_pick(w: &mut MainWindow) -> Task<Message> {
    let path = rfd::FileDialog::new()
        .set_title("Import OpenLyrics Song")
        .add_filter("OpenLyrics XML", &["xml"])
        .pick_file();

    match path {
        Some(p) => import_openlyrics_load(w, p.to_string_lossy().into_owned()),
        None => Task::none(),
    }
}

pub fn import_openlyrics_load(w: &mut MainWindow, path: String) -> Task<Message> {
    match crate::import::openlyrics::import_song(std::path::Path::new(&path)) {
        Ok(song) => {
            if let Err(e) = w.song.repo.save_song(&song) {
                eprintln!("[import OpenLyrics] DB save error: {e}");
            } else {
                w.load_songs();
            }
        }
        Err(e) => eprintln!("[import OpenLyrics] error: {e}"),
    }
    Task::none()
}

pub fn export_openlyrics(w: &mut MainWindow) -> Task<Message> {
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
        eprintln!("[export OpenLyrics] error: {e}");
    }
    Task::none()
}

pub fn toggle_panel(w: &mut MainWindow) -> Task<Message> {
    w.import_export_panel_open = !w.import_export_panel_open;
    Task::none()
}
