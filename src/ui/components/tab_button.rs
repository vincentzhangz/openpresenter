use crate::ui::messages::Message;
use crate::ui::theme;
use iced::{
    Element, Length,
    widget::{Row, button, container, text},
};

pub fn tab_btn<'a>(label: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(text(label).size(11))
        .on_press(on_press)
        .width(Length::Fill)
        .padding([6, 4])
        .style(if active {
            theme::primary_button
        } else {
            theme::tab_inactive_button
        })
        .into()
}

/// Wraps a row of [`tab_btn`] elements in the standard [`theme::tab_bar_style`] container.
pub fn tab_bar<'a>(tabs: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let r = Row::with_children(tabs).spacing(2).width(Length::Fill);
    container(r)
        .width(Length::Fill)
        .style(theme::tab_bar_style)
        .into()
}
