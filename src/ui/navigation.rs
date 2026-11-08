//! Presenting core and shell navigation logic.
//!
//! Ported from the iced-era `ui/presenter/mod.rs` (slide activation,
//! transitions, cue queueing), `ui/navigation/mod.rs` (mode switches, search,
//! presentation lifecycle) and `ui/polish/mod.rs` (undo/redo). The root
//! `Message` vocabulary is unchanged; `MainWindow::handle_message` routes the
//! shell/presenting arms here.

use std::time::Duration;

use gpui_kit::{Context, Window};

use crate::domain::{Action, Cue, Slide, Transition};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, SidebarTab, SlideViewMode, ViewMode};
use crate::ui::stage;

/// The wall-clock duration of a [`Transition`], in milliseconds.
pub(crate) fn transition_duration_ms(transition: &Transition) -> u64 {
    match transition {
        Transition::Cut => 0,
        Transition::Fade { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => *duration_ms,
    }
}

// ---------------------------------------------------------------------------
// Mode / shell navigation
// ---------------------------------------------------------------------------

/// Switch between Show and Edit mode, promoting the edited presentation to the
/// live one (Show) or loading the live one for editing (Edit).
pub(crate) fn switch_mode(w: &mut MainWindow, mode: ViewMode, cx: &mut Context<MainWindow>) {
    switch_mode_core(w, mode);
    cx.notify();
}

/// The document the show plays.
///
/// While the operator is editing, the show plays the document they are
/// editing: a live copy that was left behind would put the slide from
/// before the edit on the projector — and going back to the editor would
/// then load that stale copy and lose the edit.
fn live_document(
    edited: Option<&crate::domain::Presentation>,
    live: Option<&crate::domain::Presentation>,
) -> Option<crate::domain::Presentation> {
    edited.cloned().or_else(|| live.cloned())
}

/// Core of [`switch_mode`], shared with helpers that notify once at the end.
fn switch_mode_core(w: &mut MainWindow, mode: ViewMode) {
    let was_edit = w.shell.current_mode == ViewMode::Edit;
    w.shell.current_mode = mode;
    if mode == ViewMode::Show {
        w.presenting.presentation = live_document(
            w.editor.editing.as_ref(),
            w.presenting.presentation.as_ref(),
        );
        set_editing(w, None);
        w.editor.selected_slide_index = None;
        // Follow the slide the presenter is showing: start its video (or stop
        // the player when the new slide has none).
        let idx = w.presenting.slide_index;
        crate::ui::video::on_presenter_slide_changed(w, idx);
    } else {
        // Edit mode is the slide editor, so the rail goes back to the
        // presentations: the content editors take over the centre, and the
        // operator needs a way back to the slide they were working on.
        w.shell.sidebar_tab = SidebarTab::Presentations;
    }
    if mode == ViewMode::Edit && !was_edit {
        // The stage display is the show's speaker view: the editor needs the
        // canvas, so the swap is turned off with it. Without this the canvas
        // would never come back.
        w.presenting.stage_display_active = false;
        // Entering Edit from Show: load the live presentation for editing.
        // The show is over, so the video must not keep playing in the
        // background — its decoder thread and its sound would carry on.
        crate::ui::video::stop_video(w);
        if let Some(ref pres) = w.presenting.presentation {
            set_editing(w, Some(pres.clone()));
            w.editor.selected_slide_index = Some(w.presenting.slide_index);
            load_slide_for_editing(w);
        } else if let Some(first) = w.editor.presentations.first().cloned() {
            set_editing(w, Some(first));
            w.editor.selected_slide_index = Some(0);
            load_slide_for_editing(w);
        }
    }
}

/// Show the inspector's `tab`, lazily loading themes for the Theme tab.
pub(crate) fn switch_inspector_tab(
    w: &mut MainWindow,
    tab: InspectorTab,
    cx: &mut Context<MainWindow>,
) {
    w.shell.inspector_tab = tab;
    if tab == InspectorTab::Theme && w.theme_state.list.is_empty() {
        w.load_themes();
    }
    cx.notify();
}

fn ensure_editing_presentation(w: &mut MainWindow) {
    if w.shell.current_mode != ViewMode::Edit {
        switch_mode_core(w, ViewMode::Edit);
    }
    if w.editor.editing.is_none() {
        if let Some(ref pres) = w.presenting.presentation {
            set_editing(w, Some(pres.clone()));
        } else if let Some(first) = w.editor.presentations.first().cloned() {
            set_editing(w, Some(first));
        }
    }
    if let Some(ref pres) = w.editor.editing
        && w.editor.selected_slide_index.is_none()
        && !pres.slides.is_empty()
    {
        let idx = w
            .presenting
            .slide_index
            .min(pres.slides.len().saturating_sub(1));
        w.editor.selected_slide_index = Some(idx);
        load_slide_for_editing(w);
    }
}

/// Open the Theme inspector tab for the active (or first) presentation.
pub(crate) fn open_theme(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    ensure_editing_presentation(w);
    w.shell.inspector_tab = InspectorTab::Theme;
    if w.theme_state.list.is_empty() {
        w.load_themes();
    }
    cx.notify();
}

/// Open the Text inspector tab for the active (or first) presentation.
pub(crate) fn open_text_editor(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    ensure_editing_presentation(w);
    w.shell.inspector_tab = InspectorTab::Text;
    cx.notify();
}

/// Close the live presentation and return to the library list.
pub(crate) fn back_to_list(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.editor.editing = None;
    w.presenting.presentation = None;
    w.editor.selected_slide_index = None;
    w.presenting.slide_index = 0;
    w.editor.editing_slide_text.clear();
    w.editor.editing_slide_font_size = String::from("72");
    // The show is over, so the cues that are still waiting must not fire.
    w.presenting.pending_cues.clear();
    crate::ui::video::stop_video(w);
    cx.notify();
}

/// Select a left-rail section, lazily loading library assets for Library.
pub(crate) fn switch_sidebar_tab(
    w: &mut MainWindow,
    tab: SidebarTab,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    w.shell.sidebar_tab = tab;
    // The search replaces the section's panel while it is running, so
    // choosing a section leaves the search: a tab that lights up while the
    // panel it names never appears is a dead control.
    if !w.shell.search_query.trim().is_empty() {
        w.shell.search_query.clear();
        w.search_input.update(cx, |state, input_cx| {
            state.set_value("", window, input_cx);
        });
    }
    if tab == SidebarTab::Library && w.library.assets.is_empty() {
        w.load_lib_assets();
    }
    cx.notify();
}

/// Mirror the global-search input's value into the shell state. The
/// [`gpui_kit::component::input::InputState`] entity remains the source of
/// truth; the subscription forwards changes here.
pub(crate) fn search_query_changed(w: &mut MainWindow, q: String, cx: &mut Context<MainWindow>) {
    w.shell.search_query = q;
    cx.notify();
}

/// Focus the global search field so the user can type immediately.
pub(crate) fn focus_search(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    w.shell.sidebar_tab = SidebarTab::Presentations;
    w.search_input
        .update(cx, |input, input_cx| input.focus(window, input_cx));
    cx.notify();
}

/// Jump to a slide picked from a global-search result row: open the owning
/// presentation, select the slide, and clear the query so the rail returns to
/// its normal contents.
pub(crate) fn open_search_result_slide(
    w: &mut MainWindow,
    pres_id: String,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    // The document is loaded fresh: the search list holds a snapshot from
    // the last reload, and installing that snapshot would put back the text
    // the operator has just changed.
    let pres = match w.services.presentations.get(&pres_id) {
        Ok(pres) => pres,
        Err(e) => {
            w.set_error(format!("Failed to open the presentation: {e}"));
            cx.notify();
            return;
        }
    };

    if w.shell.current_mode != ViewMode::Edit {
        switch_mode_core(w, ViewMode::Edit);
    }
    open_for_editing(w, pres, Some(index));
    w.shell.search_query.clear();
    w.search_input.update(cx, |state, input_cx| {
        // `set_value` does not emit `InputEvent::Change`, so the mirrored
        // shell state above stays the single cleared copy.
        state.set_value("", window, input_cx);
    });
    cx.notify();
}

// ---------------------------------------------------------------------------
// Presentation lifecycle (new / delete / duplicate)
// ---------------------------------------------------------------------------

/// Open the new-presentation modal.
pub(crate) fn new_presentation_clicked(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.editor.new_presentation_open = true;
    // The field starts empty: the last name would otherwise still be there,
    // and "Create" would make a second deck with the same name.
    w.new_presentation_input.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
    // The field takes the focus, so the operator can type the name straight
    // away instead of clicking the field first. The focus is given on the
    // next tick: a field that is not on the screen yet cannot take it.
    let input = w.new_presentation_input.clone();
    let handle = w.window_handle;
    cx.spawn(async move |_this, cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(1))
            .await;
        if let Some(handle) = handle {
            let _ = handle.update(cx, |_root, window, cx| {
                input.update(cx, |state, input_cx| state.focus(window, input_cx));
            });
        }
    })
    .detach();
    cx.notify();
}

