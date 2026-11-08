//! Stage display messages, the shared stage clock, the elapsed/countdown
//! timer logic, and the speaker stage view.
//!
//! Ported from the iced-era `ui/stage/mod.rs`. The view swaps into the
//! center column when `Message::ToggleStageDisplay` fires; the dedicated
//! stage OS window is opened from the output manager. Incoming remote actions
//! bypass this enum entirely: they arrive as root `Message::TriggerFired`
//! over the trigger channel bridge. The clock tick is live (root
//! `Message::ClockTick`).

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, IntoElement, ObjectFit, Styled, div, px, size};

use crate::ui::main_window::MainWindow;
use crate::ui::theme;

/// Preview text for a slide title: the editable text truncated to
/// `max_chars` characters.
fn slide_preview_text(slide: &crate::domain::Slide, max_chars: usize) -> String {
    let text = slide
        .editable_text(None)
        .map(|(t, _)| t.as_str())
        .unwrap_or_default();
    if text.chars().count() > max_chars {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{truncated}…")
    } else {
        text.to_string()
    }
}

/// Show-mode dock controls: the stage speaker view.
pub(crate) fn show_controls(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    stage_view(w, cx)
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleTimer,
    ResetTimer,
}

/// Dispatch a stage message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::ToggleTimer => {
            toggle_timer(w);
            cx.notify();
        }
        Message::ResetTimer => {
            reset_timer(w);
            cx.notify();
        }
    }
}

/// Advance the stage clock and elapsed timer by one tick.
/// The operator's local time, in seconds since midnight.
///
/// The stage monitor's clock is the local time: a clock built from the
/// epoch is UTC, which is an hour — or seven — out in most of the world,
/// and the speaker is the one reading it.
pub(crate) fn local_clock_secs() -> u64 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    (now.hour() as u64) * 3600 + (now.minute() as u64) * 60 + now.second() as u64
}

pub(crate) fn clock_tick(w: &mut MainWindow) {
    w.presenting.clock_secs = local_clock_secs();
    if w.presenting.timer_running {
        w.presenting.timer_secs = now_epoch().saturating_sub(w.presenting.timer_start_epoch);
    }
}

pub(crate) fn toggle_timer(w: &mut MainWindow) {
    if w.presenting.timer_running {
        stop_timer(w);
    } else {
        start_timer(w);
    }
}

/// Start (or resume) the elapsed timer from the real epoch clock, preserving
/// any accumulated seconds. All timer-start paths — dock buttons and remote
/// `Action::StartTimer` — go through here so the epoch bookkeeping has a
/// single source of truth.
pub(crate) fn start_timer(w: &mut MainWindow) {
    if w.presenting.timer_running {
        return;
    }
    w.presenting.timer_start_epoch = now_epoch().saturating_sub(w.presenting.timer_secs);
    w.presenting.timer_running = true;
}

pub(crate) fn stop_timer(w: &mut MainWindow) {
    w.presenting.timer_running = false;
}

pub(crate) fn reset_timer(w: &mut MainWindow) {
    w.presenting.timer_secs = 0;
    w.presenting.timer_running = false;
    w.presenting.timer_start_epoch = now_epoch();
}

pub(crate) fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ---------------------------------------------------------------------------
// The stage monitor window
// ---------------------------------------------------------------------------

