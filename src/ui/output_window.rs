use crate::domain::Transition;
use crate::ui::messages::Message;
use crate::ui::presenter::canvas::presenter_canvas_panel;
use iced::{Background, Color, Element, Length, widget::container};

pub fn view<'a>(
    current_slide: Option<&'a crate::domain::Slide>,
    from_slide: Option<&'a crate::domain::Slide>,
    transition: Transition,
    progress: f32,
    video_frame: Option<&'a iced::widget::image::Handle>,
    black_screen: bool,
) -> Element<'a, Message> {
    if black_screen || current_slide.is_none() {
        return container("")
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into();
    }

    container(presenter_canvas_panel(
        current_slide,
        from_slide,
        transition,
        progress,
        video_frame,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::BLACK)),
        ..Default::default()
    })
    .into()
}
