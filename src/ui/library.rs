//! Media library panel: asset grid (recently used + all), import via a
//! native file picker, delete, selection, and Apply-to-Current-Slide.
//!
//! Ported from the iced-era `ui/library/mod.rs` (the rail-width panel). The
//! panel renders as the Library tab's detail pane; the global rail search
//! doubles as the asset-name filter (the iced era bound both to the same
//! `SearchQueryChanged` root message).

use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, IntoElement, ParentElement, SharedString, Styled, Window, div, px};

use crate::domain::{ImageFit, LibraryAsset};
use crate::ui::components::{self, truncate};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SidebarTab};
use crate::ui::theme;

/// Mirror library search state into the shared search input.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let query = w.shell.search_query.clone();
    crate::ui::components::sync_text_field(&w.search_input, &query, window, cx);
}

/// Messages owned by the Library feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Library(..)`.
///
/// The asset search stays a root variant (`Message::SearchQueryChanged`),
/// shared with the rail's global search field.
#[derive(Debug, Clone)]
pub enum Message {
    ImportAsset,
    ApplyToSlide(String),
    /// Ask to delete an asset.
    DeleteAsset(String),
    /// Delete the asset the confirmation names.
    ConfirmDelete,
    /// Leave the asset alone.
    CancelDelete,
    SelectAsset(String),
}

/// Dispatch a library message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::ImportAsset => import_asset(w, cx),
        Message::ApplyToSlide(id) => apply_to_slide(w, id, cx),
        Message::DeleteAsset(id) => ask_delete_asset(w, id, cx),
        Message::ConfirmDelete => delete_asset(w, cx),
        Message::CancelDelete => cancel_delete_asset(w, cx),
        Message::SelectAsset(id) => select_asset(w, id, cx),
    }
}

