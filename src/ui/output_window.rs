use iced::widget::container;
use iced::{Color, Element, Length, Task};

pub struct OutputWindow {
    background_color: Color,
}

#[derive(Debug, Clone)]
pub enum Message {
    SetBackground(Color),
}

impl OutputWindow {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                background_color: Color::BLACK,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetBackground(color) => {
                self.background_color = color;
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        container("")
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(self.background_color)),
                ..Default::default()
            })
            .into()
    }

    pub fn set_background(&mut self, color: Color) {
        self.background_color = color;
    }
}

impl Default for OutputWindow {
    fn default() -> Self {
        Self::new().0
    }
}
