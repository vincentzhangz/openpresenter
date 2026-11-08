//! NDI output control.
//!
//! Ported from the iced-era `ui/ndi/mod.rs` + `ui/output/mod.rs` NDI helpers:
//! starting/stopping the NDI output loop with the current slide, re-sending
//! the live slide, and blacking the NDI output. The root `ToggleNdi` /
//! `NdiSendCurrent` / `NdiBlackScreen` variants route here too. The video
//! frame source wiring lives in `ui/video.rs` (`set_video_frame_source`).

use gpui_kit::Context;

use crate::ndi::{FrameRate, NdiOutputLoop};
use crate::output::OutputType;
use crate::ui::main_window::MainWindow;

#[derive(Debug, Clone)]
pub enum Message {
    /// Start (or stop) the NDI output loop.
    Toggle,
}

/// Dispatch an NDI message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Toggle => toggle_ndi(w, cx),
    }
}

/// Start the NDI output loop seeded with the current slide, or stop it when
/// one is already running. Without the NDI SDK runtime the send path is a
/// no-op, so it is never started: the operator must not believe the program
/// is being distributed while nothing leaves the machine.
pub(crate) fn toggle_ndi(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The toggle drives the broadcast output: the first NDI output in the
    // manager.
    let id = crate::ui::props::ndi_screen_id(w);
    if w.presenting.ndi_outputs.contains_key(&id) {
        // The manager has to know that the broadcast is off, or the next
        // sync would stop a sender that is not running — and start one that
        // is.
        w.output.manager.set_active(&id, false);
        stop_ndi_output(w, &id);
        // A second broadcast would keep running, and the badge would go on
        // saying BROADCASTING: the operator pressed stop, so the broadcast
        // stops.
        stop_every_ndi_output(w);
        crate::ui::output::persist_config(w);
        cx.notify();
        return;
    }
    // The manager has to know as well: a sender the manager does not know
    // about is stopped by the next sync, so the broadcast would blink out
    // the moment the operator opened the stage monitor.
    w.output.manager.set_active(&id, true);
    w.presenting.ndi_blacked = false;
    if !start_ndi_output(w, &id, cx) {
        // The broadcast did not start, so the manager must not claim that it
        // is running.
        w.output.manager.set_active(&id, false);
    }
    crate::ui::output::persist_config(w);
    cx.notify();
}

/// Stop the sender of one NDI output.
pub(crate) fn stop_ndi_output(w: &mut MainWindow, id: &str) {
    if let Some(ndi) = w.presenting.ndi_outputs.remove(id) {
        ndi.stop();
    }
}

/// Stop every running broadcast.
///
/// The operator pressed stop, so every stream stops: a second broadcast that
/// kept running would leave the badge saying BROADCASTING.
pub(crate) fn stop_every_ndi_output(w: &mut MainWindow) {
    let ids: Vec<String> = w.presenting.ndi_outputs.keys().cloned().collect();
    for id in ids {
        w.output.manager.set_active(&id, false);
        stop_ndi_output(w, &id);
    }
}