/// Pick an image/video with a native dialog and register it in the library.
pub(crate) fn import_asset(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let image_exts = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];
    let video_exts = ["mp4", "mov", "avi", "mkv", "webm"];
    if let Some(path) = rfd::FileDialog::new()
        .add_filter(
            "Images & Videos",
            &[
                "png", "jpg", "jpeg", "gif", "bmp", "webp", "mp4", "mov", "avi", "mkv", "webm",
            ],
        )
        .set_title("Import Asset")
        .pick_file()
    {
        let path_str = path.to_string_lossy().into_owned();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let media_type = if image_exts.contains(&ext.as_str()) {
            "image"
        } else if video_exts.contains(&ext.as_str()) {
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
}

/// Put the asset onto the slide the operator is looking at.
///
/// In the editor that is the slide being edited; during a show it is the
/// slide on the projector, so a picture applied from the library lands on
/// what the audience is looking at.
pub(crate) fn apply_to_slide(w: &mut MainWindow, asset_id: String, cx: &mut Context<MainWindow>) {
    let asset = w.library.assets.iter().find(|a| a.id == asset_id).cloned();
    let Some(asset) = asset else {
        w.set_error("That asset is no longer in the library.".to_string());
        return;
    };
    // The file is gone: putting it on a slide would add an object that
    // draws nothing, and the operator would find out on the projector.
    if !std::path::Path::new(&asset.path).exists() {
        w.set_error(format!(
            "The file of '{}' is no longer on the disk.",
            asset.name
        ));
        return;
    }
    // The asset becomes an object on the slide, so the renderer draws it
    // like any other object.
    let object = if asset.is_image() {
        crate::domain::Object::new_image(asset.path.clone(), ImageFit::default())
    } else {
        crate::domain::Object::new_video(asset.path.clone(), None)
    };

    let showing = w.shell.current_mode == crate::ui::messages::ViewMode::Show;
    let (pres_id, index) = if showing {
        let Some(pres) = w.presenting.presentation.as_ref() else {
            w.set_error("No slide is selected.".to_string());
            return;
        };
        (pres.id.clone(), w.presenting.slide_index)
    } else {
        let Some(pres) = w.editor.editing.as_ref() else {
            w.set_error("No slide is selected.".to_string());
            return;
        };
        let Some(index) = w.editor.selected_slide_index else {
            w.set_error("No slide is selected.".to_string());
            return;
        };
        (pres.id.clone(), index)
    };

    // The slide keeps the text it had: a slide that still carries its title
    // in the legacy content has to be converted into objects first, or the
    // title stops being drawn the moment the picture is added.
    let updated = {
        let target = if showing {
            w.presenting.presentation.as_mut()
        } else {
            w.editor.editing.as_mut()
        };
        let Some(slide) = target.and_then(|p| p.slides.get_mut(index)) else {
            w.set_error("No slide is selected.".to_string());
            return;
        };
        slide.migrate_to_objects();
        // The picture goes on top: the slide is drawn back to front.
        slide.add_object_on_top(object);
        Some(slide.clone())
    };
    let Some(slide) = updated else {
        return;
    };

    // The slide is saved through the service, so the change survives a
    // restart.
    if let Err(e) = w.services.presentations.update_slide(&pres_id, &slide) {
        w.set_error(format!("Failed to save slide: {e}"));
    }
    // The use is recorded in the library itself, so the recently-used
    // strip is still there after a restart.
    if let Err(e) = w.library.repo.touch_asset(&asset_id) {
        log::warn!("could not record the use of the asset: {e}");
    }
    cx.notify();
}

/// Remove the asset from the library and every bookkeeping list.
/// Ask before an asset is removed from the library.
///
/// Every other destructive action in the app asks first: a library entry is
/// the operator's own file list, and a click that removes it without asking
/// is a click they cannot take back.
pub(crate) fn ask_delete_asset(w: &mut MainWindow, asset_id: String, cx: &mut Context<MainWindow>) {
    w.library.to_delete = Some(asset_id);
    cx.notify();
}

/// Leave the asset alone.
pub(crate) fn cancel_delete_asset(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.library.to_delete = None;
    cx.notify();
}

/// Remove the asset the confirmation names.
pub(crate) fn delete_asset(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(asset_id) = w.library.to_delete.take() else {
        return;
    };
    // The rail's context menu closes: its backdrop covers the window, so a
    // menu that stayed up would swallow the operator's next click.
    w.shell.rail_context_target = None;
    match w.library.repo.delete_asset(&asset_id) {
        Ok(_) => {
            w.library.assets.retain(|a| a.id != asset_id);
            if w.library.selected_id.as_deref() == Some(&asset_id) {
                w.library.selected_id = None;
            }
        }
        Err(e) => w.set_error(format!("library delete: {e}")),
    }
    cx.notify();
}

/// Toggle the selected asset (clicking the selection again clears it).
pub(crate) fn select_asset(w: &mut MainWindow, asset_id: String, cx: &mut Context<MainWindow>) {
    w.library.selected_id = if w.library.selected_id.as_deref() == Some(&asset_id) {
        None
    } else {
        Some(asset_id)
    };
    cx.notify();
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

/// Render the library panel for the left rail: recently-used grid, all-assets
/// grid, the Apply-to-Current-Slide action bar, and the import entry.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let assets = w.library.assets.clone();
    let selected_id = w.library.selected_id.clone();
    let query_lower = w.shell.search_query.to_lowercase();

    // The recently used assets come from the library, so the strip survives a
    // restart. They were read when the library was loaded: a query on every
    // render is a lag the operator feels.
    let recent_ids: Vec<String> = w.library.recent_ids.clone();
    let recent_assets: Vec<&LibraryAsset> = recent_ids
        .iter()
        .filter_map(|id| assets.iter().find(|a| &a.id == id))
        .collect();

    let all_filtered: Vec<&LibraryAsset> = assets
        .iter()
        .filter(|a| query_lower.is_empty() || a.name.to_lowercase().contains(&query_lower))
        .collect();

    let mut scroll_col = div().flex().flex_col().gap(px(6.)).py(px(4.)).px(px(8.));

    if !recent_assets.is_empty() {
        scroll_col = scroll_col
            .child(section_divider("RECENTLY USED"))
            .child(asset_grid(
                &recent_assets,
                selected_id.as_deref(),
                &w.library.missing,
                cx,
            ));
    }

    if all_filtered.is_empty() && assets.is_empty() {
        scroll_col = scroll_col.child(
            div()
                .py(px(16.))
                .px(px(8.))
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No assets imported yet.\nClick \"Import\" to add images or videos."),
        );
    } else {
        if !recent_assets.is_empty() {
            scroll_col = scroll_col.child(section_divider("ALL ASSETS"));
        }
        if all_filtered.is_empty() {
            scroll_col = scroll_col.child(
                div()
                    .py(px(10.))
                    .px(px(8.))
                    .text_size(px(12.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("No results"),
            );
        } else {
            scroll_col = scroll_col.child(asset_grid(
                &all_filtered,
                selected_id.as_deref(),
                &w.library.missing,
                cx,
            ));
        }
    }

    let mut body = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(components::section_header("MEDIA LIBRARY"))
        .child(
            div()
                .id("library-assets-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(scroll_col),
        );

    if let Some(selected) = selected_id {
        body = body.child(
            div().p(px(6.)).child(
                div()
                    .id("library-apply")
                    .w_full()
                    .py(px(8.))
                    .px(px(10.))
                    .rounded(px(4.))
                    .text_size(px(12.))
                    .text_color(gpui_kit::white())
                    .bg(theme::c(theme::ACCENT_ORANGE))
                    .hover(|s| s.bg(theme::c(theme::ACCENT_ORANGE_HOVER)))
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Library(Message::ApplyToSlide(selected.clone())),
                            window,
                            cx,
                        );
                    }))
                    .child("Apply to Current Slide"),
            ),
        );
    }

    body.child(components::add_button("library-import", "Import Asset", {
        cx.listener(|w, _, window, cx| {
            w.handle_message(RootMessage::Library(Message::ImportAsset), window, cx);
        })
    }))
}

