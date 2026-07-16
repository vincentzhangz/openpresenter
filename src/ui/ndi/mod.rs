use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::output;

/// Messages owned by the NDI feature module (see `AGENTS.md`).
///
/// `NdiBlackScreen` and `ClearOutput` stay as root variants: they are global
/// output controls (clear/black the live output) emitted from the Show and
/// Presenter views, not NDI-specific toggles.
#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    SendCurrent,
}

/// Dispatch an NDI message.
pub fn update(w: &mut MainWindow, msg: Message) -> iced::Task<RootMessage> {
    match msg {
        Message::Toggle => output::toggle_ndi(w),
        Message::SendCurrent => output::ndi_send_current(w),
    }
}
