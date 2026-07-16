//! Shared delete-confirmation dialog lifecycle.
//!
//! The three delete flows (presentations, songs, playlists) all open the same
//! modal window tracked by `MainWindow::editor.delete_confirm_window_id`. This
//! module centralises the window open/close logic so each feature only supplies
//! its own target-kind and the actual delete + reload step.

use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Size, Task, window};

/// Fixed size of the delete-confirmation modal.
const DELETE_DIALOG_SIZE: Size = Size::new(420.0, 260.0);

/// Which feature's target-id field the pending delete refers to.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DeleteTarget {
    Presentation,
    Song,
    Playlist,
}

/// Open (or refocus) the shared delete-confirmation window and remember the
/// `target` id that should be deleted on confirm.
pub(crate) fn open_delete_dialog(
    w: &mut MainWindow,
    target: DeleteTarget,
    id: String,
) -> Task<Message> {
    if let Some(existing) = w.editor.delete_confirm_window_id {
        return window::gain_focus(existing);
    }
    match target {
        DeleteTarget::Presentation => w.editor.delete_target_id = Some(id),
        DeleteTarget::Song => w.song.to_delete = Some(id),
        DeleteTarget::Playlist => w.service.to_delete = Some(id),
    }
    w.editor.show_delete_confirmation = true;
    let (win_id, open_task) = window::open(window::Settings {
        size: DELETE_DIALOG_SIZE,
        resizable: false,
        ..Default::default()
    });
    w.editor.delete_confirm_window_id = Some(win_id);
    open_task.map(|_| Message::Noop)
}

/// Close the confirmation window and clear the shared confirm flags.
///
/// Feature-specific target ids are cleared by the caller (in `confirm_delete`
/// or `cancel_delete`) since they live in per-feature state.
pub(crate) fn close_delete_dialog(w: &mut MainWindow) -> Task<Message> {
    w.editor.show_delete_confirmation = false;
    w.editor.delete_target_id = None;
    if let Some(win_id) = w.editor.delete_confirm_window_id.take() {
        return window::close(win_id);
    }
    Task::none()
}
