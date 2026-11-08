//! Import/export: `.opp` presentation packages and OpenLyrics songs.
//!
//! Ported from the iced-era `ui/import_export/mod.rs`. Import/export is
//! reached from the presentations rail (Import/Export `.opp` buttons) and the
//! songs panel (Import/Export OpenLyrics); there is no dedicated panel.

use gpui_kit::{Context, Window};

use crate::domain::Presentation;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{SidebarTab, ViewMode};

#[derive(Debug, Clone)]
pub enum Message {
    /// Export the presentation currently being edited as an `.opp` package.
    ExportOpp,
    /// Open the native pick dialog for an `.opp` package and import it.
    ImportOpp,
    /// Import the chosen `.opp` package (also used by the pick flow itself).
    ImportOppChosen(String),
    /// Export the song being edited as OpenLyrics XML.
    ExportOpenLyrics,
    /// Import a PowerPoint presentation.
    ImportPowerPoint,
    /// Open the native pick dialog for an OpenLyrics file and import it.
    ImportOpenLyrics,
    /// Import the chosen OpenLyrics file (also used by the pick flow itself).
    ImportOpenLyricsChosen(String),
}

/// The ways a PowerPoint file comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPointImport {
    /// The slides come in editable: the text is text and the pictures are
    /// pictures.
    AsObjectsAndText,
    /// The slides come in as they were: the picture is the slide, drawn
    /// with the app's own renderer.
    AsImages,
    /// The whole file comes in as a presentation: the slides, the
    /// transitions they carry and the notes under them.
    AsPresentation,
}

/// The outcome of a package export or import, finished on a worker thread.
#[derive(Debug, Clone)]
pub enum Done {
    /// The package was written.
    Exported {
        /// The media files that travelled with the package.
        embedded: usize,
        /// The media the package refers to but could not include.
        missing: Vec<String>,
    },
    /// The package was read; its presentation is ready to be stored.
    Imported(Presentation),
    /// A song was read; it is ready to be stored.
    SongImported(crate::domain::Song),
    /// A Bible translation was imported.
    BibleImported,
    /// The work failed.
    Failed(String),
}

/// Dispatch an import/export message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::ExportOpp => export_opp(w),
        Message::ImportOpp => import_opp_pick(w),
        Message::ImportPowerPoint => import_powerpoint(w, PowerPointImport::AsObjectsAndText, cx),
        Message::ImportOppChosen(path) => import_opp_load(w, &path),
        Message::ExportOpenLyrics => export_openlyrics(w, cx),
        Message::ImportOpenLyrics => import_openlyrics_pick(w, cx),
        Message::ImportOpenLyricsChosen(path) => import_openlyrics_load(w, &path, cx),
    }
}

/// Handle the result of a package export or import.
pub(crate) fn done(
    w: &mut MainWindow,
    result: Done,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match result {
        Done::Exported { embedded, missing } => {
            if missing.is_empty() {
                log::info!("[export .opp] the package carries {embedded} media file(s)");
            } else {
                // A package that refers to media it does not carry plays
                // nothing on the venue's machine, so the operator has to hear
                // about it.
                let names = missing
                    .iter()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.clone())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                w.set_error(format!(
                    "The package was written, but {} media file(s) are missing: {names}",
                    missing.len()
                ));
            }
        }
        Done::Imported(pres) => {
            // The presentation and its slides are written together, so an
            // import either lands whole or does not land at all.
            match w
                .services
                .presentations
                .create_with_slides(&pres.name, &pres.slides)
            {
                Ok(created) => {
                    w.load_presentations();
                    // The imported presentation is opened, so the operator
                    // sees what they just imported instead of the
                    // presentation that was open before. It opens on its
                    // first slide: an empty canvas would look like the
                    // import failed.
                    w.shell.current_mode = ViewMode::Edit;
                    w.shell.sidebar_tab = SidebarTab::Presentations;
                    crate::ui::navigation::open_for_editing(w, created, Some(0));
                }
                Err(e) => w.set_error(format!("[import .opp] DB save error: {e}")),
            }
        }
        Done::SongImported(song) => {
            // The song and its verses are one thing, so they are written
            // together: a song whose lyrics were left behind is a song the
            // operator has to fill in again.
            match w.song.service.save(&song) {
                Ok(()) => {
                    w.load_songs();
                    // The imported song is opened, so the operator sees what
                    // they just imported instead of the song that was open
                    // before.
                    crate::ui::songs::load_song_for_editing(w, song, window, cx);
                }
                Err(e) => w.set_error(format!("[import OpenLyrics] DB save error: {e}")),
            }
        }
        Done::BibleImported => {
            w.load_bible_translations();
        }
        Done::Failed(e) => w.set_error(e),
    }
    cx.notify();
}