/// Create a presentation from the modal's input value and close the modal.
pub(crate) fn create_presentation(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let name = w.new_presentation_input.read(cx).value().to_string();
    if !name.trim().is_empty() {
        match w.services.presentations.create(&name) {
            Ok(pres) => {
                w.load_presentations();
                // A new presentation opens ready to be worked on: a first
                // slide to type on, and the editor in front of it. Creating
                // a presentation and then having to find it in the list to
                // open it, only to find it empty, is two steps too many.
                match w.services.presentations.add_slide(&pres.id) {
                    Ok(updated) => {
                        w.shell.current_mode = ViewMode::Edit;
                        w.shell.sidebar_tab = SidebarTab::Presentations;
                        open_for_editing(w, updated, Some(0));
                    }
                    Err(e) => w.set_error(format!("Failed to add the first slide: {e}")),
                }
            }
            Err(e) => w.set_error(format!("Failed to create presentation: {e}")),
        }
    }
    w.editor.new_presentation_open = false;
    // The field is cleared for the next time the modal is opened.
    w.new_presentation_input.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
    cx.notify();
}

/// Close the new-presentation modal without creating anything.
pub(crate) fn cancel_new_presentation(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.editor.new_presentation_open = false;
    cx.notify();
}

/// Open the delete-confirmation modal for a presentation.
pub(crate) fn delete_presentation_clicked(
    w: &mut MainWindow,
    id: String,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    w.editor.delete_target_id = Some(id);
    w.editor.show_delete_confirmation = true;
    cx.notify();
}

