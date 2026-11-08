//! H.264 recording panel.
//!
//! Ported from the iced-era `ui/recording/mod.rs`: start/stop of the
//! `RecordingManager` session, output-path and FPS fields, and a status
//! readout that surfaces elapsed time and dropped frames inline. The navbar
//! Capture button starts/stops directly; the panel itself opens as a modal
//! from the Show Controls tab (`Message::ToggleRecordingPanel`).

use std::time::Duration;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, Styled, Window, div, px};

use crate::recording::RecordingState;
use crate::ui::main_window::MainWindow;
use crate::ui::theme;

#[derive(Debug, Clone)]
pub enum Message {
    /// Start a recording session (navbar Capture button and panel).
    Start,
    /// Stop and finalize the running session.
    ///
    /// The preset can ask the operator to confirm first, so the message
    /// arrives here and the confirmation is shown before the take is ended.
    Stop,
    /// The operator confirmed the stop.
    ConfirmStop,
    /// The operator kept recording.
    CancelStop,
    /// Change the output file path.
    PathChanged(String),
    /// Change the capture frame rate (clamped to 1–60, the rate the
    /// capture tick can deliver).
    FpsChanged(String),
}

/// Dispatch a recording message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Start => start(w, cx),
        Message::Stop => {
            // The preset can ask the operator to confirm: a take that ends
            // by accident is a take that cannot be put back.
            if w.recording.manager.confirm_before_stopping()
                && w.recording.manager.state == crate::recording::RecordingState::Recording
            {
                w.recording.confirm_stop_pending = true;
                cx.notify();
                return;
            }
            stop(w, cx);
        }
        Message::ConfirmStop => {
            w.recording.confirm_stop_pending = false;
            stop(w, cx);
        }
        Message::CancelStop => {
            w.recording.confirm_stop_pending = false;
            cx.notify();
        }
        Message::PathChanged(s) => path_changed(w, s),
        Message::FpsChanged(s) => fps_changed(w, s),
    }
}

/// Open (or close) the recording panel modal.
pub(crate) fn toggle_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.recording.panel_open = !w.recording.panel_open;
    cx.notify();
}

fn start(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The recording has the same size as the output it captures, so the
    // frames the compositor produces are the frames the encoder expects.
    w.recording.manager.frame_width = w.app_config.output.width;
    w.recording.manager.frame_height = w.app_config.output.height;
    w.recording.manager.resolution_changed = false;
    if let Err(e) = w.recording.manager.start() {
        w.set_error(format!("[recording] Failed to start: {e}"));
    }
    cx.notify();
}

fn stop(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let was_recording = w.recording.manager.state == RecordingState::Recording;
    w.recording.manager.stop();
    cx.notify();
    if !was_recording || w.recording.manager.state != RecordingState::Finishing {
        // Nothing was running, or the file is already being written out.
        return;
    }
    // The encoder writes the file out on a background thread, so the panel
    // stays in the "Finishing" state until the file is there. The UI thread
    // never waits for the encoder.
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(50))
                .await;
            match this.read_with(cx, |w, _| w.recording.manager.is_finished()) {
                Ok(true) => break,
                Ok(false) => {}
                // The window is gone: there is nobody left to tell.
                Err(_) => return,
            }
        }
        // Let the panel paint the "Finishing" state before the recording goes
        // back to idle.
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;
        let _ = this.update(cx, |w, cx| {
            if let Some(result) = w.recording.manager.take_finish_result()
                && let Err(e) = result
            {
                w.set_error(format!("[recording] Failed to write the file: {e}"));
            }
            cx.notify();
        });
    })
    .detach();
}

pub(crate) fn path_changed(w: &mut MainWindow, s: String) {
    w.recording.manager.output_path = s;
}

/// The frame rates the recorder can actually deliver.
///
/// The capture tick runs at most 60 times a second, so a file that claims
/// 120 frames a second would carry half of them twice: the panel's number and
/// the file's time base have to be the same.
pub(crate) const MAX_FPS: u32 = 60;

pub(crate) fn fps_changed(w: &mut MainWindow, s: String) {
    if let Ok(fps) = s.parse::<u32>() {
        w.recording.manager.fps = fps.clamp(1, MAX_FPS);
    }
}

/// Keep the panel inputs consistent with the recording manager: the path
/// input mirrors `manager.output_path` (the operator's own setting — a
/// generated name belongs to the session and never lands in the input), and
/// the FPS input snaps to the clamped value once the user typed a valid
/// number. Invalid partial input is left untouched so typing is never
/// clobbered.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    // A field the operator is typing in is left alone: the path they are
    // typing is theirs, and the manager's empty path would wipe it.
    let path = w.recording.manager.output_path.clone();
    crate::ui::components::sync_text_field(&w.recording_path_input, &path, window, cx);

    // The field snaps to the rate the recorder will really use, so the
    // panel and the file agree: a field that reads 120 while the capture
    // runs at 60 is a field that lies.
    let canonical = w.recording.manager.fps.to_string();
    let fps_value = w.recording_fps_input.read(cx).value().to_string();
    if fps_value != canonical && fps_value.parse::<u32>().is_ok() {
        w.recording_fps_input.update(cx, |state, input_cx| {
            state.set_value(&canonical, window, input_cx);
        });
    }
}