/// Build what the stage monitor shows from the main window's state.
pub(crate) fn stage_content(w: &MainWindow) -> crate::ui::stage_window::StageContent {
    let presentation = w.presenting.presentation.as_ref();
    let index = w.presenting.slide_index;
    let current = presentation.and_then(|pres| pres.slides.get(index));

    crate::ui::stage_window::StageContent {
        frame: w
            .compositor
            .slot
            .latest(STAGE_SCREEN_ID)
            .map(|(_, image)| image),
        next_slide: presentation
            .and_then(|pres| pres.slides.get(index + 1))
            .cloned(),
        notes: current
            .and_then(|slide| slide.notes.clone())
            .unwrap_or_default(),
        title: current
            .map(|slide| slide_preview_text(slide, 120))
            .unwrap_or_default(),
        group: current
            .and_then(|slide| slide.group.clone())
            .unwrap_or_default(),
        number: index + 1,
        // The alert goes through the screen's own overlay, so the matrix
        // decides: a screen whose MESSAGES layer is off does not show the
        // alert, and the stage is a screen like any other.
        live_message: crate::ui::props::active_overlay_for_screen(w, STAGE_SCREEN_ID)
            .live_message
            .map(|message| message.text),
        black: w.output.black_screen,
        reference_width: w.app_config.output.width,
        clock_secs: w.presenting.clock_secs,
        timer_secs: w.presenting.timer_secs,
        timer_running: w.presenting.timer_running,
        countdown_end_epoch: w.presenting.countdown_end_epoch,
        revision: 0,
    }
}

/// Publish what the stage monitor shows.
///
/// The monitor is a separate window, so it reads this shared slot rather than
/// the main window's state. Publishing is skipped when no monitor is open:
/// building the content copies the next slide, which is wasted work for a
/// window nobody is looking at.
pub(crate) fn publish_stage_if_open(w: &MainWindow) {
    if stage_monitor_is_open(w) {
        publish_stage(w);
    }
}

/// Whether a stage monitor is open.
///
/// The stage content carries the stage screen's frame, so the content is
/// published whenever *a* monitor is open: a screen routed to the stage (or
/// mirroring it) opens a monitor of its own, and it would show an empty
/// picture while the built-in stage was closed.
pub(crate) fn stage_monitor_is_open(w: &MainWindow) -> bool {
    let routes = w.output.route_slot.read();
    w.output.windows.keys().any(|id| {
        routes
            .get(id)
            .map(|route| route.resolve(&routes) == crate::output::ScreenKind::Stage)
            .unwrap_or(false)
    })
}

/// Publish what the stage monitor shows.
pub(crate) fn publish_stage(w: &MainWindow) {
    w.presenting.stage_slot.publish(stage_content(w));
}

/// Open or close the dedicated stage monitor window.
///
/// The stage monitor is an entry in the output manager like any other
/// screen, so the same machinery opens and closes its window.
pub(crate) fn toggle_stage_window(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let active = w
        .output
        .manager
        .get(STAGE_SCREEN_ID)
        .map(|output| output.active)
        .unwrap_or(false);
    w.output.manager.set_active(STAGE_SCREEN_ID, !active);
    crate::ui::output::sync_output_windows(w, cx);
    cx.notify();
}

/// The id of the stage monitor screen in the output manager.
pub(crate) const STAGE_SCREEN_ID: &str = "stage";

/// Remaining seconds until the armed countdown target, or `None` if no
/// countdown is set. `Some(0)` means the target has been reached.
pub(crate) fn countdown_remaining_secs(countdown_end_epoch: Option<u64>) -> Option<u64> {
    countdown_end_epoch.map(|end| end.saturating_sub(now_epoch()))
}

/// Extend the countdown by `secs`; arms it if it is not running yet, counting
/// from now. Adding time to an expired countdown restarts it from now.
pub(crate) fn countdown_add(w: &mut MainWindow, secs: u64) {
    let now = now_epoch();
    let base = w
        .presenting
        .countdown_end_epoch
        .map_or(now, |end| end.max(now));
    w.presenting.countdown_end_epoch = Some(base + secs);
}

pub(crate) fn countdown_clear(w: &mut MainWindow) {
    w.presenting.countdown_end_epoch = None;
}

// ---------------------------------------------------------------------------
// View (the in-app stage display swap)
// ---------------------------------------------------------------------------

/// Clock/timer values shown in the stage header, grouped so the view
/// functions stay under the argument-count lint.
#[derive(Debug, Clone, Copy)]
pub struct StageClock {
    pub clock_secs: u64,
    pub timer_secs: u64,
    pub timer_running: bool,
    pub countdown_end_epoch: Option<u64>,
}

