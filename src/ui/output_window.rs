use crate::domain::Transition;
use crate::ui::messages::Message;
use crate::ui::presenter::canvas::{CompositeLayers, presenter_composite_panel};
use iced::{Background, Color, Element, Length, widget::container};

pub fn view<'a>(
    current_slide: Option<&'a crate::domain::Slide>,
    from_slide: Option<&'a crate::domain::Slide>,
    transition: Transition,
    progress: f32,
    video_frame: Option<&'a iced::widget::image::Handle>,
    black_screen: bool,
    layers: &CompositeLayers,
) -> Element<'a, Message> {
    if black_screen
        || (current_slide.is_none() && layers.props.is_empty() && layers.live_message.is_none())
    {
        return container("")
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into();
    }

    container(presenter_composite_panel(
        current_slide,
        from_slide,
        transition,
        progress,
        video_frame,
        layers,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(Color::BLACK)),
        ..Default::default()
    })
    .into()
}