/// Start the sender of one NDI output.
///
/// The sender is bound to the output's screen, so it shows that screen's
/// look: the broadcast can carry different props than the projector.
pub(crate) fn start_ndi_output(
    w: &mut MainWindow,
    id: &str,
    _cx: &mut Context<MainWindow>,
) -> bool {
    if !crate::ndi::is_available() {
        w.set_error(
            "NDI output unavailable: install the NDI SDK runtime and use an \
             NDI-enabled build of OpenPresenter"
                .to_string(),
        );
        return false;
    }
    if !w.app_config.ndi.enabled {
        // The setting says the machine is not allowed to send video, so the
        // broadcast does not start: a setting that is stored and never read
        // is a setting that suggests the opposite of what happens.
        w.set_error("The broadcast is switched off in the settings.".to_string());
        return false;
    }
    // The name the receiver sees is the output's own stream name: two
    // broadcasts have to be told apart, and the row shows this name. The
    // settings' "Source Name" is the fallback, and the historical default
    // the last one.
    let source_name = {
        let own = w
            .output
            .manager
            .get(id)
            .and_then(|output| match &output.output_type {
                OutputType::Ndi { stream_name } => Some(stream_name.trim().to_string()),
                _ => None,
            })
            .filter(|name| !name.is_empty());
        own.or_else(|| {
            let configured = w.app_config.ndi.source_name.trim().to_string();
            (!configured.is_empty()).then_some(configured)
        })
        .unwrap_or_else(|| String::from("OpenPresenter"))
    }; // The configured frame rate seeds the loop (30/60 selectable in the
    // settings Network tab).
    let frame_rate = if w.app_config.ndi.frame_rate >= 60 {
        FrameRate::FPS_60
    } else {
        FrameRate::FPS_30
    };
    // The broadcast publishes into the shared frame slot under its own
    // screen, so the operator's preview can show the stream.
    let publish = Some((id.to_string(), std::sync::Arc::clone(&w.compositor.slot)));
    // The broadcast follows the resolution of its own output, so what
    // leaves the machine matches the projector it is paired with.
    let (width, height) = crate::ui::compositor::screen_resolution(w, id);
    match NdiOutputLoop::start(&source_name, width, height, frame_rate, publish) {
        Ok(ndi) => {
            let slide = w
                .presenting
                .presentation
                .as_ref()
                .and_then(|p| p.slides.get(w.presenting.slide_index))
                .cloned();
            if let Some(s) = slide {
                ndi.set_slide(s);
            }
            // The sender composites the overlay itself, so it needs the
            // current props, mask and live message as well.
            ndi.set_overlay(crate::ui::props::active_overlay_for_screen(w, id));
            ndi.set_style(crate::ui::props::style_for_screen(w, id));
            // A video that is already playing has to reach the new sender,
            // otherwise the broadcast would show no video at all.
            if let Some(ref player) = w.video.player {
                ndi.set_video_frame_source(Some(player.video.shared_frame_arc()));
            }
            w.presenting.ndi_outputs.insert(id.to_string(), ndi);
            true
        }
        Err(e) => {
            w.set_error(format!("Failed to start NDI: {e}"));
            false
        }
    }
}

/// Push the current live slide to the NDI output, if one is running.
pub(crate) fn ndi_send_current(w: &mut MainWindow, _cx: &mut Context<MainWindow>) {
    let Some(ref pres) = w.presenting.presentation else {
        return;
    };
    let Some(slide) = pres.slides.get(w.presenting.slide_index) else {
        return;
    };
    // The output is black, so the broadcast stays black as well: a stream
    // that came back on while the projector stayed black would show the
    // two surfaces different pictures.
    if w.output.black_screen {
        for ndi in w.presenting.ndi_outputs.values() {
            ndi.black_screen();
        }
        return;
    }
    // Every sender shows the same slide, but each with its own look.
    for ndi in w.presenting.ndi_outputs.values() {
        ndi.set_slide(slide.clone());
    }
}

/// Black the NDI output, if one is running.
pub(crate) fn ndi_black_screen(w: &mut MainWindow, _cx: &mut Context<MainWindow>) {
    // The broadcast is blacked: the panel's "Live" switch has to show it, so
    // the operator can see why the stream is black.
    w.presenting.ndi_blacked = true;
    for ndi in w.presenting.ndi_outputs.values() {
        ndi.black_screen();
    }
}

/// Root-message alias for [`ndi_send_current`].
pub(crate) fn send_current(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    ndi_send_current(w, cx);
}

/// Root-message alias for [`ndi_black_screen`].
pub(crate) fn black_screen(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    ndi_black_screen(w, cx);
}

/// Black or restore the broadcast output.
///
/// The sender composites the slide itself, so the black screen has to be
/// mirrored to it — otherwise a remote black hides the projector only.
pub(crate) fn set_black_screen(w: &mut MainWindow, on: bool, cx: &mut Context<MainWindow>) {
    if on {
        ndi_black_screen(w, cx);
    } else {
        w.presenting.ndi_blacked = false;
        ndi_send_current(w, cx);
    }
}

/// Black the broadcast, or bring it back when it is blacked.
///
/// The dock's "Live" switch is a toggle, and the state it shows has to be
/// the state the broadcast is in: a switch that stays dark after the stream
/// came back is a switch the operator cannot trust, and pressing it again
/// would only black the stream once more.
pub(crate) fn toggle_ndi_black_screen(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let blacked = w.presenting.ndi_blacked;
    set_black_screen(w, !blacked, cx);
    cx.notify();
}
