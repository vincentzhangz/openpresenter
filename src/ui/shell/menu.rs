//! The commands behind the menu bar that reach into the app's features.
//!
//! The menu itself is a map of the app (see `ui/menus.rs`); the commands it
//! names are implemented here, next to the features they belong to, so a
//! command is never a menu item that does nothing.

use gpui_kit::{Context, Window};

use crate::domain::{Background, Slide};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message, SidebarTab};

/// Pick every object on the slide.
pub(crate) fn select_all(w: &mut MainWindow, _window: &mut Window, cx: &mut Context<MainWindow>) {
    crate::ui::layers::select_all_layers(w, cx);
}

/// Leave every object alone.
pub(crate) fn deselect_all(w: &mut MainWindow, _window: &mut Window, cx: &mut Context<MainWindow>) {
    crate::ui::layers::deselect_all_layers(w, cx);
}

/// Cut the selected object to the clipboard.
///
/// Cutting is copying and then removing, and the removal is undoable: a cut
/// that could not be undone would lose a slide the operator built.
pub(crate) fn cut_object(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if w.layer.selected_index.is_none() {
        return;
    }
    w.handle_message(Message::CopyObject, window, cx);
    w.handle_message(Message::DeleteObject, window, cx);
}

/// Put the library's rail on the presentation being edited.
///
/// "Reveal in Library" is how the operator finds the presentation again
/// after the show has moved on: the rail opens on the library and the
/// presentation is the one that is picked.
pub(crate) fn reveal_in_library(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let Some(presentation) = w.editor.editing.as_ref() else {
        return;
    };
    let name = presentation.name.clone();
    w.handle_message(
        Message::SelectLeftSection(SidebarTab::Presentations),
        window,
        cx,
    );
    // The search box is the rail's way of finding one presentation among
    // many, so the name goes into it: the operator sees where they landed.
    w.shell.search_query = name;
    cx.notify();
}

/// Move the slide's background to the next picture in the library.
///
/// The backgrounds are the library's pictures, in the order they were
/// imported: an operator who has a set of backgrounds knows which one comes
/// next.
pub(crate) fn next_background(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    step_background(w, 1, window, cx);
}

/// Move the slide's background to the previous picture in the library.
pub(crate) fn previous_background(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    step_background(w, -1, window, cx);
}

/// Move the slide's background one step through the library's pictures.
fn step_background(
    w: &mut MainWindow,
    step: i32,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let pictures: Vec<String> = w
        .library
        .assets
        .iter()
        .filter(|asset| asset.media_type == "image")
        .map(|asset| asset.path.clone())
        .collect();
    if pictures.is_empty() {
        return;
    }
    let current = match w.get_current_slide().map(|slide| &slide.background) {
        Some(Background::Image(path)) => path.clone(),
        _ => String::new(),
    };
    let position = pictures
        .iter()
        .position(|path| *path == current)
        .map(|index| index as i32)
        .unwrap_or(-1);
    let count = pictures.len() as i32;
    // The backgrounds are a ring: the one after the last is the first, and
    // the one before the first is the last.
    let next = ((position + step).rem_euclid(count)) as usize;
    let background = Background::Image(pictures[next].clone());
    w.push_undo();
    w.handle_message(
        Message::Slides(crate::ui::slides::Message::SlideBackgroundChanged(
            background,
        )),
        window,
        cx,
    );
}

