use crate::recording::RecordingState;
use crate::ui::messages::Message;
use iced::{
    Element, Length,
    widget::{Space, button, column, row, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

pub fn recording_panel<'a>(
    state: RecordingState,
    output_path: &'a str,
    fps: u32,
    elapsed_secs: Option<u64>,
    frames_captured: u64,
) -> Element<'a, Message> {
    let header = row![
        text("Recording").size(16.0),
        Space::new().width(Length::Fill),
        button("X").on_press(Message::ToggleRecordingPanel),
    ]
    .spacing(8);

    let (status_label, record_btn) = match state {
        RecordingState::Idle => (
            "Ready".to_string(),
            button("Record").on_press(Message::RecordingStart),
        ),
        RecordingState::Recording => {
            let secs = elapsed_secs.unwrap_or(0);
            let mm = secs / 60;
            let ss = secs % 60;
            (
                format!("Recording  {mm:02}:{ss:02}  ({frames_captured} frames)"),
                button(fa_icon_solid("stop").size(13.0)).on_press(Message::RecordingStop),
            )
        }
        RecordingState::Finishing => (
            "Finishing…".to_string(),
            button("Stop").on_press(Message::RecordingStop),
        ),
    };

    let status_row = row![
        text(status_label).size(13.0),
        Space::new().width(Length::Fill),
        record_btn,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let path_row = row![
        text("Output file:").size(12.0),
        text_input("~/Videos/output.mp4", output_path)
            .on_input(Message::RecordingPathChanged)
            .width(Length::Fill),
    ]
    .spacing(8);

    let fps_row = row![
        text("FPS:").size(12.0),
        text_input("60", &fps.to_string())
            .on_input(Message::RecordingFpsChanged)
            .width(80),
        text("(1–120)").size(11.0),
    ]
    .spacing(8);

    column![header, status_row, path_row, fps_row]
        .spacing(10)
        .padding(12)
        .width(Length::Fill)
        .into()
}
