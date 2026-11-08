//! Top navbar: app identity, editor tools, mode selector, media/looks
//! toggles, and the live-status cluster (record / NDI / audience / stage /
//! help / settings). Ported from the iced-era `ui/navbar.rs`; FontAwesome
//! icons become text labels and the shared button styling keeps the same
//! active/live color semantics.

use gpui_kit::prelude::*;
use gpui_kit::{App, ClickEvent, Context, IntoElement, Styled, TestSupportExt, Window, div, px};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, Message, SidebarTab, ViewMode};
use crate::ui::theme;

/// The height of the top bar.
///
/// The centre column starts under it, so the context menu that opens at the
/// pointer has to know where the column begins.
pub(crate) const NAVBAR_HEIGHT: f32 = 46.0;

/// Render the top navbar.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let text_active =
        w.shell.current_mode == ViewMode::Edit && w.shell.inspector_tab == InspectorTab::Text;
    let theme_active =
        w.shell.current_mode == ViewMode::Edit && w.shell.inspector_tab == InspectorTab::Theme;
    let show_active = w.shell.current_mode == ViewMode::Show;
    let edit_active = w.shell.current_mode == ViewMode::Edit;
    let bible_active = w.shell.sidebar_tab == SidebarTab::Bible;
    let media_open = w.shell.media_bin_open;
    let matrix_open = w.output.matrix_open;
    let ndi_active = !w.presenting.ndi_outputs.is_empty();
    let recording_state = w.recording.manager.state;
    let recording_active = recording_state == crate::recording::RecordingState::Recording;
    // The button says what the capture is doing: a recording that is being
    // finalised is not a recording that can be started again.
    let recording_label = match recording_state {
        crate::recording::RecordingState::Recording => "Stop",
        crate::recording::RecordingState::Finishing => "Finishing…",
        crate::recording::RecordingState::Idle => "Capture",
    };
    let audience_active = w.output.window_id.is_some();
    let stage_active = w.presenting.stage_display_active;

    div()
        .w_full()
        .h(px(NAVBAR_HEIGHT))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .px(px(10.))
        .bg(theme::c(theme::BG_TOOLBAR))
        .border_b_1()
        .border_color(theme::c(theme::BORDER_STRONG))
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child("OpenPresenter"),
        )
        .child(toolbar_divider())
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(2.))
                .child(tool_btn(
                    "nav-search",
                    "Search",
                    "",
                    false,
                    false,
                    cx.listener(|w, _, window, cx| {
                        w.handle_message(Message::FocusSearch, window, cx);
                    }),
                )),
        )
        .child(tool_btn(
            "nav-text",
            "Text",
            "",
            text_active,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::OpenTextEditor, window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-theme",
            "Theme",
            "",
            theme_active,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::OpenTheme, window, cx);
            }),
        ))
        .child(toolbar_divider())
        .child(tool_btn(
            "nav-show",
            "Show",
            "",
            show_active,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SwitchMode(ViewMode::Show), window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-edit",
            "Edit",
            "",
            edit_active,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SwitchMode(ViewMode::Edit), window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-bible",
            "Bible",
            "",
            bible_active,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SelectLeftSection(SidebarTab::Bible), window, cx);
            }),
        ))
        .child(toolbar_divider())
        .child(tool_btn(
            "nav-media",
            "Media",
            "",
            media_open,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::ToggleMediaBin, window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-looks",
            "Looks",
            "",
            matrix_open,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::ToggleLooksMatrixModal, window, cx);
            }),
        ))
        .child(div().flex_1())
        .child(tool_btn(
            "nav-rec",
            recording_label,
            "REC",
            recording_active,
            recording_active,
            cx.listener(move |w, _, window, cx| {
                // A recording that is being finalised cannot be started
                // again, and a press that does nothing is a press the
                // operator cannot read.
                let msg = match recording_state {
                    crate::recording::RecordingState::Recording => {
                        Message::Recording(crate::ui::recording::Message::Stop)
                    }
                    crate::recording::RecordingState::Finishing => return,
                    crate::recording::RecordingState::Idle => {
                        Message::Recording(crate::ui::recording::Message::Start)
                    }
                };
                w.handle_message(msg, window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-ndi",
            "NDI",
            "NDI LIVE",
            ndi_active,
            ndi_active,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(Message::Ndi(crate::ui::ndi::Message::Toggle), window, cx);
            }),
        ))
        .child(toolbar_divider())
        .child(status_btn(
            "nav-audience",
            "Audience",
            audience_active,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(Message::ToggleOutputWindow, window, cx);
            }),
        ))
        // The audience window can be put fullscreen, which is the usual way
        // it is run in a room.
        .child(tool_btn(
            "nav-fullscreen",
            "Fullscreen",
            "",
            audience_active && w.output.is_fullscreen,
            false,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(Message::OutputFullscreenToggled, window, cx);
            }),
        ))
        .child(status_btn(
            "nav-stage",
            "Stage",
            stage_active,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(Message::ToggleStageDisplay, window, cx);
            }),
        ))
        .child(toolbar_divider())
        .child(tool_btn(
            "nav-help",
            "?",
            "",
            false,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::ToggleShortcutsOverlay, window, cx);
            }),
        ))
        .child(tool_btn(
            "nav-settings",
            "Settings",
            "",
            false,
            false,
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Output(crate::ui::output::Message::SettingsOpen),
                    window,
                    cx,
                );
            }),
        ))
        .child(div())
}

