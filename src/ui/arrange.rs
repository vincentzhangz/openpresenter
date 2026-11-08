//! The arrangement commands: where the selected objects sit on the slide.
//!
//! The menu, the canvas and the inspector arrange a slide the same way: an
//! object that moves in the menu and stays put in the editor is an
//! arrangement the operator cannot trust. The drawing rules themselves are
//! the domain's (`domain::Slide::align_objects` and its siblings), so the
//! editor, the canvas and the output show the same picture.
//!
//! The commands act on the slide that is being edited: with no slide open
//! there is nothing to arrange, so nothing happens. The shell keeps the
//! commands out of Show mode and away from the fields the operator is
//! typing in (`ui/shell/mod.rs::editing_keys_allowed`), so a command never
//! fires over a half-typed title.

use crate::domain::Slide;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::ArrangeCommand;
use gpui_kit::Context;

/// Apply an arrangement command to the slide being edited.
///
/// Returns `true` when the slide really changed.
///
/// A command that finds nothing to move — an object that already sits where
/// it is put, a text object that cannot be mirrored — changes nothing, and
/// it says so: an undo entry that restores what is already on the screen
/// swallows the next undo.
pub(crate) fn apply(
    w: &mut MainWindow,
    command: ArrangeCommand,
    cx: &mut Context<MainWindow>,
) -> bool {
    // The commands are given the objects the operator picked: the editor
    // picks one object at a time or a group of them. Unlock All reaches
    // further — it frees every object of the slide — but it is one of the
    // arrangement commands, so it is given the same target as the others.
    let indices = w.selected_object_indices();
    if indices.is_empty() {
        // Nothing is picked: there is no object to arrange.
        return false;
    }
    let Some(slide) = w.get_current_slide() else {
        // No slide is open: there is nothing to arrange.
        return false;
    };
    // The command is tried on a copy of the slide first, so the undo
    // snapshot is only taken when the command really changes something.
    let mut copy = slide.clone();
    if !arrange(&mut copy, &indices, command) {
        return false;
    }
    // A reorder moves the objects in the slide's list, so the selection is
    // remembered before the move and put back after it.
    let selection = match command {
        ArrangeCommand::Reorder(_) => Some(Selection::capture(w, &indices)),
        _ => None,
    };

    // The snapshot is taken before the slide is touched: one taken
    // afterwards would restore the slide *with* the arrangement on it, and
    // the undo would do nothing.
    w.push_undo();

    let Some(slide) = w.get_current_slide_mut() else {
        return false;
    };
    // The copy said the command changes something, so the slide changes:
    // it is the same slide and the same command.
    arrange(slide, &indices, command);

    if let Some(selection) = selection {
        selection.restore(w);
    }

    // The panel fields are a second copy of the model, so they follow the
    // arrangement: the inspector shows where the object really is.
    crate::ui::layers::load_layer_for_editing(w);
    // The slide is saved with the rest of the slide's edits, so the
    // arrangement is still there after a restart.
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
    true
}

/// Apply the command to a slide.
///
/// The rules are the domain's: the same command moves the objects the same
/// way in the editor, on the canvas and in the output.
fn arrange(slide: &mut Slide, indices: &[usize], command: ArrangeCommand) -> bool {
    match command {
        ArrangeCommand::Align(mode) => slide.align_objects(indices, mode),
        ArrangeCommand::Reorder(direction) => slide.reorder_objects(indices, direction),
        ArrangeCommand::FlipHorizontal => slide.flip_objects(indices, true),
        ArrangeCommand::FlipVertical => slide.flip_objects(indices, false),
        ArrangeCommand::Lock => slide.set_objects_locked(indices, true),
        // Unlock All frees every object of the slide: it is the way out of a
        // slide whose objects are all locked.
        ArrangeCommand::UnlockAll => slide.unlock_all(),
    }
}

/// The objects the selection points at, remembered by identity.
///
/// The draw order is the list order, so a reorder moves the objects in the
/// list — and the selection holds places in that list. A selection that
/// keeps its place would point at a different object after the move: the
/// panel would show another object than the one the operator arranged.
struct Selection {
    /// The primary object: the one the panel fields show.
    primary: Option<String>,
    /// The objects picked together, in the order they were picked.
    group: Vec<String>,
    /// Whether the objects were picked as a group. The list is empty when a
    /// single object is selected, and the editor reads that as one object.
    grouped: bool,
    /// The object whose text is being edited on the canvas.
    editing: Option<String>,
}

impl Selection {
    /// Remember the objects the selection points at.
    fn capture(w: &MainWindow, indices: &[usize]) -> Self {
        let grouped = !w.layer.selected_indices.is_empty();
        let Some(slide) = w.get_current_slide() else {
            return Self {
                primary: None,
                group: Vec::new(),
                grouped,
                editing: None,
            };
        };
        let id_of = |index: usize| slide.layers.get(index).map(|object| object.id.clone());
        Self {
            primary: w.layer.selected_index.and_then(id_of),
            group: indices.iter().filter_map(|index| id_of(*index)).collect(),
            grouped,
            editing: w.canvas.editing.and_then(id_of),
        }
    }

    /// Point the selection at the objects it pointed at before the move.
    fn restore(&self, w: &mut MainWindow) {
        let Some(slide) = w.get_current_slide() else {
            return;
        };
        let (primary, group, editing) = {
            let index_of = |id: &str| slide.layers.iter().position(|object| object.id == id);
            (
                self.primary.as_deref().and_then(index_of),
                self.group
                    .iter()
                    .filter_map(|id| index_of(id.as_str()))
                    .collect::<Vec<usize>>(),
                self.editing.as_deref().and_then(index_of),
            )
        };
        w.layer.selected_index = primary;
        if self.grouped {
            // The group is written back only when every object was found: a
            // list that points at objects that are not there would arrange
            // the wrong objects the next time.
            w.layer.selected_indices = if group.len() == self.group.len() {
                group
            } else {
                Vec::new()
            };
        }
        // The text being edited on the canvas belongs to an object, and the
        // object moved: the edit stays with it.
        w.canvas.editing = editing;
    }
}