/// Export the presentation being edited (no-op without an edited one).
fn export_opp(w: &mut MainWindow) {
    let Some(pres) = w.editor.editing.clone() else {
        return;
    };
    export_package(w, &pres);
}

/// Export the presentation as a PDF.
///
/// The slides are drawn with the same pipeline the projector uses, so the
/// PDF is the show the audience saw — the same pages the operator would
/// print.
pub(crate) fn export_pdf(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(pres) = w.editor.editing.clone() else {
        return;
    };
    let Some(dest) = rfd::FileDialog::new()
        .set_title("Export Presentation as PDF")
        .add_filter("PDF", &["pdf"])
        .set_file_name(format!("{}.pdf", pres.name))
        .save_file()
    else {
        return;
    };

    // The slides are drawn off the UI thread: a presentation is a lot of
    // pixels, and drawing them on the window's thread would freeze the app
    // while the operator waits.
    let width = w.app_config.output.width;
    let height = w.app_config.output.height;
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The export could not be started.".to_string());
        return;
    };
    std::thread::spawn(move || {
        let result = crate::pdf::render_presentation(&pres, width, height, &dest);
        let done = match result {
            Ok(()) => Done::Exported {
                embedded: pres.slides.len(),
                missing: Vec::new(),
            },
            Err(e) => Done::Failed(format!("[export .pdf] error: {e}")),
        };
        let _ = tx.blocking_send(done);
    });
    cx.notify();
}

/// Print the presentation.
///
/// The slides are written out as a PDF and handed to the system's PDF
/// viewer, which carries the print panel: the operator prints the order,
/// the lyrics or the slides, and the document is the same one the
/// projector was showing.
pub(crate) fn print_presentation(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(pres) = w.editor.editing.clone() else {
        return;
    };
    let width = w.app_config.output.width;
    let height = w.app_config.output.height;

    // The document goes into the system's temporary folder: a print is a
    // document the operator takes away, not a file they have to tidy up.
    let dir = std::env::temp_dir().join("openpresenter-print");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        w.set_error(format!("Print: {e}"));
        cx.notify();
        return;
    }
    let dest = dir.join(format!("{}.pdf", pres.name.replace('/', "-")));

    std::thread::spawn(move || {
        let result = crate::pdf::render_presentation(&pres, width, height, &dest);
        match result {
            Ok(()) => {
                // The system opens the document, and the operator prints it
                // from there.
                let _ = std::process::Command::new("open").arg(&dest).spawn();
            }
            Err(e) => log::error!("print: {e}"),
        }
    });
    cx.notify();
}

/// Export the presentation with the given id as an `.opp` package, asking for
/// the destination with a native save dialog. Used by the rail context menu.
pub(crate) fn export_presentation_by_id(
    w: &mut MainWindow,
    id: &str,
    cx: &mut Context<MainWindow>,
) {
    let pres = match w.services.presentations.get(id) {
        Ok(p) => p,
        Err(e) => {
            w.set_error(format!("Failed to load presentation for export: {e}"));
            cx.notify();
            return;
        }
    };
    export_package(w, &pres);
}

/// Export the presentation with the given id (rail context menu, no redraw
/// context). Thin wrapper over [`export_presentation_by_id`] without a
/// redraw notification.
pub(crate) fn export_presentation(w: &mut MainWindow, id: &str) {
    let pres = match w.services.presentations.get(id) {
        Ok(p) => p,
        Err(e) => {
            w.set_error(format!("Failed to load presentation for export: {e}"));
            return;
        }
    };
    export_package(w, &pres);
}