impl StageClock {
    fn from_state(presenting: &crate::ui::state::PresentingState) -> Self {
        Self {
            clock_secs: presenting.clock_secs,
            timer_secs: presenting.timer_secs,
            timer_running: presenting.timer_running,
            countdown_end_epoch: presenting.countdown_end_epoch,
        }
    }
}

/// Render the stage display (speaker view) for the currently presenting
/// presentation.
///
/// This is the in-app swap; the dedicated OS window has its own chrome in
/// `stage_window.rs`.
pub(crate) fn stage_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let Some(ref pres) = w.presenting.presentation.clone() else {
        return div().flex_1();
    };
    // Whether the dedicated stage window is open: the toolbar lights its
    // button up.
    let stage_window_open = w
        .output
        .manager
        .get(STAGE_SCREEN_ID)
        .map(|output| output.active)
        .unwrap_or(false);
    let clock = StageClock::from_state(&w.presenting);
    // The alert goes through the screen's own overlay, so the matrix
    // decides: a screen whose MESSAGES layer is off does not show the
    // alert, and the stage is a screen like any other.
    let live_message = crate::ui::props::active_overlay_for_screen(w, STAGE_SCREEN_ID)
        .live_message
        .map(|message| message.text);

    let top_bar = stage_toolbar(w, clock, stage_window_open, cx);

    if pres.slides.is_empty() {
        return div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(theme::c(theme::BG_DARKEST))
            .child(top_bar)
            .child(
                div().flex_1().flex().items_center().justify_center().child(
                    div()
                        .text_size(px(18.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("No slides in this presentation"),
                ),
            );
    }

    let idx = w
        .presenting
        .slide_index
        .min(pres.slides.len().saturating_sub(1));
    let current = &pres.slides[idx];
    let current_title = slide_preview_text(current, 120);
    let current_group = current.group.clone().unwrap_or_else(|| String::from("—"));
    let notes_text = current.notes.clone().unwrap_or_default();

    let next_slide = pres.slides.get(idx + 1);
    let next_group = next_slide.and_then(|slide| slide.group.clone());

    // The live slide is shown as the composited frame, so the speaker sees
    // exactly what the audience sees. The next slide is rendered with the
    // native canvas, which is crisp at thumbnail size.
    let live_frame = crate::ui::compositor::frame_image(w);
    let next_slide_cloned = next_slide.cloned();
    let reference_width = w.app_config.output.width as f32;

    let mut body = div()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .flex_grow(3.)
                .min_h_0()
                .child(current_preview(
                    live_frame,
                    &current_title,
                    idx + 1,
                    &current_group,
                    w.output.black_screen,
                ))
                .child(next_preview_col(
                    next_slide_cloned,
                    next_group,
                    reference_width,
                    cx,
                )),
        )
        .child(notes_section(&notes_text));

    if let Some(msg) = live_message {
        body = div()
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(alert_banner(&msg))
            .child(body);
    }

    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARKEST))
        .child(top_bar)
        .child(body)
}