/// Delete the presentation pending confirmation and close the modal.
pub(crate) fn confirm_delete_presentation(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(id) = w.editor.delete_target_id.take() {
        match w.services.presentations.delete(&id) {
            Ok(_) => {
                // Only the document that was deleted is dropped: deleting
                // another presentation from the rail must not close the one
                // the operator is working on.
                let was_editing = w.editor.editing.as_ref().is_some_and(|p| p.id == id);
                let was_playing = w
                    .presenting
                    .presentation
                    .as_ref()
                    .is_some_and(|p| p.id == id);
                if was_editing {
                    w.editor.editing = None;
                }
                if was_playing {
                    w.presenting.presentation = None;
                    // The presentation is gone, so whatever it was playing has
                    // to stop.
                    crate::ui::video::stop_video(w);
                }
                w.load_presentations();
            }
            Err(e) => w.set_error(format!("Failed to delete: {e}")),
        }
    }
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

/// Close the delete-confirmation modal without deleting.
pub(crate) fn cancel_delete(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.editor.delete_target_id = None;
    w.editor.show_delete_confirmation = false;
    cx.notify();
}

/// Duplicate a presentation by id; in Show mode the copy goes live at slide 0.
pub(crate) fn duplicate_presentation(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.shell.rail_context_target = None;
    match w.services.presentations.duplicate_presentation(&id) {
        Ok(pres) => {
            w.load_presentations();
            if w.shell.current_mode == ViewMode::Show {
                w.presenting.presentation = Some(pres);
                w.presenting.slide_index = 0;
                activate_slide(w, 0, false, cx);
            } else {
                open_for_editing(w, pres, Some(0));
            }
        }
        Err(e) => w.set_error(format!("Failed to duplicate presentation: {e}")),
    }
    cx.notify();
}

/// Open a presentation for presenting or editing depending on the mode.
pub(crate) fn open_presentation(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    // The rail leaves the search: the operator asked for a presentation, and
    // the results list is in the way of the one they opened.
    w.shell.search_query.clear();
    w.search_input.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
    match w.services.presentations.get(&id) {
        Ok(pres) => {
            if w.shell.current_mode == ViewMode::Edit {
                // Another presentation is open for editing, so whatever was
                // playing has to stop.
                crate::ui::video::stop_video(w);
                open_for_editing(w, pres, None);
            } else {
                w.presenting.slide_index = 0;
                w.presenting.presentation = Some(pres);
                activate_slide(w, 0, false, cx);
            }
        }
        Err(e) => w.set_error(format!("Failed to load presentation: {e}")),
    }
    cx.notify();
}

/// Open a document for editing, and let the live document follow it.
///
/// The show that follows has to show the presentation the operator has
/// open: a live document that was left behind would put the wrong slides
/// on the projector.
pub(crate) fn open_for_editing(
    w: &mut MainWindow,
    pres: crate::domain::Presentation,
    slide_index: Option<usize>,
) {
    w.editor.editing_name = pres.name.clone();
    w.editor.selected_slide_index = match slide_index {
        Some(index) if index < pres.slides.len() => Some(index),
        _ if !pres.slides.is_empty() => Some(0),
        _ => None,
    };
    set_editing(w, Some(pres.clone()));
    // The live document follows the one that is open, so the next show
    // shows what the operator is working on.
    w.presenting.presentation = Some(pres);
    w.presenting.slide_index = 0;
    load_slide_for_editing(w);
}

/// Present a presentation: the show starts on its first slide.
///
/// The rail's "Present" item is not the same as clicking the row: the row
/// opens the presentation for editing, and this one puts it on the projector.
pub(crate) fn present_presentation(
    w: &mut MainWindow,
    id: String,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    let Ok(pres) = w.services.presentations.get(&id) else {
        w.set_error(format!("Failed to load presentation: {id}"));
        cx.notify();
        return;
    };
    w.presenting.presentation = Some(pres);
    w.presenting.slide_index = 0;
    // The show runs: the editor's copy is put aside, and the cues that were
    // waiting on the last slide must not fire into this one.
    w.presenting.pending_cues.clear();
    switch_mode_core(w, ViewMode::Show);
    activate_slide(w, 0, false, cx);
    cx.notify();
}

/// Open a presentation for editing from the rail context menu.
pub(crate) fn edit_presentation(
    w: &mut MainWindow,
    id: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    w.shell.rail_context_target = None;
    w.shell.current_mode = ViewMode::Edit;
    // The show is over, so the cues that are still waiting must not fire
    // into the document the operator is about to edit.
    w.presenting.pending_cues.clear();
    open_presentation(w, id, window, cx);
}

/// Mirror the rename field's value into `editor.editing_name`; applied by
/// [`rename_presentation`]. The [`gpui_kit::component::input::InputState`]
/// entity stays the source of truth via its subscription.
pub(crate) fn rename_changed(w: &mut MainWindow, name: String, cx: &mut Context<MainWindow>) {
    w.editor.editing_name = name;
    cx.notify();
}

/// Rename the presentation being edited via the inspector name field.
pub(crate) fn rename_presentation(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let name = w.editor.editing_name.clone();
    let Some(id) = w.editor.editing.as_ref().map(|pres| pres.id.clone()) else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    // A rename is an edit like any other, so it can be undone: the snapshot
    // carries the old name.
    w.push_undo();
    match w.services.presentations.rename(&id, &name) {
        Ok(_) => {
            if let Some(p) = &mut w.editor.editing {
                p.name = name;
            }
            w.load_presentations();
        }
        Err(e) => w.set_error(format!("Failed to rename: {e}")),
    }
    cx.notify();
}

/// Open a document for editing, with a fresh history.
///
/// The undo stack belongs to the document that is open: a snapshot of
/// another presentation must never be restored over this one, or the
/// editor would show a document that was never opened and the other
/// document's slides would be rewritten.
pub(crate) fn set_editing(w: &mut MainWindow, pres: Option<crate::domain::Presentation>) {
    w.editor.editing = pres;
    w.editor.undo_stack.clear();
    w.editor.redo_stack.clear();
    w.editor.undo_coalesce.clear();
}

/// Undo the last editing change by restoring a full presentation snapshot.
///
/// Undo belongs to the document being edited. In Show mode there is no
/// document open, and a stray Ctrl+Z would rewrite the *saved* presentation
/// with an old snapshot while the show kept running the current slides —
/// the rollback would only show up the next time the app started.
pub(crate) fn undo(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if w.shell.current_mode != ViewMode::Edit {
        return;
    }
    // The coalesce window is closed: the next edit takes a fresh snapshot,
    // so the redo stack cannot keep an edit that was made after the undo.
    w.editor.undo_coalesce.clear();
    // The snapshot has to belong to the document that is open: a stack that
    // survived from another presentation would replace this one with it.
    let editing_id = w.editor.editing.as_ref().map(|pres| pres.id.clone());
    if let Some(snapshot) = w
        .editor
        .undo_stack
        .pop()
        .filter(|snapshot| Some(&snapshot.id) == editing_id.as_ref())
    {
        if let Some(ref current) = w.editor.editing {
            w.editor.redo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let name = snapshot.name.clone();
        let slides = snapshot.slides.clone();
        w.editor.editing = Some(snapshot);
        // The name is part of the snapshot, so it is written back with the
        // slides: the editor and the library must not disagree.
        if w.editor
            .editing
            .as_ref()
            .is_some_and(|pres| pres.name != name)
            && let Err(e) = w.services.presentations.rename(&pres_id, &name)
        {
            w.set_error(format!("undo: {e}"));
        }
        if let Err(e) = w.services.presentations.replace_slides(&pres_id, &slides) {
            w.set_error(format!("undo: {e}"));
        }
        w.load_presentations();
        clamp_selected_slide(w);
        load_slide_for_editing(w);
        // The layer selection belongs to the slide, and the slide is still
        // there: the Layers tab and the canvas outline keep showing the
        // object the operator was working on.
        crate::ui::layers::load_layer_for_editing(w);
    }
    cx.notify();
}

/// Redo the last undone editing change.
pub(crate) fn redo(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if w.shell.current_mode != ViewMode::Edit {
        return;
    }
    w.editor.undo_coalesce.clear();
    let editing_id = w.editor.editing.as_ref().map(|pres| pres.id.clone());
    if let Some(snapshot) = w
        .editor
        .redo_stack
        .pop()
        .filter(|snapshot| Some(&snapshot.id) == editing_id.as_ref())
    {
        if let Some(ref current) = w.editor.editing {
            w.editor.undo_stack.push(current.clone());
        }
        let pres_id = snapshot.id.clone();
        let name = snapshot.name.clone();
        let slides = snapshot.slides.clone();
        w.editor.editing = Some(snapshot);
        if w.editor
            .editing
            .as_ref()
            .is_some_and(|pres| pres.name != name)
            && let Err(e) = w.services.presentations.rename(&pres_id, &name)
        {
            w.set_error(format!("redo: {e}"));
        }
        if let Err(e) = w.services.presentations.replace_slides(&pres_id, &slides) {
            w.set_error(format!("redo: {e}"));
        }
        w.load_presentations();
        clamp_selected_slide(w);
        load_slide_for_editing(w);
        crate::ui::layers::load_layer_for_editing(w);
    }
    cx.notify();
}

/// Keep the selected slide inside the document.
///
/// An undo can bring back a document with fewer slides, and a selection
/// past the end would leave the canvas blank and every further edit would
/// be dropped.
fn clamp_selected_slide(w: &mut MainWindow) {
    let count = w
        .editor
        .editing
        .as_ref()
        .map(|pres| pres.slides.len())
        .unwrap_or(0);
    if count == 0 {
        w.editor.selected_slide_index = None;
        return;
    }
    w.editor.selected_slide_index = Some(w.editor.selected_slide_index.unwrap_or(0).min(count - 1));
}

/// Toggle reduced motion.
///
/// With reduced motion the compositor changes the slide without a transition.
pub(crate) fn toggle_reduce_motion(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The setting's own handler saves the choice, so the shortcut does the
    // same: a preference that is lost on restart is not a preference.
    crate::ui::output::toggle_reduce_motion(w, !w.ui.reduce_motion, cx);
    cx.notify();
}

/// Global Escape handling: close the topmost overlay (output settings, looks
/// matrix, shortcuts overlay, new-presentation modal, delete confirmation),
/// then fall through to ending inline slide-text editing.
pub(crate) fn escape_pressed(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    if w.output.settings_open {
        w.output.settings_open = false;
        w.output.settings_status_message = None;
    } else if w.shell.rail_context_target.is_some() {
        // The rail's context menu closes with Escape, like every other menu.
        w.shell.rail_context_target = None;
    } else if w.output.matrix_open {
        w.output.matrix_open = false;
    } else if w.triggers.panel_open {
        w.triggers.panel_open = false;
    } else if w.recording.panel_open {
        w.recording.panel_open = false;
    } else if w.ui.shortcuts_open {
        w.ui.shortcuts_open = false;
    } else if w.ui.about_open {
        w.ui.about_open = false;
    } else if w.editor.new_presentation_open {
        w.editor.new_presentation_open = false;
    } else if w.editor.show_delete_confirmation {
        w.editor.delete_target_id = None;
        w.service.to_delete = None;
        w.song.to_delete = None;
        w.bible.translation_to_delete = None;
        w.editor.show_delete_confirmation = false;
    } else if w.canvas.editing.is_some() {
        // Leave the text being edited on the canvas.
        crate::ui::canvas::end_text_edit(w, cx);
        window.focus(&w.focus_handle, cx);
    } else {
        // Release any focused text input by returning focus to the shell
        // root, so presenter keys work again.
        window.focus(&w.focus_handle, cx);
        crate::ui::slides::update(w, crate::ui::slides::Message::EndInlineTextEditing, cx);
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Presenting core
// ---------------------------------------------------------------------------

/// Activate a slide with an animated transition (re-selecting the live slide
/// is a no-op so re-clicking does not restart a transition).
pub(crate) fn select_slide(w: &mut MainWindow, i: usize, cx: &mut Context<MainWindow>) {
    reset_slide_context(w);
    activate_slide(w, i, true, cx);
}

/// Activate the last slide of the live presentation.
pub(crate) fn select_last_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    reset_slide_context(w);
    let last = w
        .presenting
        .presentation
        .as_ref()
        .map(|p| p.slides.len().saturating_sub(1))
        .unwrap_or(0);
    activate_slide(w, last, true, cx);
}

/// Advance to the next slide (no-op at the end of the deck).
pub(crate) fn next_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    reset_slide_context(w);
    let next = {
        let Some(ref pres) = w.presenting.presentation else {
            return;
        };
        let current_i = w.presenting.slide_index;
        let next = (current_i + 1).min(pres.slides.len().saturating_sub(1));
        if next == current_i && w.presenting.slide_layer_active {
            return;
        }
        next
    };
    activate_slide(w, next, true, cx);
}

/// Step back to the previous slide (no-op at the first slide).
pub(crate) fn prev_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    reset_slide_context(w);
    let prev = {
        if w.presenting.presentation.is_none() {
            return;
        }
        let current_i = w.presenting.slide_index;
        let prev = current_i.saturating_sub(1);
        if prev == current_i && w.presenting.slide_layer_active {
            return;
        }
        prev
    };
    activate_slide(w, prev, true, cx);
}

/// Jump to the next slide whose group label starts with `group_prefix`
/// (wrapping around the deck). A missing "Chorus" group clears the slide
/// layer, matching lyric-show operator muscle memory.
pub(crate) fn jump_to_group(w: &mut MainWindow, group_prefix: &str, cx: &mut Context<MainWindow>) {
    if let Some(ref pres) = w.presenting.presentation {
        let pfx = group_prefix.to_lowercase();
        let cur = w.presenting.slide_index;
        let next_match = pres
            .slides
            .iter()
            .enumerate()
            .skip(cur + 1)
            .find(|(_, s)| {
                s.group
                    .as_deref()
                    .map(|g| g.to_lowercase().starts_with(&pfx))
                    .unwrap_or(false)
            })
            .or_else(|| {
                pres.slides.iter().enumerate().find(|(_, s)| {
                    s.group
                        .as_deref()
                        .map(|g| g.to_lowercase().starts_with(&pfx))
                        .unwrap_or(false)
                })
            });
        if let Some((idx, _)) = next_match {
            select_slide(w, idx, cx);
        } else if pfx == "chorus" {
            clear_slide(w, cx);
        } else {
            cx.notify();
        }
        return;
    }
    cx.notify();
}

fn reset_slide_context(w: &mut MainWindow) {
    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.cue_submenu = false;
    w.presenting.transition_submenu = false;
}

/// Core slide activation: clamp the index, begin the target transition, push
/// the slide to NDI, and fire the target slide's cues. Also used by the
/// playlist-driven presenting flow to open plan items.
pub(crate) fn activate_slide(
    w: &mut MainWindow,
    to_idx: usize,
    animate: bool,
    cx: &mut Context<MainWindow>,
) {
    let from_slide = {
        let Some(ref pres) = w.presenting.presentation else {
            return;
        };
        if pres.slides.get(to_idx).is_none() {
            return;
        }
        if animate {
            if to_idx == w.presenting.slide_index && w.presenting.slide_layer_active {
                return;
            }
            pres.slides.get(w.presenting.slide_index).cloned()
        } else {
            None
        }
    };
    begin_slide_change(w, from_slide, to_idx, animate, cx);
    // The stage monitor shows the next slide and the notes, so it has to
    // follow the slide change.
    crate::ui::stage::publish_stage_if_open(w);
}

fn begin_slide_change(
    w: &mut MainWindow,
    from_slide: Option<Slide>,
    to_idx: usize,
    animate: bool,
    cx: &mut Context<MainWindow>,
) {
    let (mut transition, ndi_slide, cues) = {
        let Some(ref pres) = w.presenting.presentation else {
            return;
        };
        let Some(target) = pres.slides.get(to_idx) else {
            return;
        };
        (target.transition, target.clone(), target.cues.clone())
    };

    if matches!(transition, Transition::Cut) {
        transition = w.presenting.global_transition;
    }

    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.cue_submenu = false;
    w.presenting.slide_index = to_idx;
    w.presenting.slide_layer_active = true;
    w.presenting.media_layer_active = true;
    w.output.black_screen = false;

    // The transition itself is drawn by the compositor, which runs the
    // slide's own transition between the two compositions. Reduced motion
    // turns it into a cut there.
    let _ = (animate, from_slide, transition);

    // The layers are back on, so the broadcast has to be told before it
    // draws the new slide.
    crate::ui::props::push_overlay(w);
    for ndi in w.presenting.ndi_outputs.values() {
        ndi.set_slide(ndi_slide.clone());
    }
    // Follow the activated slide: start its video (or stop the player when
    // the target slide has none).
    crate::ui::video::on_presenter_slide_changed(w, to_idx);
    queue_slide_cues(w, cues);
    cx.notify();
}

/// The delayed cues that are waiting to fire.
///
/// A cue belongs to the slide it was queued from. When the operator moves
/// on, the waiting cues are dropped: a "Clear All" that fires thirty
/// seconds later, on a slide the operator has moved on to, would blank the
/// output in the middle of the show.
#[derive(Default)]
pub(crate) struct PendingCues {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl PendingCues {
    /// Drop the cues that are still waiting.
    pub(crate) fn clear(&mut self) {
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }

    /// Drop the waiting cues and queue the given ones.
    ///
    /// An immediate cue is sent straight away; a delayed one is spawned with
    /// a sleep and is dropped when the next slide's cues are queued.
    pub(crate) fn replace(&mut self, cues: &[Cue], tx: &tokio::sync::mpsc::Sender<Action>) {
        self.clear();
        for cue in cues {
            if !cue.enabled {
                continue;
            }
            if cue.delay_ms == 0 {
                let _ = tx.try_send(cue.action.clone());
                continue;
            }
            let delayed_tx = tx.clone();
            let action = cue.action.clone();
            let delay_ms = cue.delay_ms;
            let handle = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let _ = delayed_tx.send(action).await;
            });
            self.handles.push(handle);
        }
    }
}

