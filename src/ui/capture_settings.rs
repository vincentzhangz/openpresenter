//! The capture presets.
//!
//! A service is recorded the same way every week, so the settings are
//! chosen once and kept: the folder the take goes into, the name it is
//! given, the frame rate it is made at, and whether the operator is asked
//! before the recording stops.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::config::CapturePreset;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// Messages owned by the Capture Settings feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the capture settings.
    Open,
    /// Close the capture settings.
    Close,
    /// The operator picked the preset the recordings are made with.
    SelectPreset(String),
    /// The operator typed a name for a new preset.
    NewNameChanged(String),
    /// The operator added a preset.
    AddPreset,
    /// The operator took a preset away.
    DeletePreset(String),
    /// The operator changed the folder the take goes into.
    DestinationChanged(String),
    /// The operator changed the name the take is given.
    FileNameChanged(String),
    /// The operator changed the frame rate.
    FpsChanged(u32),
    /// The operator turned the confirmation on or off.
    ToggleConfirm(bool),
    /// The operator picked the codec the take is made in.
    SetCodec(crate::config::CaptureCodec),
}

/// Apply a message from the capture settings.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    let config = &mut w.app_config.capture;
    match msg {
        Message::Open => {
            w.shell.capture_settings_open = true;
        }
        Message::Close => {
            w.shell.capture_settings_open = false;
        }
        Message::SelectPreset(id) => {
            config.active = Some(id);
            save(w, cx);
        }
        Message::NewNameChanged(name) => {
            w.capture_preset_name = name;
        }
        Message::AddPreset => {
            let name = w.capture_preset_name.trim().to_string();
            if name.is_empty() {
                return;
            }
            let preset = CapturePreset {
                name,
                ..CapturePreset::default()
            };
            config.presets.push(preset.clone());
            config.active = Some(preset.id.clone());
            w.capture_preset_name.clear();
            save(w, cx);
        }
        Message::DeletePreset(id) => {
            config.presets.retain(|preset| preset.id != id);
            if config.active.as_deref() == Some(id.as_str()) {
                config.active = config.presets.first().map(|preset| preset.id.clone());
            }
            save(w, cx);
        }
        Message::DestinationChanged(destination) => {
            if let Some(preset) = config.active_preset_mut() {
                preset.destination = destination;
                save(w, cx);
            }
        }
        Message::FileNameChanged(file_name) => {
            if let Some(preset) = config.active_preset_mut() {
                preset.file_name = file_name;
                save(w, cx);
            }
        }
        Message::FpsChanged(fps) => {
            if let Some(preset) = config.active_preset_mut() {
                preset.fps = fps.clamp(1, 60);
                save(w, cx);
            }
        }
        Message::ToggleConfirm(confirm) => {
            if let Some(preset) = config.active_preset_mut() {
                preset.confirm_before_stopping = confirm;
                save(w, cx);
            }
        }
        Message::SetCodec(codec) => {
            if let Some(preset) = config.active_preset_mut() {
                preset.codec = codec;
                save(w, cx);
            }
        }
    }
    // The recorder follows the preset: the take is made the way the
    // operator chose it, not the way it was last time.
    let preset = w.app_config.capture.active_preset().cloned();
    w.recording.manager.apply_preset(preset);
    cx.notify();
}

/// Save the capture settings and put them on the recorder.
fn save(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let _ = w.app_config.save();
    cx.notify();
}

/// The capture settings window.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let config = w.app_config.capture.clone();
    let active = config.active_preset().cloned();

    let mut panel = div()
        .w(px(420.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // The presets.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("PRESETS"),
    );
    if config.presets.is_empty() {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No Presets"),
        );
    }
    let mut list = div().flex().flex_col().gap(px(4.));
    for preset in &config.presets {
        let id = preset.id.clone();
        let selected = config.active.as_deref() == Some(id.as_str());
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    Button::new(SharedString::from(format!("capture-preset-{id}")))
                        .label(preset.name.clone())
                        .ghost()
                        .xsmall()
                        .when(selected, |button| button.primary())
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::CaptureSettings(Message::SelectPreset(id.clone())),
                                    window,
                                    cx,
                                );
                            }
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("capture-preset-delete-{id}")))
                        .label("Remove")
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::CaptureSettings(Message::DeletePreset(id.clone())),
                                    window,
                                    cx,
                                );
                            }
                        })),
                ),
        );
    }
    panel = panel.child(list);

    // A new preset.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .w(px(200.))
                    .child(Input::new(&w.capture_preset_name_input.clone())),
            )
            .child(
                Button::new("capture-preset-add")
                    .label("Add")
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            RootMessage::CaptureSettings(Message::AddPreset),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    // The settings of the preset that is chosen.
    if let Some(preset) = active {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("ACTIVE SETTINGS"),
        );
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(110.))
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Destination:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&w.capture_destination_input.clone())),
                ),
        );
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(110.))
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("File name:"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&w.capture_file_name_input.clone())),
                ),
        );
        panel = panel.child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("{date} and {time} are the moment the take starts, and {name} is the preset's own name."),
        );

        // The frame rate.
        let mut rates = div().flex().flex_row().items_center().gap(px(4.));
        for fps in [24u32, 25, 30, 50, 60] {
            rates = rates.child(
                Button::new(SharedString::from(format!("capture-fps-{fps}")))
                    .label(format!("{fps} fps"))
                    .ghost()
                    .xsmall()
                    .when(preset.fps == fps, |button| button.primary())
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::CaptureSettings(Message::FpsChanged(fps)),
                            window,
                            cx,
                        );
                    })),
            );
        }
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(110.))
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Frame rate:"),
                )
                .child(rates),
        );

        // The codec. H.264 is what everything plays; H.265 is a smaller
        // file for the same picture.
        let mut codecs = div().flex().flex_row().items_center().gap(px(4.));
        for codec in crate::config::CaptureCodec::ALL {
            codecs = codecs.child(
                Button::new(SharedString::from(format!(
                    "capture-codec-{}",
                    codec.label()
                )))
                .label(codec.label())
                .ghost()
                .xsmall()
                .when(preset.codec == codec, |button| button.primary())
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::CaptureSettings(Message::SetCodec(codec)),
                        window,
                        cx,
                    );
                })),
            );
        }
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(110.))
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Codec:"),
                )
                .child(codecs),
        );

        // The confirmation.
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Confirm Before Stopping"),
                )
                .child(
                    Button::new("capture-confirm")
                        .label(if preset.confirm_before_stopping {
                            "On"
                        } else {
                            "Off"
                        })
                        .ghost()
                        .xsmall()
                        .when(preset.confirm_before_stopping, |button| button.primary())
                        .on_click(cx.listener(|w, _, window, cx| {
                            let confirm = !w
                                .app_config
                                .capture
                                .active_preset()
                                .map(|preset| preset.confirm_before_stopping)
                                .unwrap_or(false);
                            w.handle_message(
                                RootMessage::CaptureSettings(Message::ToggleConfirm(confirm)),
                                window,
                                cx,
                            );
                        })),
                ),
        );
    }

    panel
}
