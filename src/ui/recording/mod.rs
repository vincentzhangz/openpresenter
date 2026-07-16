use crate::recording::RecordingState;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::{
    Element, Length, Task,
    widget::{Space, button, column, row, text, text_input},
};
use iced_font_awesome::fa_icon_solid;

/// Messages owned by the Recording feature module (see `AGENTS.md`).
///
/// `ToggleRecordingPanel` stays as a root variant (it is a global panel
/// visibility toggle shared with the navbar).
#[derive(Debug, Clone)]
pub enum Message {
    Start,
    Stop,
    PathChanged(String),
    FpsChanged(String),
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Recording(msg)
}

/// Render the recording panel.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let mgr = &w.recording.manager;
    recording_panel(
        mgr.state,
        &mgr.output_path,
        mgr.fps,
        mgr.elapsed().map(|d| d.as_secs()),
        mgr.frames_captured(),
    )
}

/// Dispatch a recording message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::Start => start(w),
        Message::Stop => stop(w),
        Message::PathChanged(s) => path_changed(w, s),
        Message::FpsChanged(s) => fps_changed(w, s),
    }
}

pub(crate) fn start(w: &mut MainWindow) -> Task<RootMessage> {
    if let Err(e) = w.recording.manager.start() {
        w.set_error(format!("[recording] Failed to start: {e}"));
    }
    Task::none()
}

pub(crate) fn stop(w: &mut MainWindow) -> Task<RootMessage> {
    w.recording.manager.stop();
    Task::none()
}

pub(crate) fn path_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.recording.manager.output_path = s;
    Task::none()
}

pub(crate) fn fps_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    if let Ok(fps) = s.parse::<u32>() {
        w.recording.manager.fps = fps.clamp(1, 120);
    }
    Task::none()
}

pub(crate) fn toggle_panel(w: &mut MainWindow) -> Task<RootMessage> {
    w.recording.panel_open = !w.recording.panel_open;
    Task::none()
}

pub fn recording_panel<'a>(
    state: RecordingState,
    output_path: &'a str,
    fps: u32,
    elapsed_secs: Option<u64>,
    frames_captured: u64,
) -> Element<'a, RootMessage> {
    let header = row![
        text("Recording").size(16.0),
        Space::new().width(Length::Fill),
        button("X").on_press(RootMessage::ToggleRecordingPanel),
    ]
    .spacing(8);

    let (status_label, record_btn) = match state {
        RecordingState::Idle => (
            "Ready".to_string(),
            button("Record").on_press(wrap(Message::Start)),
        ),
        RecordingState::Recording => {
            let secs = elapsed_secs.unwrap_or(0);
            let mm = secs / 60;
            let ss = secs % 60;
            (
                format!("Recording  {mm:02}:{ss:02}  ({frames_captured} frames)"),
                button(fa_icon_solid("stop").size(13.0_f32)).on_press(wrap(Message::Stop)),
            )
        }
        RecordingState::Finishing => (
            "Finishing…".to_string(),
            button("Stop").on_press(wrap(Message::Stop)),
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
            .on_input(move |v| wrap(Message::PathChanged(v)))
            .width(Length::Fill),
    ]
    .spacing(8);

    let fps_row = row![
        text("FPS:").size(12.0),
        text_input("60", &fps.to_string())
            .on_input(move |v| wrap(Message::FpsChanged(v)))
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