/// Queue a slide's cues onto the trigger channel: immediate actions are
/// try-sent, delayed ones are spawned with a sleep.
///
/// The cues of the slide the operator just left are dropped. A cue belongs
/// to its slide: a "Clear All" that fires thirty seconds later, on a slide
/// the operator has moved on to, would blank the output in the middle of the
/// show.
fn queue_slide_cues(w: &mut MainWindow, cues: Vec<Cue>) {
    let tx = w.triggers.manager.sender();
    w.presenting.pending_cues.replace(&cues, &tx);
}

/// Open a slide of the live presentation in the editor.
pub(crate) fn edit_slide(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.cue_submenu = false;
    if let Some(ref pres) = w.presenting.presentation {
        set_editing(w, Some(pres.clone()));
        w.editor.selected_slide_index = Some(idx);
        w.shell.current_mode = ViewMode::Edit;
        load_slide_for_editing(w);
    }
    cx.notify();
}

/// Persist a cue onto the given live slide.
/// The delay the next cue added to a slide will carry.
pub(crate) fn set_pending_cue_delay(w: &mut MainWindow, ms: u64, cx: &mut Context<MainWindow>) {
    w.editor.pending_cue_delay_ms = ms;
    cx.notify();
}

pub(crate) fn add_slide_cue(
    w: &mut MainWindow,
    idx: usize,
    cue: Cue,
    cx: &mut Context<MainWindow>,
) {
    reset_slide_context(w);
    if let Some(ref mut pres) = w.presenting.presentation
        && let Some(slide) = pres.slides.get_mut(idx)
    {
        slide.cues.push(cue);
        let updated = slide.clone();
        let pres_id = pres.id.clone();
        let _ = w.services.presentations.update_slide(&pres_id, &updated);
    }
    cx.notify();
}

