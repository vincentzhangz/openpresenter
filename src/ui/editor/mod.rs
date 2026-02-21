pub mod canvas;
pub mod inspector;
pub mod slide_list;

use crate::slides::Presentation;
use crate::ui::messages::{InspectorTab, Message};
use crate::ui::theme;
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, container, row, text, text_input},
};

#[allow(clippy::too_many_arguments)]
pub fn editor_view<'a>(
    presentation: &'a Presentation,
    selected_index: Option<usize>,
    tab: InspectorTab,
    editing_text: &'a str,
    editing_font_size: &'a str,
    editing_transition_dur: &'a str,
    editing_group_label: &'a str,
    editing_notes: &'a str,
    themes: &'a [crate::slides::SlideTheme],
    selected_theme_id: Option<&'a str>,
    new_theme_name: &'a str,
    video_playing: bool,
    video_looping: bool,
    video_volume: f32,
    video_muted: bool,
    video_speed: f64,
    video_position: f64,
    video_duration: f64,
    video_frame: Option<&'a iced::widget::image::Handle>,
    layer_state: inspector::LayerPanelState,
) -> Element<'a, Message> {
    let editing_name = &presentation.name;
    let current_slide = selected_index.and_then(|i| presentation.slides.get(i));

    let top_bar = container(
        row![
            button(text("‹ Library").size(12).color(theme::TEXT_SECONDARY))
                .on_press(Message::BackToList)
                .padding([6, 12])
                .style(theme::ghost_button),
            Space::new().width(10),
            text_input("Presentation name", editing_name)
                .on_input(Message::RenamePresentationChanged)
                .padding([6, 8])
                .size(13)
                .width(220),
            button(text("Rename").size(11).color(theme::TEXT_SECONDARY))
                .on_press(Message::RenamePresentation)
                .padding([6, 10])
                .style(theme::secondary_button),
            Space::new().width(Length::Fill),
            button(text("🗑 Delete").size(11))
                .on_press(Message::DeletePresentationClicked(presentation.id.clone()))
                .padding([6, 12])
                .style(theme::danger_button),
        ]
        .spacing(8)
        .padding([8, 14])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::toolbar_style);

    let work_area = row![
        slide_list::slide_list(presentation, selected_index),
        canvas::canvas_panel(current_slide, video_frame),
        inspector::inspector_panel(
            current_slide,
            tab,
            editing_text,
            editing_font_size,
            editing_transition_dur,
            editing_group_label,
            editing_notes,
            selected_index,
            themes,
            selected_theme_id,
            new_theme_name,
            video_playing,
            video_looping,
            video_volume,
            video_muted,
            video_speed,
            video_position,
            video_duration,
            &layer_state,
        ),
    ]
    .width(Length::Fill)
    .height(Length::Fill);

    container(iced::widget::column![top_bar, work_area])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