/// The stage toolbar: back-to-Show button, title, optional Stage Window
/// toggle, clock, countdown, and the elapsed-timer controls.
fn stage_toolbar(
    _w: &mut MainWindow,
    clock: StageClock,
    stage_window_open: bool,
    cx: &mut Context<MainWindow>,
) -> Div {
    let clock_secs = clock.clock_secs;
    let hh = clock_secs / 3600;
    let mm = (clock_secs % 3600) / 60;
    let ss = clock_secs % 60;

    let th = clock.timer_secs / 3600;
    let tm = (clock.timer_secs % 3600) / 60;
    let ts = clock.timer_secs % 60;

    let countdown_widget = match countdown_remaining_secs(clock.countdown_end_epoch) {
        Some(remaining) => {
            let cm = remaining / 60;
            let cs = remaining % 60;
            let (color, label) = if remaining == 0 {
                (theme::DANGER_RED, String::from("00:00"))
            } else if remaining < 60 {
                (theme::DANGER_RED, format!("{cm}:{cs:02}"))
            } else {
                (theme::TEXT_PRIMARY, format!("{cm:02}:{cs:02}"))
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(color))
                        .child("COUNTDOWN"),
                )
                .child(
                    div()
                        .text_size(px(18.))
                        .text_color(theme::c(color))
                        .child(label),
                )
                .into_any_element()
        }
        None => div().into_any_element(),
    };

    let leading: gpui_kit::Stateful<Div> =
        stage_toolbar_btn("stage-back-show", "‹ Show", cx, |w, window, cx| {
            w.handle_message(crate::ui::messages::Message::ToggleStageDisplay, window, cx);
        });

    let window_toggle: gpui_kit::AnyElement = stage_toolbar_btn(
        "stage-window-toggle",
        "Stage Window",
        cx,
        |w, window, cx| {
            w.handle_message(crate::ui::messages::Message::ToggleStageWindow, window, cx);
        },
    )
    .when(stage_window_open, |el| {
        el.text_color(theme::c(theme::LIVE_GREEN))
    })
    .into_any_element();

    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(8.))
        .px(px(14.))
        .bg(theme::c(theme::BG_TOOLBAR))
        .border_b_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(leading)
        .child(div().w(px(12.)))
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("STAGE DISPLAY"),
        )
        .child(window_toggle)
        .child(div().flex_1())
        .child(
            div()
                .py(px(4.))
                .px(px(12.))
                .text_size(px(20.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(format!("{hh:02}:{mm:02}:{ss:02}")),
        )
        .child(countdown_widget)
        .child(
            Button::new("stage-timer-toggle")
                .icon(gpui_kit::component::Icon::new(if clock.timer_running {
                    gpui_kit::component::IconName::Pause
                } else {
                    gpui_kit::component::IconName::Play
                }))
                .secondary()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Stage(Message::ToggleTimer),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            div()
                .py(px(4.))
                .px(px(8.))
                .text_size(px(16.))
                .text_color(theme::c(theme::WARNING_AMBER))
                .child(format!("{th:02}:{tm:02}:{ts:02}")),
        )
        .child(
            Button::new("stage-timer-reset")
                .icon(gpui_kit::component::Icon::new(
                    gpui_kit::component::IconName::RefreshCw,
                ))
                .ghost()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Stage(Message::ResetTimer),
                        window,
                        cx,
                    );
                })),
        )
}

/// A small ghost toolbar button with secondary text color.
fn stage_toolbar_btn(
    id: &'static str,
    label: &'static str,
    cx: &mut Context<MainWindow>,
    on_click: impl Fn(&mut MainWindow, &mut gpui_kit::Window, &mut Context<MainWindow>) + 'static,
) -> gpui_kit::Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .py(px(6.))
        .px(px(14.))
        .rounded(px(4.))
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            on_click(w, window, cx);
        }))
        .child(label)
}

/// The live slide area: the composited frame the audience sees, plus the
/// slide's number and group ribbon.
fn current_preview(
    frame: Option<gpui_kit::Img>,
    title: &str,
    index: usize,
    group: &str,
    black: bool,
) -> Div {
    let body = match frame {
        Some(frame) => div()
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(frame.object_fit(ObjectFit::Contain).size_full()),
        None => div()
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(34.))
                    .text_color(theme::c(if black {
                        // The audience is seeing black: the speaker has to
                        // know that, whatever the slide says.
                        theme::WARNING_AMBER
                    } else {
                        theme::TEXT_PRIMARY
                    }))
                    .child(if black {
                        "BLACK".to_string()
                    } else {
                        title.to_string()
                    }),
            ),
    };

    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARKEST))
        .border_r_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(body)
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(px(8.))
                .py(px(6.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(format!("Slide {index} · {group}")),
                ),
        )
}

