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

/// Semantic, ProPresenter-style palette that maps a slide group label to a colour.
///
/// Unlike [`group_color`] (which is hash-based), this assigns meaningful colours:
/// verses are blue, choruses pink, bridges purple, tags red, etc. Used for the
/// per-slide ribbons/strips and the context-menu group swatches so the same group
/// always reads with the same colour across the app.
pub fn group_option_color(label: &str) -> Color {
    match label {
        "Verse" => Color::from_rgb8(30, 146, 245),
        "Verse 1" => Color::from_rgb8(29, 129, 229),
        "Verse 2" => Color::from_rgb8(24, 114, 214),
        "Verse 3" => Color::from_rgb8(21, 97, 194),
        "Verse 4" => Color::from_rgb8(18, 84, 171),
        "Verse 5" => Color::from_rgb8(17, 74, 152),
        "Verse 6" => Color::from_rgb8(15, 66, 136),
        "Chorus" => Color::from_rgb8(222, 20, 109),
        "Chorus 1" => Color::from_rgb8(205, 18, 99),
        "Chorus 2" => Color::from_rgb8(186, 16, 90),
        "Chorus 3" => Color::from_rgb8(167, 15, 82),
        "Chorus 4" => Color::from_rgb8(148, 14, 74),
        "Bridge" => Color::from_rgb8(120, 35, 206),
        "Bridge 1" => Color::from_rgb8(108, 30, 186),
        "Bridge 2" => Color::from_rgb8(95, 25, 164),
        "Bridge 3" => Color::from_rgb8(83, 22, 145),
        "PreChorus" => Color::from_rgb8(203, 40, 156),
        "Tag" => Color::from_rgb8(214, 56, 45),
        "Intro" => Color::from_rgb8(185, 182, 44),
        "Ending" => Color::from_rgb8(164, 163, 39),
        "Outro" => Color::from_rgb8(144, 143, 35),
        "Interlude" => Color::from_rgb8(54, 190, 94),
        "Vamp" => Color::from_rgb8(45, 173, 84),
        "Turnaround" => Color::from_rgb8(39, 156, 74),
        "Blank" => Color::from_rgb8(0, 0, 0),
        _ => Color::from_rgb8(95, 95, 95),
    }
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