/// Handle a [`Done`] without a `Window` (worker-thread completion bridge).
///
/// The completion arrives on a thread without a window, so the song-import
/// path cannot sync the metadata inputs: the strings are set and the inputs
/// catch up on the next render via `sync_inputs`.
pub(crate) fn done_now(w: &mut MainWindow, result: Done, cx: &mut Context<MainWindow>) {
    match result {
        Done::Exported { embedded, missing } => {
            if missing.is_empty() {
                log::info!("[export .opp] the package carries {embedded} media file(s)");
            } else {
                let names = missing
                    .iter()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| p.clone())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                w.set_error(format!(
                    "The package was written, but {} media file(s) are missing: {names}",
                    missing.len()
                ));
            }
        }
        Done::Imported(pres) => match w
            .services
            .presentations
            .create_with_slides(&pres.name, &pres.slides)
        {
            Ok(created) => {
                w.load_presentations();
                w.shell.current_mode = ViewMode::Edit;
                w.shell.sidebar_tab = SidebarTab::Presentations;
                crate::ui::navigation::open_for_editing(w, created, Some(0));
            }
            Err(e) => w.set_error(format!("[import .opp] DB save error: {e}")),
        },
        Done::SongImported(song) => match w.song.service.save(&song) {
            Ok(()) => {
                w.load_songs();
                w.song.edit_title = song.title.clone();
                w.song.edit_artist = song.artist.clone().unwrap_or_default();
                w.song.edit_copyright = song.copyright.clone().unwrap_or_default();
                w.song.edit_ccli = song.ccli_number.clone().unwrap_or_default();
                w.song.edit_key = song.key_signature.clone().unwrap_or_default();
                w.song.edit_bpm = song.bpm.map(|v| v.to_string()).unwrap_or_default();
                w.song.editing = Some(song);
            }
            Err(e) => w.set_error(format!("[import OpenLyrics] DB save error: {e}")),
        },
        Done::BibleImported => {
            w.load_bible_translations();
        }
        Done::Failed(e) => w.set_error(e),
    }
    cx.notify();
}

/// Shared `.opp` save-dialog + write flow for both export entry points.
fn export_package(w: &mut MainWindow, pres: &crate::domain::Presentation) {
    let dest = rfd::FileDialog::new()
        .set_title("Export Presentation Package")
        .add_filter("OpenPresenter Package", &["opp"])
        .set_file_name(format!("{}.opp", pres.name))
        .save_file();

    let Some(path) = dest else {
        return;
    };
    export_to(w, pres, path);
}

/// Write the package, off the UI thread.
///
/// A show with a video in it is a show with a lot of bytes in it, and
/// deflating them on the UI thread freezes the window for as long as it
/// takes.
fn export_to(w: &mut MainWindow, pres: &crate::domain::Presentation, path: std::path::PathBuf) {
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The export could not be started.".to_string());
        return;
    };
    let pres = pres.clone();
    std::thread::spawn(move || {
        let result = crate::import::opp::export(&pres, &path);
        let done = match result {
            Ok(report) => Done::Exported {
                embedded: report.embedded,
                missing: report.missing,
            },
            Err(e) => Done::Failed(format!("Export failed: {e}")),
        };
        let _ = tx.blocking_send(done);
    });
}

/// The directory the imported media is unpacked into.
///
/// It sits next to the library the app already uses, so the package's media
/// and the operator's own files live in one place.
pub(crate) fn media_dir(w: &MainWindow) -> std::path::PathBuf {
    w.app_config
        .db_path
        .parent()
        .map(|dir| dir.join("media"))
        .unwrap_or_else(|| std::path::PathBuf::from("media"))
}

/// Open the native pick dialog for an `.opp` package and import it.
fn import_opp_pick(w: &mut MainWindow) {
    let path = rfd::FileDialog::new()
        .set_title("Import Presentation Package")
        .add_filter("OpenPresenter Package", &["opp"])
        .pick_file();

    if let Some(p) = path {
        import_opp_load(w, &p.to_string_lossy());
    }
}