/// Write the edited presentation's slides out as text.
///
/// The text is what the operator reads in the notes and on the slides, so
/// it is what goes into the file: a slide's group and its notes, in the
/// order the slides are shown.
pub(crate) fn export_text(w: &mut MainWindow, _window: &mut Window, cx: &mut Context<MainWindow>) {
    let Some(presentation) = w.editor.editing.as_ref() else {
        return;
    };
    let Some(dest) = rfd::FileDialog::new()
        .set_title("Export Text")
        .set_file_name(format!("{}.txt", presentation.name))
        .add_filter("Text", &["txt"])
        .save_file()
    else {
        return;
    };
    let mut out = String::new();
    out.push_str(&presentation.name);
    out.push('\n');
    out.push('\n');
    for (index, slide) in presentation.slides.iter().enumerate() {
        out.push_str(&format!("--- Slide {} ---\n", index + 1));
        if let Some(group) = &slide.group {
            out.push_str(&format!("Group: {group}\n"));
        }
        if let Some((text, _)) = slide.editable_text(None) {
            out.push_str(text);
            out.push('\n');
        }
        if let Some(notes) = &slide.notes
            && !notes.trim().is_empty()
        {
            out.push_str("Notes: ");
            out.push_str(notes);
            out.push('\n');
        }
        out.push('\n');
    }
    if let Err(e) = std::fs::write(&dest, out) {
        w.set_error(format!("Export text: {e}"));
    }
    cx.notify();
}

/// Write the app's themes out.
///
/// The themes are the app's own, so they are written in the app's own
/// format: a file another tool cannot read is a file nobody can use, but a
/// file this app cannot read back is worse.
pub(crate) fn export_themes(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let themes = w.theme_state.repo.list_themes().unwrap_or_default();
    let Some(dest) = rfd::FileDialog::new()
        .set_title("Export Themes")
        .set_file_name("openpresenter-themes.json")
        .add_filter("Themes", &["json"])
        .save_file()
    else {
        return;
    };
    match serde_json::to_string_pretty(&themes) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&dest, json) {
                w.set_error(format!("Export themes: {e}"));
            }
        }
        Err(e) => w.set_error(format!("Export themes: {e}")),
    }
    cx.notify();
}

/// Write the playlist the operator is working on out.
pub(crate) fn export_playlist(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let playlists = w.services.playlists.list().unwrap_or_default();
    let Some(dest) = rfd::FileDialog::new()
        .set_title("Export Playlist")
        .set_file_name("openpresenter-playlist.json")
        .add_filter("Playlist", &["json"])
        .save_file()
    else {
        return;
    };
    match serde_json::to_string_pretty(&playlists) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&dest, json) {
                w.set_error(format!("Export playlist: {e}"));
            }
        }
        Err(e) => w.set_error(format!("Export playlist: {e}")),
    }
    cx.notify();
}

/// Write every slide of the edited presentation out as a picture.
///
/// The pictures are drawn with the same pipeline the projector uses, so a
/// slide that is exported is a slide that is seen.
pub(crate) fn export_slide_images(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let Some(presentation) = w.editor.editing.as_ref() else {
        return;
    };
    let Some(dest) = rfd::FileDialog::new()
        .set_title("Export Slide Images")
        .set_file_name(&presentation.name)
        .pick_folder()
    else {
        return;
    };
    let slides: Vec<Slide> = presentation.slides.clone();
    let name = presentation.name.clone();
    let width = w.app_config.output.width.max(320);
    let height = w.app_config.output.height.max(180);
    let mut pipeline = crate::render::RenderPipeline::new_software(width, height);
    for (index, slide) in slides.iter().enumerate() {
        let frame = match pipeline.render_slide(slide) {
            Ok(frame) => frame,
            Err(e) => {
                w.set_error(format!("Export slide images: {e}"));
                return;
            }
        };
        let path = dest.join(format!("{}-{:02}.png", name, index + 1));
        if let Err(e) = write_frame_as_png(&frame, &path) {
            w.set_error(format!("Export slide images: {e}"));
            return;
        }
    }
    cx.notify();
}

/// Write a rendered frame out as a PNG.
fn write_frame_as_png(frame: &crate::render::Frame, path: &std::path::Path) -> anyhow::Result<()> {
    let mut buffer = image::RgbImage::new(frame.width, frame.height);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        // The frame is BGRA and the PNG is RGB.
        let index = ((y * frame.width + x) * 4) as usize;
        *pixel = image::Rgb([
            frame.data[index + 2],
            frame.data[index + 1],
            frame.data[index],
        ]);
    }
    buffer.save(path)?;
    Ok(())
}