/// Remove every cue from the given live slide.
pub(crate) fn clear_slide_cues(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    reset_slide_context(w);
    if let Some(ref mut pres) = w.presenting.presentation
        && let Some(slide) = pres.slides.get_mut(idx)
    {
        slide.cues.clear();
        let updated = slide.clone();
        let pres_id = pres.id.clone();
        let _ = w.services.presentations.update_slide(&pres_id, &updated);
    }
    cx.notify();
}

/// Set the per-slide transition for the given live slide.
pub(crate) fn set_slide_transition(
    w: &mut MainWindow,
    idx: usize,
    transition: Transition,
    cx: &mut Context<MainWindow>,
) {
    reset_slide_context(w);
    if let Some(ref mut pres) = w.presenting.presentation
        && let Some(slide) = pres.slides.get_mut(idx)
    {
        slide.transition = transition;
        let updated = slide.clone();
        let pres_id = pres.id.clone();
        let _ = w.services.presentations.update_slide(&pres_id, &updated);
    }
    cx.notify();
}

/// Set the slide-strip grid density (clamped to 2–6 columns).
pub(crate) fn set_slide_grid_cols(w: &mut MainWindow, cols: usize, cx: &mut Context<MainWindow>) {
    w.presenting.slide_grid_cols = cols.clamp(2, 6);
    cx.notify();
}

