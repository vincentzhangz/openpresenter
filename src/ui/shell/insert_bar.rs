//! The editor's object toolbar.
//!
//! The toolbar sits under the canvas and holds the things that can go on a
//! slide. It is the same list as the Insert menu, in the same order: an
//! operator who learned one has learned the other, and a toolbar that held
//! something the menu did not would be a toolbar that lies.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InsertCommand, Message};
use crate::ui::theme;

/// The objects the operator can put on a slide.
const BUTTONS: [(&str, InsertCommand); 10] = [
    ("Text", InsertCommand::Text),
    ("Media", InsertCommand::Media),
    ("Shapes", InsertCommand::Shape),
    ("Web", InsertCommand::Web),
    ("Stage Message", InsertCommand::StageMessage),
    ("Timecode", InsertCommand::Timecode),
    ("Timers", InsertCommand::Timer),
    ("Playback Marker", InsertCommand::PlaybackMarker),
    ("Screen Preview", InsertCommand::ScreenPreview),
    ("Chord Chart", InsertCommand::ChordChart),
];

/// The object toolbar, shown under the canvas while a slide is edited.
///
/// The toolbar is the editor's own: it is not shown in Show mode, where
/// there is no slide to add anything to.
pub fn view(_w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let mut bar = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .px(px(10.))
        .py(px(5.))
        .bg(theme::c(theme::BG_DARK))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL));

    for (index, (label, command)) in BUTTONS.into_iter().enumerate() {
        bar = bar.child(
            Button::new(SharedString::from(format!("insert-bar-{index}")))
                .label(label)
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(Message::Insert(command), window, cx);
                })),
        );
    }
    bar
}
