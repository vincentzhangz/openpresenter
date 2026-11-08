//! The dedicated stage-monitor window.
//!
//! A second OS window for the speaker's screen: it shows the live slide,
//! the next slide, the speaker's notes, the clock and the elapsed timer. In
//! a real setup the audience output goes to the projector and the stage
//! monitor faces the speaker, so the two windows are usually on different
//! displays.
//!
//! The window is a separate GPUI entity, so it cannot read the main
//! window's state directly. The main window publishes what the monitor shows
//! into a shared slot, and the window reads it from there.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyWindowHandle, App, Context, Entity, IntoElement, ObjectFit, RenderImage, Window, div, img,
    px, size,
};

use crate::domain::Slide;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::theme;

/// What the stage monitor shows.
///
/// The main window publishes this into the shared slot whenever the show
/// moves on.
#[derive(Clone, Default)]
pub(crate) struct StageContent {
    /// The composited live frame: what the audience sees.
    pub(crate) frame: Option<Arc<RenderImage>>,
    /// The slide after the current one.
    pub(crate) next_slide: Option<Slide>,
    /// The current slide's speaker notes.
    pub(crate) notes: String,
    /// The current slide's title.
    pub(crate) title: String,
    /// The current slide's group label.
    pub(crate) group: String,
    /// The current slide's number, one-based.
    pub(crate) number: usize,
    /// The live alert message.
    pub(crate) live_message: Option<String>,
    /// The resolution the slides were authored against.
    ///
    /// The next-slide thumbnail is scaled against it, so the preview on the
    /// monitor matches the one in the app.
    pub(crate) reference_width: u32,
    /// Whether the output is blacked out.
    ///
    /// The speaker has to know whether the audience is seeing black, so the
    /// monitor says so instead of leaving an empty picture area.
    pub(crate) black: bool,
    /// The wall clock, in seconds since midnight.
    pub(crate) clock_secs: u64,
    /// The elapsed timer, in seconds.
    pub(crate) timer_secs: u64,
    /// Whether the elapsed timer runs.
    pub(crate) timer_running: bool,
    /// The countdown's target time.
    pub(crate) countdown_end_epoch: Option<u64>,
    /// Bumped on every publish so the window knows to repaint.
    pub(crate) revision: u64,
}

/// The shared slot the stage window reads.
#[derive(Default)]
pub(crate) struct StageSlot {
    content: Mutex<StageContent>,
}

impl StageSlot {
    /// Publish new content.
    pub(crate) fn publish(&self, content: StageContent) {
        if let Ok(mut guard) = self.content.lock() {
            let revision = guard.revision.wrapping_add(1);
            *guard = StageContent {
                revision,
                ..content
            };
        }
    }

    /// The current content.
    pub(crate) fn read(&self) -> StageContent {
        self.content
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

/// The stage monitor window's root entity.
pub struct StageWindow {
    /// The shared slot the main window publishes into.
    slot: Arc<StageSlot>,
    /// The main window, so the monitor's buttons can drive the show.
    main: Entity<MainWindow>,
    /// The revision currently on screen.
    shown: u64,
}

impl StageWindow {
    /// Create the window entity and start watching the slot.
    pub(crate) fn new(
        slot: Arc<StageSlot>,
        main: Entity<MainWindow>,
        cx: &mut Context<Self>,
    ) -> Self {
        let window = Self {
            slot,
            main,
            shown: 0,
        };

        // The main window publishes; the monitor watches for a new revision.
        // The poller holds the window weakly: a closed window must not leave
        // a timer behind that watches the slot for ever.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let Some(entity) = this.upgrade() else {
                    // The window was closed: the poller stops with it.
                    break;
                };
                let stale = cx.read_entity(&entity, |w, _| w.slot.read().revision != w.shown);
                if stale {
                    entity.update(cx, |_w, cx| cx.notify());
                }
            }
        })
        .detach();

        window
    }
}

