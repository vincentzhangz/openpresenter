use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::shell;
use crate::ui::theme;
use iced::{Element, Length, widget::container};

/// Bottom collapsible media bin spanning the width under the left + center
/// columns (ProPresenter convention).
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, Message> {
    let bin = shell::show::media_bin_workspace(&w.library.assets, w.library.selected_id.as_deref());
    container(bin)
        .width(Length::Fill)
        .style(theme::panel_style)
        .into()
}
