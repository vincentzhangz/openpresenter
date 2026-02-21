use crate::ui::messages::Message;
use iced::{
    Alignment, Color, Element, Length,
    widget::{Space, container, row, text},
};

/// Colour palette used to visually distinguish slide group labels.
pub const GROUP_COLORS: [(u8, u8, u8); 8] = [
    (52, 120, 246),
    (255, 149, 0),
    (52, 199, 89),
    (255, 59, 48),
    (175, 82, 222),
    (255, 204, 0),
    (0, 199, 190),
    (162, 132, 94),
];

/// Deterministically maps a group label string to one of the [`GROUP_COLORS`] entries.
///
/// The same label always produces the same colour across all panels.
pub fn group_color(label: &str) -> Color {
    let hash: usize = label.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    let (r, g, b) = GROUP_COLORS[hash % GROUP_COLORS.len()];
    Color::from_rgb8(r, g, b)
}

/// Renders a coloured dot + uppercase label on a lightly tinted background chip.
///
/// Used in slide lists and presenter panels to identify which group a slide belongs to.
pub fn group_label_widget<'a>(label: &str) -> Element<'a, Message> {
    let accent = group_color(label);
    let label_upper = label.to_uppercase();

    let dot = container(Space::new().width(8).height(8)).style(move |_: &iced::Theme| {
        iced::widget::container::Style {
            background: Some(iced::Background::Color(accent)),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    container(
        row![
            dot,
            Space::new().width(6),
            text(label_upper).size(10).color(accent)
        ]
        .align_y(Alignment::Center)
        .padding([5, 8]),
    )
    .width(Length::Fill)
    .style(move |_: &iced::Theme| iced::widget::container::Style {
        background: Some(iced::Background::Color(Color { a: 0.08, ..accent })),
        border: iced::Border {
            color: accent,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}