impl Render for StageWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.slot.read();
        self.shown = content.revision;

        let clock = content.clock_secs;
        let hh = clock / 3600;
        let mm = (clock % 3600) / 60;
        let ss = clock % 60;
        let th = content.timer_secs / 3600;
        let tm = (content.timer_secs % 3600) / 60;
        let ts = content.timer_secs % 60;

        let countdown = content
            .countdown_end_epoch
            .map(|end| end.saturating_sub(crate::ui::stage::now_epoch()))
            .map(|remaining| {
                let cm = remaining / 60;
                let cs = remaining % 60;
                let color = if remaining < 60 {
                    theme::DANGER_RED
                } else {
                    theme::TEXT_PRIMARY
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
                            .child(format!("{cm:02}:{cs:02}")),
                    )
            });

        let main = self.main.clone();
        let toolbar = div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .py(px(8.))
            .px(px(14.))
            .bg(theme::c(theme::BG_TOOLBAR))
            .border_b_1()
            .border_color(theme::c(theme::BORDER_PANEL))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("STAGE"),
            )
            .child(
                Button::new("stage-window-prev")
                    .icon(Icon::new(gpui_kit::component::IconName::ChevronLeft))
                    .secondary()
                    .small()
                    .on_click(drive(main.clone(), Message::PresentingPrevSlide)),
            )
            .child(
                Button::new("stage-window-next")
                    .icon(Icon::new(gpui_kit::component::IconName::ChevronRight))
                    .secondary()
                    .small()
                    .on_click(drive(main.clone(), Message::PresentingNextSlide)),
            )
            .child(
                Button::new("stage-window-black")
                    // The speaker has to know whether the audience is
                    // seeing black, so the button says which way it goes.
                    .label(if content.black { "Unblack" } else { "Black" })
                    .ghost()
                    .small()
                    .when(content.black, |b| b.secondary())
                    .on_click(drive(main.clone(), Message::ToggleOutputBlackScreen)),
            )
            // The elapsed timer belongs to the speaker, so the monitor can
            // start, stop and reset it — a timer it can only read is a
            // timer it cannot use.
            .child(
                Button::new("stage-window-timer")
                    .label(if content.timer_running {
                        "Pause"
                    } else {
                        "Start"
                    })
                    .ghost()
                    .small()
                    .on_click(drive(
                        main.clone(),
                        if content.timer_running {
                            Message::StopTimer
                        } else {
                            Message::StartTimer
                        },
                    )),
            )
            .child(
                Button::new("stage-window-timer-reset")
                    .label("Reset")
                    .ghost()
                    .small()
                    .on_click(drive(main.clone(), Message::ResetTimer)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child(format!(
                        "{} · {}",
                        content.number,
                        if content.group.is_empty() {
                            "—"
                        } else {
                            &content.group
                        }
                    )),
            )
            .child(div().flex_1())
            .child(
                div()
                    .py(px(4.))
                    .px(px(12.))
                    .text_size(px(20.))
                    .text_color(theme::c(theme::TEXT_PRIMARY))
                    .child(format!("{hh:02}:{mm:02}:{ss:02}")),
            )
            .when_some(countdown, |el, countdown| el.child(countdown))
            .child(
                div()
                    .py(px(4.))
                    .px(px(8.))
                    .text_size(px(16.))
                    .text_color(theme::c(if content.timer_running {
                        theme::LIVE_GREEN
                    } else {
                        theme::WARNING_AMBER
                    }))
                    .child(format!("{th:02}:{tm:02}:{ts:02}")),
            );

        let live = content
            .frame
            .clone()
            .map(|frame| {
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .child(img(frame).object_fit(ObjectFit::Contain).size_full())
            })
            .unwrap_or_else(|| {
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(30.))
                            .text_color(theme::c(if content.black {
                                // The audience is seeing black: the speaker
                                // has to know that, whatever the slide says.
                                theme::WARNING_AMBER
                            } else {
                                theme::TEXT_PRIMARY
                            }))
                            .child(if content.black {
                                "BLACK".to_string()
                            } else {
                                content.title.clone()
                            }),
                    )
            });

        let next = match content.next_slide.clone() {
            Some(slide) => div()
                .w(px(280.))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .bg(theme::c(theme::BG_DARK))
                .border_l_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .child(
                    div()
                        .w_full()
                        .py(px(6.))
                        .px(px(10.))
                        .bg(theme::c(theme::BG_DARKEST))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::LIVE_GREEN))
                        .child("NEXT"),
                )
                .child(
                    div()
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .bg(theme::c(theme::BG_DARKEST))
                        .child(crate::ui::canvas::slide_preview(
                            &slide,
                            size(px(260.), px(146.)),
                            content.reference_width as f32,
                            None,
                        )),
                ),
            None => div()
                .w(px(280.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme::c(theme::BG_DARK))
                .border_l_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("— End of presentation —"),
                ),
        };

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
                    .child(live)
                    .child(next),
            )
            .child(
                div()
                    .id("stage-window-notes")
                    .w_full()
                    .max_h(px(220.))
                    .overflow_y_scroll()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(theme::c(theme::BORDER_PANEL))
                    .bg(theme::c(theme::BG_DARK))
                    .p(px(12.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child("NOTES"),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .text_size(px(15.))
                            .text_color(theme::c(theme::TEXT_PRIMARY))
                            .child(if content.notes.trim().is_empty() {
                                String::from("No notes for this slide")
                            } else {
                                content.notes.clone()
                            }),
                    ),
            );

        if let Some(message) = content.live_message.clone() {
            body = div()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .w_full()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(theme::c(theme::DANGER_RED))
                        .text_size(px(13.))
                        .text_color(gpui_kit::white())
                        .child(message),
                )
                .child(body);
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::c(theme::BG_DARKEST))
            .child(toolbar)
            .child(body)
    }
}

/// A click handler that drives the show from the stage monitor.
fn drive(
    main: Entity<MainWindow>,
    message: Message,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static {
    move |_event, window, cx| {
        main.update(cx, |w, cx| w.handle_message(message.clone(), window, cx));
    }
}

/// Open the stage monitor window.
pub(crate) fn open(
    slot: Arc<StageSlot>,
    main: Entity<MainWindow>,
    bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
    cx: &mut App,
) -> anyhow::Result<AnyWindowHandle> {
    let options = gpui_kit::WindowOptions {
        window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("Stage Monitor".into()),
            ..Default::default()
        }),
        focus: false,
        show: true,
        kind: gpui_kit::WindowKind::Normal,
        window_background: gpui_kit::WindowBackgroundAppearance::Opaque,
        ..Default::default()
    };

    let (handle, _entity) = gpui_kit::open_window(options, cx, move |_window, cx| {
        cx.new(|cx| StageWindow::new(Arc::clone(&slot), main.clone(), cx))
    })?;
    Ok(handle)
}
