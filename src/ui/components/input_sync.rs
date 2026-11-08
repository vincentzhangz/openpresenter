//! Keeping a text field and the state behind it in step.
//!
//! The panel fields are a second copy of the model: the model is the truth,
//! and the field is what the operator types into. The two are the same when
//! the value changed from outside the field — a different slide, a preset, an
//! undo — and the field has to be rewritten then.

use gpui_kit::component::input::InputState;
use gpui_kit::{Context, Entity, Focusable, Window};

/// Write a value into a text field when the field does not hold it.
///
/// A field the operator is typing in is left alone. The change arrives after
/// the keystroke, so the state is a moment behind the field: writing it over
/// the field would eat the character that was just typed, and a title the
/// operator cannot type is a title they cannot write.
pub fn sync_text_field(
    entity: &Entity<InputState>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<impl Sized>,
) -> bool {
    if entity.read(cx).focus_handle(cx).is_focused(window) {
        return false;
    }
    let current = entity.read(cx).value().to_string();
    if current != value {
        entity.update(cx, |state, input_cx| {
            state.set_value(value, window, input_cx);
        });
        return true;
    }
    false
}
