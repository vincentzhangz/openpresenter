use crate::domain::Object;
use crate::ui::main_window::MainWindow;

/// Persist the current slide (the one being edited) to the database via the
/// presentation service, surfacing any error in the UI toast.
pub(crate) fn save_current_slide(w: &mut MainWindow) {
    if let Some(slide) = w.get_current_slide() {
        w.persist_slide(slide.clone());
    }
}

/// Return a mutable reference to the currently selected layer of the slide
/// being edited, if any.
pub(crate) fn selected_layer_mut(w: &mut MainWindow) -> Option<&mut Object> {
    let idx = w.layer.selected_index?;
    w.get_current_slide_mut()?.layers.get_mut(idx)
}