/// The NEXT column: header strip, a rendered thumbnail of the next slide, and
/// the next slide's group ribbon.
fn next_preview_col(
    next_slide: Option<crate::domain::Slide>,
    next_group: Option<String>,
    reference_width: f32,
    cx: &mut Context<MainWindow>,
) -> Div {
    let preview: Div = match next_slide {
        Some(slide) => {
            // 16:9 thumbnail of the next slide, rendered with the native
            // canvas so the text stays readable.
            let canvas = crate::ui::canvas::slide_element(
                &slide,
                size(px(276.), px(155.)),
                reference_width,
                crate::ui::canvas::CanvasMode::Preview,
                None,
                None,
                None,
                crate::ui::canvas::CanvasGuides::default(),
                cx,
            );
            div()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .bg(theme::c(theme::BG_DARKEST))
                .child(canvas)
        }
        None => div()
            .w_full()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::c(theme::BG_DARKEST))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("— End of presentation —"),
            ),
    };
    div()
        .w(px(300.))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_DARK))
        .border_l_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .w_full()
                .py(px(8.))
                .px(px(12.))
                .bg(theme::c(theme::BG_DARKEST))
                .text_size(px(11.))
                .text_color(theme::c(theme::LIVE_GREEN))
                .child("NEXT"),
        )
        .child(preview)
        .children(
            next_group
                .iter()
                .map(|g| crate::ui::components::group_label_widget(g)),
        )
}

/// Operator notes for the current slide, sized for reading at a distance.
fn notes_section(notes_text: &str) -> Div {
    let is_empty = notes_text.is_empty();
    div()
        .w_full()
        .flex_grow(2.)
        .min_h_0()
        .flex()
        .flex_col()
        .p(px(14.))
        .px(px(18.))
        .bg(theme::c(theme::BG_DARK))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("NOTES"),
        )
        .child(if is_empty {
            div()
                .flex_1()
                .flex()
                .items_start()
                .pt(px(8.))
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("(no notes for this slide)"),
                )
                .into_any_element()
        } else {
            div()
                .id("stage-notes")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .pt(px(8.))
                        .text_size(px(22.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(notes_text.to_string()),
                )
                .into_any_element()
        })
}

/// The amber live-alert banner shown above the stage body.
fn alert_banner(msg: &str) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.))
        .py(px(8.))
        .px(px(16.))
        .bg(theme::c(theme::WARNING_AMBER).opacity(0.2))
        .border_1()
        .border_color(theme::c(theme::WARNING_AMBER))
        .rounded(px(4.))
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::WARNING_AMBER))
                .child("LIVE ALERT"),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme::c(theme::WARNING_AMBER))
                .child(msg.to_string()),
        )
}

#[cfg(test)]
mod tests {
    /// The stage monitor's clock is the operator's local time.
    ///
    /// A clock built from the epoch is UTC, which is an hour — or seven —
    /// out in most of the world, and the speaker is the one reading it.
    #[test]
    fn the_clock_is_the_local_time() {
        use chrono::Timelike;

        let clock = super::local_clock_secs();
        let now = chrono::Local::now();
        let expected =
            (now.hour() as u64) * 3600 + (now.minute() as u64) * 60 + now.second() as u64;

        assert!(clock < 86_400, "the clock is a time of day");
        // Within a second of the local time.
        assert!(
            clock.abs_diff(expected) <= 1,
            "the clock is the local time ({clock} vs {expected})"
        );

        // And it is not the epoch's time of day, which is UTC.
        let utc = chrono::Utc::now();
        let utc_secs =
            (utc.hour() as u64) * 3600 + (utc.minute() as u64) * 60 + utc.second() as u64;
        let offset = chrono::Local::now().offset().local_minus_utc();
        if offset != 0 {
            assert!(
                clock.abs_diff(utc_secs) >= 1,
                "the clock must not be the UTC time of day"
            );
        }
    }
}