/// Bring a media file into the library.
pub(crate) fn import_media(w: &mut MainWindow, _window: &mut Window, cx: &mut Context<MainWindow>) {
    let video_exts = ["mp4", "mov", "avi", "mkv", "webm"];
    let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Images & Videos",
            &[
                "png", "jpg", "jpeg", "gif", "bmp", "webp", "mp4", "mov", "avi", "mkv", "webm",
            ],
        )
        .set_title("Import Media")
        .pick_file()
    else {
        return;
    };
    let path_str = path.to_string_lossy().into_owned();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    // A file the app does not know is treated as a picture: the library
    // shows it as one, and the operator can replace it.
    let media_type = if video_exts.contains(&ext.as_str()) {
        "video"
    } else {
        "image"
    };
    match w.library.repo.add_asset(&path_str, media_type) {
        Ok(_) => w.load_lib_assets(),
        Err(e) => w.set_error(format!("library import: {e}")),
    }
    w.shell.sidebar_tab = SidebarTab::Library;
    cx.notify();
}

/// Bring the text on the clipboard in as a presentation.
///
/// The text is split on its blank lines: a blank line ends a slide, so a
/// block of copied text becomes the slides it reads as.
pub(crate) fn import_text_from_clipboard(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    use std::borrow::BorrowMut;
    let text = {
        let app: &mut gpui_kit::App = cx.borrow_mut();
        app.read_from_clipboard().and_then(|item| {
            item.entries.into_iter().find_map(|entry| match entry {
                gpui_kit::ClipboardEntry::String(string) => Some(string.text),
                _ => None,
            })
        })
    };
    let Some(text) = text else {
        w.set_error(String::from("The clipboard holds no text."));
        cx.notify();
        return;
    };
    let blocks: Vec<String> = text
        .split("\n\n")
        .map(|block| block.trim().to_string())
        .filter(|block| !block.is_empty())
        .collect();
    if blocks.is_empty() {
        w.set_error(String::from("The clipboard holds no text."));
        cx.notify();
        return;
    }

    let slides: Vec<crate::domain::Slide> = blocks
        .into_iter()
        .map(|block| {
            let mut slide = crate::domain::Slide::new_text(block);
            slide.group = Some(String::from("Verse"));
            slide
        })
        .collect();
    match w
        .services
        .presentations
        .create_with_slides("Clipboard", &slides)
    {
        Ok(saved) => {
            w.editor.presentations.push(saved);
            w.shell.sidebar_tab = SidebarTab::Presentations;
            cx.notify();
        }
        Err(e) => w.set_error(format!("Import text: {e}")),
    }
}

/// How much of the slide the editor shows.
///
/// The canvas fills its workspace with the slide, and the zoom multiplies
/// that: the operator looks at one corner of a slide without the slide's
/// own size changing.
pub(crate) fn zoom(
    w: &mut MainWindow,
    command: crate::ui::messages::ZoomCommand,
    cx: &mut Context<MainWindow>,
) {
    use crate::ui::messages::ZoomCommand as Z;
    match command {
        Z::ActualSize => {
            // The slide at the resolution it was authored against: one
            // pixel of the slide is one pixel of the screen.
            w.canvas.actual_size = true;
            w.canvas.zoom = 1.0;
        }
        Z::FitCanvas | Z::FitSlide => {
            w.canvas.actual_size = false;
            w.canvas.zoom = 1.0;
        }
        Z::In => {
            w.canvas.actual_size = false;
            w.canvas.zoom = (w.canvas.zoom * 1.25).clamp(0.1, 8.0);
        }
        Z::Out => {
            w.canvas.actual_size = false;
            w.canvas.zoom = (w.canvas.zoom / 1.25).clamp(0.1, 8.0);
        }
    }
    cx.notify();
}