/// One toolbar button: label with optional `live` variant text, active tint,
/// and orange border when active.
fn tool_btn(
    id: &'static str,
    label: &'static str,
    live_label: &'static str,
    active: bool,
    live: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let text = if live && !live_label.is_empty() {
        live_label
    } else {
        label
    };
    let text_color = if live {
        theme::c(theme::LIVE_GREEN)
    } else if active {
        theme::c(theme::TEXT_PRIMARY)
    } else {
        theme::c(theme::TEXT_MUTED)
    };

    div()
        .id(id)
        // The button is observed, so a test can press it: the top bar is how
        // the operator moves between showing and editing, and nothing else
        // observes it.
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .py(px(6.))
        .px(px(10.))
        .rounded(px(4.))
        .text_size(px(11.))
        .text_color(text_color)
        .bg(if live {
            theme::c(theme::LIVE_GREEN).opacity(0.16)
        } else if active {
            theme::c(theme::ACCENT_ORANGE).opacity(0.16)
        } else {
            gpui_kit::transparent_black()
        })
        .border_1()
        .border_color(if live {
            theme::c(theme::LIVE_GREEN)
        } else if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            gpui_kit::transparent_black()
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(on_click)
        .child(text)
}

/// Live-status pill with a colored dot (Audience / Stage).
fn status_btn(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let dot_color = if active {
        theme::c(theme::LIVE_GREEN)
    } else {
        theme::c(theme::TEXT_MUTED)
    };
    div()
        .id(id)
        // The pill is observed, so a test can press it: the audience and
        // stage pills are how the operator opens the screens.
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(5.))
        .px(px(10.))
        .rounded(px(4.))
        .text_size(px(11.))
        .text_color(if active {
            theme::c(theme::TEXT_PRIMARY)
        } else {
            theme::c(theme::TEXT_MUTED)
        })
        .bg(if active {
            theme::c(theme::LIVE_GREEN).opacity(0.16)
        } else {
            theme::c(theme::BG_HOVER).opacity(0.4)
        })
        .border_1()
        .border_color(if active {
            theme::c(theme::LIVE_GREEN)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(on_click)
        .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(dot_color))
        .child(label)
}

/// Thin vertical separator between navbar groups.
fn toolbar_divider() -> impl IntoElement {
    div()
        .w(px(1.))
        .h(px(18.))
        .bg(gpui_kit::white().opacity(0.12))
}