// ---------------------------------------------------------------------------
// View (modal panel)
// ---------------------------------------------------------------------------

/// Render the recording panel body (wrapped in a titled dialog modal).
pub(crate) fn recording_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let mgr = &w.recording.manager;
    let state = mgr.state;
    let elapsed_secs = mgr.elapsed().map(|d| d.as_secs());
    let frames_captured = mgr.frames_captured();
    let frames_dropped = mgr.frames_dropped();
    let session_path = mgr.session_path.clone();

    let (status_label, recording) = match state {
        RecordingState::Idle => {
            // The totals of the take that ended last stay on screen: the
            // operator wants to see them after the stop.
            match elapsed_secs {
                Some(secs) => {
                    let mm = secs / 60;
                    let ss = secs % 60;
                    let mut status =
                        format!("Ready  — last take {mm:02}:{ss:02}  ({frames_captured} frames");
                    if frames_dropped > 0 {
                        status.push_str(&format!(", {frames_dropped} dropped"));
                    }
                    status.push(')');
                    (status, false)
                }
                None => (String::from("Ready"), false),
            }
        }
        RecordingState::Recording => {
            let secs = elapsed_secs.unwrap_or(0);
            let mm = secs / 60;
            let ss = secs % 60;
            // Surface dropped frames inline: a stalled encoder silently
            // degrades the recording, and the operator needs to see it happen.
            let mut status = if frames_dropped > 0 {
                format!(
                    "Recording  {mm:02}:{ss:02}  ({frames_captured} frames, {frames_dropped} dropped)"
                )
            } else {
                format!("Recording  {mm:02}:{ss:02}  ({frames_captured} frames)")
            };
            if mgr.resolution_changed {
                // The encoder keeps the size it started with, so the frames
                // of the new resolution are left out.
                status.push_str("  — resolution changed, size unchanged");
            }
            if mgr.encoder_failed() {
                // The encoder died: every frame is dropped and the file is
                // not written. The operator has to know before the show.
                status.push_str("  — the encoder stopped");
            }
            (status, true)
        }
        RecordingState::Finishing => (String::from("Finishing…"), true),
    };

    div()
        .w(px(430.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(13.))
                        .text_color(if recording {
                            theme::c(theme::DANGER_RED)
                        } else {
                            theme::c(theme::TEXT_SECONDARY)
                        })
                        .child(status_label),
                )
                .child(
                    Button::new("recording-toggle")
                        .label(match state {
                            RecordingState::Idle => "Record",
                            RecordingState::Recording => "Stop",
                            RecordingState::Finishing => "Finishing…",
                        })
                        .when(recording, |b| b.secondary())
                        .when(!recording, |b| b.primary())
                        .small()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            let msg = match state {
                                RecordingState::Idle => Message::Start,
                                RecordingState::Recording => Message::Stop,
                                // The recording is already being written out.
                                RecordingState::Finishing => return,
                            };
                            w.handle_message(
                                crate::ui::messages::Message::Recording(msg),
                                window,
                                cx,
                            );
                        })),
                ),
        )
        // The preset. A service is recorded the same way every week, so the
        // settings are chosen once and kept: the button opens them.
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Preset:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(
                            w.app_config
                                .capture
                                .active_preset()
                                .map(|preset| preset.name.clone())
                                .unwrap_or_else(|| String::from("None")),
                        ),
                )
                .child(
                    Button::new("recording-capture-settings")
                        .label("Capture Settings…")
                        .ghost()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::CaptureSettings(
                                    crate::ui::capture_settings::Message::Open,
                                ),
                                window,
                                cx,
                            );
                        })),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Output file:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&w.recording_path_input.clone())),
                ),
        )
        // The file the session writes to. The operator's own path is a
        // setting; a generated name belongs to the session, so the operator
        // needs to see where the frames actually went. It is only the file
        // of a take that is running: a finished take's file named next to
        // "Ready" is a file the operator might think is still being written.
        .when_some(
            session_path.filter(|_| state != crate::recording::RecordingState::Idle),
            |this, path| {
                this.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_SECONDARY))
                                .child("Recording to:"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .child(path.to_string_lossy().into_owned()),
                        ),
                )
            },
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("FPS:"),
                )
                .child(
                    div()
                        .w(px(80.))
                        .child(Input::new(&w.recording_fps_input.clone())),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("(1–60)"),
                ),
        )
}