/// Two-column grid of asset cards.
fn asset_grid(
    assets: &[&LibraryAsset],
    selected_id: Option<&str>,
    missing: &[String],
    cx: &mut Context<MainWindow>,
) -> Div {
    let mut grid = div().flex().flex_col().gap(px(4.));
    let mut row_buf: Vec<gpui_kit::AnyElement> = Vec::new();

    for (i, asset) in assets.iter().enumerate() {
        let selected = selected_id == Some(asset.id.as_str());
        row_buf
            .push(asset_card(asset, selected, missing.contains(&asset.id), cx).into_any_element());

        if row_buf.len() == 2 {
            grid = flush_asset_row(grid, &mut row_buf, false);
        } else if i == assets.len() - 1 {
            grid = flush_asset_row(grid, &mut row_buf, true);
        }
    }

    grid
}

/// Drain the pending card buffer into one grid row (`pad` adds a filler so a
/// trailing odd card keeps its column width).
fn flush_asset_row(grid: Div, row_buf: &mut Vec<gpui_kit::AnyElement>, pad: bool) -> Div {
    if row_buf.is_empty() {
        return grid;
    }
    let mut row = div().flex().flex_row().gap(px(4.)).w_full();
    for card in row_buf.drain(..) {
        row = row.child(card);
    }
    if pad {
        row = row.child(div().flex_1());
    }
    grid.child(row)
}

/// One asset card: preview tile (type badge over black), badge/delete row,
/// and the truncated name. Click selects, right-click opens the rail context
/// menu, the × deletes.
fn asset_card(
    asset: &LibraryAsset,
    selected: bool,
    missing: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let id_select = asset.id.clone();
    let id_menu = asset.id.clone();
    let id_delete = asset.id.clone();
    let type_badge_text = if asset.is_image() { "IMG" } else { "VID" };
    let type_color = if asset.is_image() {
        theme::ACCENT_ORANGE
    } else {
        theme::WARNING_AMBER
    };
    // A file that is no longer on the disk would draw nothing, so the card
    // says so rather than looking like a normal asset.
    let glyph = if missing {
        "⚠"
    } else if asset.is_image() {
        "🖼"
    } else {
        "🎬"
    };

    let tile = div()
        .w_full()
        .h(px(52.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(gpui_kit::black())
        .border_1()
        .border_color(if missing {
            theme::c(theme::DANGER_RED)
        } else if selected {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .child(div().text_size(px(22.)).child(glyph));

    let badge_row = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .child(
            div()
                .py(px(1.))
                .px(px(4.))
                .rounded(px(2.))
                .text_size(px(8.))
                .text_color(theme::c(type_color))
                .bg(theme::c(type_color).opacity(0.15))
                .border_1()
                .border_color(theme::c(type_color))
                .child(type_badge_text),
        )
        .when(missing, |row| {
            // The file is gone: the asset cannot be put on a slide.
            row.child(
                div()
                    .py(px(1.))
                    .px(px(4.))
                    .rounded(px(2.))
                    .text_size(px(8.))
                    .text_color(theme::c(theme::DANGER_RED))
                    .bg(theme::c(theme::DANGER_RED).opacity(0.15))
                    .border_1()
                    .border_color(theme::c(theme::DANGER_RED))
                    .child("GONE"),
            )
        })
        .child(div().flex_1())
        .child(
            div()
                .id(SharedString::from(format!("library-del-{}", asset.id)))
                .px(px(4.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .hover(|s| {
                    s.bg(theme::c(theme::BG_HOVER))
                        .text_color(theme::c(theme::DANGER_RED))
                })
                .on_click(cx.listener(move |w, _, window, cx| {
                    // The × is nested inside the clickable card; stop the
                    // event so deleting does not also toggle the selection.
                    cx.stop_propagation();
                    w.handle_message(
                        RootMessage::Library(Message::DeleteAsset(id_delete.clone())),
                        window,
                        cx,
                    );
                }))
                .child("✕"),
        );

    div()
        .id(SharedString::from(format!("library-asset-{}", asset.id)))
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .p(px(4.))
        .rounded(px(5.))
        .bg(if selected {
            theme::c(theme::ACCENT_ORANGE).opacity(0.15)
        } else {
            gpui_kit::transparent_black()
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::Library(Message::SelectAsset(id_select.clone())),
                window,
                cx,
            );
        }))
        .on_mouse_down(
            gpui_kit::MouseButton::Right,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::ShowRailContextMenu(
                        crate::ui::messages::RailContextTarget::LibraryAsset(id_menu.clone()),
                    ),
                    window,
                    cx,
                );
            }),
        )
        .child(tile)
        .child(badge_row)
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(truncate(&asset.name, 14)),
        )
}

/// Hairline label divider between grid sections ("RECENTLY USED" / "ALL
/// ASSETS").
fn section_divider(label: &'static str) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(4.))
        .child(div().flex_1().h(px(1.)).bg(theme::c(theme::BORDER_PANEL)))
        .child(
            div()
                .text_size(px(9.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(label),
        )
        .child(div().flex_1().h(px(1.)).bg(theme::c(theme::BORDER_PANEL)))
}
