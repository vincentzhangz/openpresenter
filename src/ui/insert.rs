//! The insert menu: what goes on the slide the operator is editing.
//!
//! Every command puts a **new object** on the slide being edited, selects
//! it and saves the slide. The insertion is an edit like any other: it can
//! be undone, it survives a restart, and the panel fields follow the new
//! selection, so the operator can move or type into what they just put on
//! the slide.
//!
//! ## The kinds the domain does not have yet
//!
//! The domain knows four object kinds: text, shape, image and video. A web
//! page, a countdown, a timecode, a stage message, a playback marker, a
//! screen preview and a chord chart are not among them, and this module
//! cannot add them — the domain owns the model.
//!
//! Until the domain grows those kinds, each of those commands inserts a
//! **text object that carries the right content**: a timer is inserted as a
//! countdown label, a timecode as a timecode label, and so on. That is a
//! known limitation, not a hidden gap: what lands on the slide is a picture
//! of the thing the operator asked for, not the live thing. The countdown
//! label does not count down, the timecode label does not advance and the
//! web object does not load a page — they are text the operator can type
//! over, until the domain has a kind of their own.
//!
//! The placeholders differ from each other on purpose: the operator has to
//! be able to tell on the slide which kind they inserted.

use crate::domain::{ImageFit, Object, ObjectContent, ShapeType};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InsertCommand, InspectorTab};
use gpui_kit::Context;

/// The object a command puts on the slide.
///
/// The text placeholders are the stand-ins described at the top of this
/// module: each one names the kind it stands for, so the operator can see
/// what they inserted.
fn object_for(command: InsertCommand) -> Object {
    match command {
        InsertCommand::Text => Object::new_text(String::from("Text")),
        InsertCommand::Shape => Object::new_shape(ShapeType::Rectangle),
        // A media object is a media slide that is waiting for a file: the
        // path is empty, and the inspector's picker fills it in. The media
        // covers the slide, the way a media object does everywhere else.
        InsertCommand::Media => Object::new_image(String::new(), ImageFit::default()),
        InsertCommand::Web => Object::new_text(String::from("Web address")),
        InsertCommand::Timer => Object::new_text(String::from("Countdown 00:00")),
        InsertCommand::PlaybackMarker => Object::new_text(String::from("Marker 0:00")),
        InsertCommand::ScreenPreview => Object::new_text(String::from("Screen preview")),
        InsertCommand::StageMessage => Object::new_text(String::from("Stage message")),
        InsertCommand::Timecode => Object::new_text(String::from("Timecode 00:00:00")),
        InsertCommand::ChordChart => Object::new_text(String::from("Chord chart")),
    }
}

/// The inspector tab the inserted object is worked on in.
///
/// The tab follows the kind of the object: a text object is typed on the
/// Text tab, a shape's fill and corners are set on the Shape tab, and a
/// media object is filled in on the Build tab — where the picker is, so a
/// media slide that has no file yet still offers it.
fn tab_for(object: &Object) -> InspectorTab {
    match &object.content {
        ObjectContent::Text { .. } => InspectorTab::Text,
        ObjectContent::Shape { .. } => InspectorTab::Layers,
        _ => InspectorTab::Slide,
    }
}

/// Put the object a command names on the slide being edited.
///
/// Returns `true` when an object was inserted.
///
/// The command acts on the slide being edited: with no slide open there is
/// nothing to put an object on, so nothing happens and the command says
/// so. The same rule keeps the insert menu from writing into a document
/// that is not on the screen — in Show mode the edited document has been
/// handed over to the live projection, so the editor holds nothing.
pub(crate) fn apply(
    w: &mut MainWindow,
    command: InsertCommand,
    cx: &mut Context<MainWindow>,
) -> bool {
    if w.get_current_slide().is_none() {
        return false;
    }
    let object = object_for(command);
    let tab = tab_for(&object);

    // The insertion is an edit like any other, so it can be undone. The
    // snapshot is taken before the slide is touched: one taken afterwards
    // would restore the slide *with* the new object on it, and the undo
    // would do nothing.
    w.push_undo();

    let Some(slide) = w.get_current_slide_mut() else {
        return false;
    };
    // A slide that still carries its content in the slide itself is
    // converted first: a new object on a slide that has not been converted
    // would hide the slide's own title, because the objects are the
    // picture.
    slide.migrate_to_objects();
    // The object goes on top: the slide is drawn back to front, so an
    // object with the default depth would be hidden behind the ones that
    // are already there. It lands in the middle of the slide, which is
    // where the app puts every new object, so the operator sees it.
    let index = slide.add_object_on_top(object);

    // The slide is saved with the rest of the slide's edits, so the object
    // is still there after a restart.
    crate::ui::layers::save_current_slide(w, cx);

    // The new object is the selected one: the operator inserted it, so it
    // is the thing they want to move, type into or delete.
    w.layer.selected_index = Some(index);
    // The panel fields are a second copy of the model, so they follow the
    // selection: the inspector shows the new object's text and geometry.
    crate::ui::layers::load_layer_for_editing(w);
    // The tab follows the object that was inserted, so the operator lands
    // on the control that edits it.
    w.shell.inspector_tab = tab;

    cx.notify();
    true
}