/// Import an `.opp` package: unpack it into the media dir and read its
/// presentation, off the UI thread.
///
/// The package is stored in one transaction when it comes back, so a
/// presentation whose slides failed to save cannot leave an empty entry
/// behind.
fn import_opp_load(w: &mut MainWindow, path: &str) {
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The import could not be started.".to_string());
        return;
    };
    let media_dir = media_dir(w);
    let path = path.to_string();
    std::thread::spawn(move || {
        let done = match crate::import::opp::import(std::path::Path::new(&path), &media_dir) {
            Ok(pres) => Done::Imported(pres),
            Err(e) => Done::Failed(format!("[import .opp] error: {e}")),
        };
        let _ = tx.blocking_send(done);
    });
}

/// Import a PowerPoint presentation.
///
/// The file is read off the UI thread: a presentation is a big file, and
/// reading it on the UI thread would freeze the app while the operator
/// waits.
pub(crate) fn import_powerpoint(
    w: &mut MainWindow,
    mode: PowerPointImport,
    _cx: &mut Context<MainWindow>,
) {
    let title = match mode {
        PowerPointImport::AsImages => "Import PowerPoint as Images",
        PowerPointImport::AsObjectsAndText => "Import PowerPoint as Objects and Text",
        PowerPointImport::AsPresentation => "Import PowerPoint as Presentation",
    };
    let Some(path) = rfd::FileDialog::new()
        .set_title(title)
        .add_filter("PowerPoint", &["pptx"])
        .pick_file()
    else {
        return;
    };
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The import could not be started.".to_string());
        return;
    };
    let media_dir = media_dir(w);
    let path = path.to_string_lossy().into_owned();
    let width = w.app_config.output.width;
    let height = w.app_config.output.height;
    std::thread::spawn(move || {
        // The deck that is played carries the show's furniture — the
        // transition and the notes — while the deck that is edited comes in
        // as objects the operator is going to change.
        let with_show_details = mode != PowerPointImport::AsObjectsAndText;
        let done = match crate::import::pptx::import_with(
            std::path::Path::new(&path),
            &media_dir,
            with_show_details,
        ) {
            Ok(pptx) => {
                // The presentation is stored with its slides in one go, so
                // a failed slide write cannot leave an empty entry behind.
                let mut presentation = Presentation::new(pptx.name);
                presentation.slides = pptx.slides;
                // The ways in: the objects and the text can be
                // edited afterwards, and the picture is the slide drawn by
                // the app's own renderer.
                let presentation = if mode == PowerPointImport::AsImages {
                    match crate::import::pptx::import_as_images(
                        &presentation,
                        &media_dir,
                        width,
                        height,
                    ) {
                        Ok(imported) => imported,
                        Err(e) => {
                            let _ = tx.blocking_send(Done::Failed(format!(
                                "[import .pptx as images] error: {e}"
                            )));
                            return;
                        }
                    }
                } else {
                    presentation
                };
                Done::Imported(presentation)
            }
            Err(e) => Done::Failed(format!("[import .pptx] error: {e}")),
        };
        let _ = tx.blocking_send(done);
    });
}

/// Open the native pick dialog for an OpenLyrics file and import it.
fn import_openlyrics_pick(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let path = rfd::FileDialog::new()
        .set_title("Import OpenLyrics Song")
        .add_filter("OpenLyrics XML", &["xml"])
        .pick_file();

    if let Some(p) = path {
        import_openlyrics_load(w, &p.to_string_lossy(), cx);
    }
}

/// Import an OpenLyrics song into the song library and refresh the list.
///
/// The file is read and parsed off the UI thread: a lyrics file is a few
/// hundred kilobytes of XML, and parsing it on the window's thread is a
/// stall the operator feels.
fn import_openlyrics_load(w: &mut MainWindow, path: &str, _cx: &mut Context<MainWindow>) {
    let Some(tx) = w.import_export_tx.clone() else {
        w.set_error("The import could not be started.".to_string());
        return;
    };
    let path = path.to_string();
    std::thread::spawn(move || {
        let done = match crate::import::openlyrics::import_song(std::path::Path::new(&path)) {
            Ok(song) => Done::SongImported(song),
            Err(e) => Done::Failed(format!("[import OpenLyrics] error: {e}")),
        };
        let _ = tx.blocking_send(done);
    });
}

/// Export the song being edited as OpenLyrics XML (no-op without an edited
/// song).
fn export_openlyrics(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(song) = w.song.editing.clone() else {
        return;
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
    cx.notify();
}