/// Switch the slide workspace between Easy, Grid and Table presentation.
pub(crate) fn set_slide_view_mode(
    w: &mut MainWindow,
    mode: SlideViewMode,
    cx: &mut Context<MainWindow>,
) {
    w.presenting.slide_view_mode = mode;
    cx.notify();
}

/// Set the transition applied to slides whose own transition is `Cut`.
pub(crate) fn set_global_transition(
    w: &mut MainWindow,
    transition: Transition,
    cx: &mut Context<MainWindow>,
) {
    w.presenting.global_transition = transition;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Output layer clears
// ---------------------------------------------------------------------------

/// Clear everything the output is showing.
///
/// The screen goes black, the layers are switched off, the props and the
/// message are dismissed and the sound stops. Every path that clears the
/// output does the same thing: a remote that blacks the projector while the
/// music keeps playing is a different show from the button beside it.
pub(crate) fn clear_output(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.output.black_screen = true;
    w.presenting.slide_layer_active = false;
    w.presenting.media_layer_active = false;
    for p in &mut w.props.manager.props {
        p.visible = false;
    }
    w.props.manager.clear_message();
    // The sound stops with the picture: an output that is cleared but still
    // playing the music is not cleared. A clip carries its own sound, and it
    // has to stop with the media layer — otherwise the soundtrack keeps
    // playing in the room.
    crate::ui::audio::stop(w);
    crate::ui::video::stop_video(w);
    // The broadcast composites the slide itself, so it has to be told about
    // the cleared props, the dismissed message and the switched-off layers.
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_black_screen(w, cx);
    cx.notify();
}

/// Clear everything the output is showing.
///
/// The same as [`clear_output`]: "clear all" means one thing, whichever way
/// it is triggered.
pub(crate) fn clear_all(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    clear_output(w, cx);
}

/// Clear the output to the logo.
///
/// The logo is the house's own mark: it is what the screen shows when there
/// is nothing to show — between the songs, while the band comes on, and at
/// the end of the service. The logo stays on the screen while everything
/// else is cleared, so the screen is never blank and never showing the
/// slide that was there before.
pub(crate) fn clear_to_logo(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // Everything goes, and the logo comes on.
    for prop in &mut w.props.manager.props {
        prop.visible = false;
    }
    w.props.manager.clear_message();
    w.presenting.slide_layer_active = false;
    w.presenting.media_layer_active = false;
    w.output.black_screen = false;
    crate::ui::audio::stop(w);
    crate::ui::video::stop_video(w);

    // The logo is only turned on if there is one: a screen with no logo
    // shows nothing rather than a broken picture.
    let has_logo = w.props.manager.props.iter().any(|p| p.id == "logo");
    if has_logo {
        crate::ui::props::set_prop_visible(w, "logo", true);
    }
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Clear only the slide layer from the output.
pub(crate) fn clear_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.presenting.slide_layer_active = false;
    // The broadcast composites the slide itself, so it has to be told that
    // the slide layer is off.
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Clear only the media layer from the output.
pub(crate) fn clear_media(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.presenting.media_layer_active = false;
    // The media layer owns its sound: a clip that is still playing would
    // keep its soundtrack in the room after the picture is gone.
    crate::ui::video::stop_video(w);
    crate::ui::audio::stop(w);
    // The broadcast composites the slide itself, so it has to be told that
    // the media layer is off.
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Hide every visible prop.
pub(crate) fn clear_props(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    for p in &mut w.props.manager.props {
        p.visible = false;
    }
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Send the announcement the operator wrote to the output.
///
/// The announcement is the app's own word, so it is its own layer: the
/// operator's alert and the app's announcement can both be on the screen,
/// and either can be taken away without touching the other.
pub(crate) fn send_announcement(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let text = w.announcement_input.read(cx).value().to_string();
    if text.trim().is_empty() {
        return;
    }
    w.props.manager.set_announcement(text);
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    // The field is emptied, so the next announcement is not the last one
    // sent again. The input needs the window to be cleared, so the clearing
    // waits for the next render.
    w.announcement_input_clear_pending = true;
    cx.notify();
}

/// The announcement the operator is typing changed.
///
/// The input is the source of the text, so the value is read from it when
/// the announcement is sent rather than being copied here.
pub(crate) fn announcement_input_changed(
    w: &mut MainWindow,
    _value: String,
    cx: &mut Context<MainWindow>,
) {
    let _ = w;
    cx.notify();
}

/// Take the announcement off the output.
///
/// The announcement is the app's own word, so clearing it is the app
/// saying nothing again — the screen goes back to what it was showing.
pub(crate) fn clear_announcements(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.props.manager.clear_announcement();
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Dismiss the live/audience alert message.
pub(crate) fn clear_messages(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.props.manager.clear_message();
    crate::ui::props::push_overlay(w);
    crate::ui::ndi::ndi_send_current(w, cx);
    cx.notify();
}

/// Stop audio playback (remote `ClearAudio` and the dock button).
pub(crate) fn clear_audio(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    crate::ui::audio::stop(w);
    cx.notify();
}

/// Clear the layers of a clear group.
///
/// A clear group is a named set of layers, so one press clears "everything
/// but the slide" and another clears the slide and the sound and leaves the
/// props up. The layers are cleared in the order they are listed, so the
/// sound stops with the picture: an output that is cleared but still playing
/// the music is not cleared.
pub(crate) fn clear_layers(
    w: &mut MainWindow,
    layers: &[crate::ui::clear_groups::Layer],
    cx: &mut Context<MainWindow>,
) {
    for layer in layers {
        match layer {
            crate::ui::clear_groups::Layer::Presentation => clear_slide(w, cx),
            crate::ui::clear_groups::Layer::PresentationMedia => clear_media(w, cx),
            crate::ui::clear_groups::Layer::Props => clear_props(w, cx),
            crate::ui::clear_groups::Layer::Messages => clear_messages(w, cx),
            crate::ui::clear_groups::Layer::Announcements => clear_announcements(w, cx),
            crate::ui::clear_groups::Layer::Music => clear_audio(w, cx),
        }
    }
    cx.notify();
}

/// Toggle the audience-screen black state (`B` key and dock button); the NDI
/// output mirrors the new state.
pub(crate) fn toggle_black_screen(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.output.black_screen = !w.output.black_screen;
    if w.output.black_screen {
        crate::ui::ndi::ndi_black_screen(w, cx);
        // The stage monitor says BLACK as well: the speaker has to know that
        // the audience sees nothing, and the monitor only hears about it
        // when the content is published.
        crate::ui::stage::publish_stage_if_open(w);
    } else {
        // The broadcast follows the audience back: it was blacked with the
        // audience, so it comes back with it. Leaving the stream blacked
        // would leave the "Live" switch dark while the stream is live, and
        // pressing it would black the stream again instead of restoring it.
        crate::ui::ndi::set_black_screen(w, false, cx);
    }
    cx.notify();
}

/// Select which look target the dock preview mirrors.
pub(crate) fn select_preview_screen(
    w: &mut MainWindow,
    screen_id: String,
    cx: &mut Context<MainWindow>,
) {
    w.presenting.preview_screen_target = screen_id;
    cx.notify();
}

/// Toggle the in-app stage display swap (speaker view in the center column).
///
/// The swap works in any mode: the operator may want the speaker view while
/// they are working on a slide.
pub(crate) fn toggle_stage_display(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.presenting.stage_display_active = !w.presenting.stage_display_active;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Timers (dock Timers tab + remote actions)
// ---------------------------------------------------------------------------

/// Start the elapsed stage timer (dock button and remote `Action::StartTimer`).
pub(crate) fn start_timer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    stage::start_timer(w);
    cx.notify();
}

/// Stop the elapsed stage timer.
pub(crate) fn stop_timer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    stage::stop_timer(w);
    cx.notify();
}

/// Reset the elapsed stage timer to zero.
pub(crate) fn reset_timer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    stage::reset_timer(w);
    cx.notify();
}

/// Extend (or arm) the stage countdown.
pub(crate) fn countdown_add(w: &mut MainWindow, secs: u64, cx: &mut Context<MainWindow>) {
    stage::countdown_add(w, secs);
    cx.notify();
}

/// Clear the stage countdown.
pub(crate) fn countdown_clear(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    stage::countdown_clear(w);
    cx.notify();
}

// ---------------------------------------------------------------------------
// Live messages (dock Messages tab + Show-controls alert field)
// ---------------------------------------------------------------------------

/// Send the composed live/audience alert message and clear the input.
pub(crate) fn send_live_message(
    w: &mut MainWindow,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let text = w.alert_input.read(cx).value().trim().to_string();
    if !text.is_empty() {
        w.props.manager.set_message(text);
        w.alert_input.update(cx, |state, input_cx| {
            state.set_value("", window, input_cx);
        });
        crate::ui::props::push_overlay(w);
    }
    cx.notify();
}

/// Dismiss the live/audience alert message.
pub(crate) fn dismiss_live_message(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.props.manager.clear_message();
    crate::ui::props::push_overlay(w);
    cx.notify();
}

/// Mirror the live-message input's value (subscription-driven).
pub(crate) fn live_message_input_changed(
    w: &mut MainWindow,
    _value: String,
    cx: &mut Context<MainWindow>,
) {
    let _ = w;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Editor field loader (slide → inspector text fields)
// ---------------------------------------------------------------------------

/// Fill the editor's per-slide text fields from the selected slide. Ported
/// from the iced-era `MainWindow::load_slide_for_editing`; the layer loader is
/// folded in (layer selection is cleared whenever the slide changes).
pub(crate) fn load_slide_for_editing(w: &mut MainWindow) {
    let mut video_path: Option<String> = None;
    let (text, font_size, transition_dur, group, notes) = if let Some(slide) = w
        .editor
        .selected_slide_index
        .and_then(|index| w.editor.editing.as_ref().and_then(|p| p.slides.get(index)))
    {
        video_path = slide.video_path();
        let dur = transition_duration_ms(&slide.transition);
        let group = slide.group.clone().unwrap_or_default();
        let notes = slide.notes.clone().unwrap_or_default();
        // The text comes from the domain's single rule — the text object
        // the editor writes to. Reading the slide's legacy content instead
        // showed the panel one thing while it wrote another: a slide whose
        // text lives in an object showed an empty box, and typing in it
        // overwrote the object's real text.
        match slide.editable_text(None) {
            Some((txt, style)) => (txt.clone(), style.font_size, dur, group, notes),
            None => (String::new(), 72.0, dur, group, notes),
        }
    } else {
        (String::new(), 72.0, 500, String::new(), String::new())
    };
    w.editor.editing_slide_text = text;
    w.editor.editing_slide_font_size = font_size.to_string();
    w.editor.editing_transition_duration = transition_dur.to_string();
    w.editor.editing_group_label = group;
    w.editor.editing_slide_notes = notes;
    w.layer.selected_index = None;
    // The transport in the inspector belongs to the slide being edited, so
    // the player has to be that slide's file. A transport left pointing at
    // another file would play the wrong thing.
    if let Some(path) = video_path {
        crate::ui::video::load_video(w, &path);
    } else {
        // The new slide carries no video, so the old clip's sound has to
        // stop with it: a soundtrack that keeps playing under a slide
        // without a video is a show that is out of sync.
        crate::ui::video::stop_video(w);
    }
}

#[cfg(test)]
mod pending_cue_tests {
    use super::*;
    use crate::domain::Action;

    /// A cue belongs to the slide it was queued from. When the operator
    /// moves on, the waiting cue must not fire: a "Clear All" that fires
    /// thirty seconds later, on another slide, would blank the output in
    /// the middle of the show.
    #[tokio::test]
    async fn a_waiting_cue_is_dropped_when_the_next_slide_is_queued() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Action>(8);
        let mut pending = PendingCues::default();

        // The cue is due soon, so without the drop it would fire while the
        // test waits.
        pending.replace(
            &[Cue::new("Clear All", Action::ClearOutput).with_delay(200)],
            &tx,
        );

        // The operator moves on: the next slide's cues replace the waiting
        // ones.
        pending.replace(&[], &tx);

        // Wait past the moment the cue was due.
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(
            rx.try_recv().is_err(),
            "the cue of the slide the operator left must not fire"
        );
    }

    /// A cue with no delay is sent straight away, so it is not waiting for
    /// anything.
    #[tokio::test]
    async fn an_immediate_cue_fires_right_away() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Action>(8);
        let mut pending = PendingCues::default();

        pending.replace(&[Cue::new("Next Slide", Action::NextSlide)], &tx);

        assert_eq!(rx.try_recv().ok(), Some(Action::NextSlide));
    }

    /// A disabled cue does nothing at all.
    #[tokio::test]
    async fn a_disabled_cue_does_not_fire() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Action>(8);
        let mut pending = PendingCues::default();
        let mut cue = Cue::new("Next Slide", Action::NextSlide);
        cue.enabled = false;

        pending.replace(&[cue], &tx);
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(rx.try_recv().is_err(), "a disabled cue must not fire");
    }

    /// The show plays the document the operator has been editing.
    ///
    /// A live copy that was left behind would put the slide from before the
    /// edit on the projector — and going back to the editor would then load
    /// that stale copy and lose the edit.
    #[test]
    fn the_show_plays_the_edited_document() {
        let mut edited = crate::domain::Presentation::new("Edited".to_string());
        let mut edited_slide = crate::domain::Slide::new_text("The new title".to_string());
        edited_slide.id = "edited-slide".to_string();
        edited.slides.push(edited_slide);
        let mut live = crate::domain::Presentation::new("Live".to_string());
        let mut live_slide = crate::domain::Slide::new_text("The old title".to_string());
        live_slide.id = "live-slide".to_string();
        live.slides.push(live_slide);

        let shown = live_document(Some(&edited), Some(&live)).expect("a document");
        assert_eq!(
            shown.slides[0].id, edited.slides[0].id,
            "the projector shows what the operator edited"
        );

        // With nothing being edited, the live document is played.
        let shown = live_document(None, Some(&live)).expect("a document");
        assert_eq!(shown.slides[0].id, live.slides[0].id);
    }
}
