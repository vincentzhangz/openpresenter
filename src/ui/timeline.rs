//! The presentation timeline.
//!
//! The timeline is the show as a piece of time: the slides in the order
//! they play, and how long each one is up. It is the operator's third way
//! of running a show — the first is the slide list, the second is the
//! keyboard, and this one is the clock.
//!
//! A service usually wants the slides to wait for the operator, so a slide
//! with no time of its own waits. A timed show — a countdown, a loop of
//! announcements — gives the slides their time and runs from the timeline.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px, relative};

use crate::domain::Timeline;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// The height of the timeline strip.
const STRIP_HEIGHT: f32 = 34.0;

/// The timeline strip and its transport.
///
/// The strip is the show at a glance: every slide is a block, the one that
/// is live is marked, and the cues are the marks on it.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let Some(presentation) = w.presenting.presentation.clone() else {
        return div();
    };
    let timeline = presentation.timeline.clone();
    let slide_index = w.presenting.slide_index;
    let playing = w.presenting.timeline_playing;
    let elapsed = w.presenting.timeline_elapsed_secs;
    let total = timeline.total_secs(presentation.slides.len());

    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .py(px(5.))
        .bg(theme::c(theme::BG_DARKEST))
        .border_t_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        // The transport.
        .child(
            Button::new("timeline-play")
                .label(if playing { "Pause" } else { "Play" })
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Timeline(crate::ui::timeline::Message::TogglePlay),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new("timeline-stop")
                .label("Stop")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Timeline(crate::ui::timeline::Message::Stop),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("{} / {}", format_time(elapsed), format_time(total))),
        )
        // The strip.
        .child(strip(
            w,
            &presentation,
            &timeline,
            slide_index,
            elapsed,
            total,
            cx,
        ))
}

/// The strip of slides.
fn strip(
    w: &mut MainWindow,
    presentation: &crate::domain::Presentation,
    timeline: &Timeline,
    slide_index: usize,
    elapsed: u64,
    total: u64,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let mut track = div()
        .flex_1()
        .min_w_0()
        .h(px(STRIP_HEIGHT))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(1.))
        .rounded(px(3.))
        .overflow_hidden();

    let slides = &presentation.slides;
    for (index, slide) in slides.iter().enumerate() {
        let live = index == slide_index;
        let seconds = timeline.duration_for(index);
        // A slide with no time of its own still takes a place on the strip,
        // so the operator can see where it is in the show.
        let width = if seconds > 0 && total > 0 {
            (seconds as f32 / total as f32 * 100.0).clamp(2.0, 100.0)
        } else {
            100.0 / slides.len().max(1) as f32
        };
        let group = slide.group.clone().unwrap_or_default();
        let colour = if group.is_empty() {
            theme::c(theme::BORDER_PANEL)
        } else {
            crate::ui::components::group_option_color(&group)
        };
        let cues = slide.cues.len();
        track = track.child(
            div()
                .id(SharedString::from(format!("timeline-slide-{index}")))
                .w(relative(width / 100.0))
                .h(px(STRIP_HEIGHT - 6.))
                .flex()
                .flex_col()
                .justify_center()
                .px(px(4.))
                .rounded(px(2.))
                .bg(if live {
                    colour.opacity(0.85)
                } else {
                    colour.opacity(0.28)
                })
                .border_1()
                .border_color(if live {
                    theme::c(theme::TEXT_PRIMARY).opacity(0.6)
                } else {
                    gpui_kit::transparent_black()
                })
                .hover(|s| s.bg(colour.opacity(0.5)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Slides(crate::ui::slides::Message::SelectSlide(index)),
                        window,
                        cx,
                    );
                }))
                .child(
                    div()
                        .text_size(px(8.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(format!("{}", index + 1)),
                )
                .when(cues > 0, |el| {
                    el.child(
                        div()
                            .text_size(px(7.))
                            .text_color(theme::c(theme::TEXT_PRIMARY))
                            .child(format!("{cues} cues")),
                    )
                }),
        );
    }

    // The playhead: where the show is in its own time.
    let head = if total > 0 {
        (elapsed as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let _ = w;
    div()
        .flex_1()
        .min_w_0()
        .relative()
        .h(px(STRIP_HEIGHT))
        .child(track)
        .child(
            div()
                .absolute()
                .left(relative(head / 100.0))
                .top(px(0.))
                .w(px(2.))
                .h(px(STRIP_HEIGHT))
                .bg(theme::c(theme::ACCENT_ORANGE)),
        )
}

/// A length of time as minutes and seconds.
fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Messages owned by the Timeline feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Start or pause the timeline.
    TogglePlay,
    /// Stop the timeline and go back to the first slide.
    Stop,
    /// The operator set how long a slide is up.
    SetSlideDuration(usize, u64),
    /// The operator turned the loop on or off.
    SetLoop(bool),
    /// The operator set the whole length of the timeline.
    SetDuration(u64),
}

/// Apply a message from the timeline.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::TogglePlay => {
            if w.presenting.timeline_playing {
                w.presenting.timeline_playing = false;
            } else {
                w.presenting.timeline_playing = true;
                // The clock is read from the real epoch, so a pause of the
                // app does not lose the show's place.
                w.presenting.timeline_start_epoch = Some(
                    crate::ui::stage::now_epoch()
                        .saturating_sub(w.presenting.timeline_elapsed_secs),
                );
            }
        }
        Message::Stop => {
            w.presenting.timeline_playing = false;
            w.presenting.timeline_elapsed_secs = 0;
            w.presenting.timeline_start_epoch = Some(crate::ui::stage::now_epoch());
            crate::ui::navigation::select_slide(w, 0, cx);
        }
        Message::SetSlideDuration(index, seconds) => {
            let Some(presentation) = w.presenting.presentation.as_mut() else {
                return;
            };
            presentation.timeline.set_duration(index, seconds);
            let id = presentation.id.clone();
            let timeline = presentation.timeline.clone();
            let _ = w
                .services
                .presentations
                .repo()
                .update_timeline(&id, &timeline);
        }
        Message::SetLoop(looping) => {
            if let Some(presentation) = w.presenting.presentation.as_mut() {
                presentation.timeline.loop_playback = looping;
                let id = presentation.id.clone();
                let timeline = presentation.timeline.clone();
                let _ = w
                    .services
                    .presentations
                    .repo()
                    .update_timeline(&id, &timeline);
            }
        }
        Message::SetDuration(seconds) => {
            if let Some(presentation) = w.presenting.presentation.as_mut() {
                presentation.timeline.duration_secs = seconds;
                let id = presentation.id.clone();
                let timeline = presentation.timeline.clone();
                let _ = w
                    .services
                    .presentations
                    .repo()
                    .update_timeline(&id, &timeline);
            }
        }
    }
    cx.notify();
}

/// Advance the timeline.
///
/// The clock is the real one, so the show keeps its place even when the app
/// is busy. A slide whose time is up hands over to the next one, and the
/// last slide hands over to the first when the timeline loops.
pub(crate) fn tick(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if !w.presenting.timeline_playing {
        return;
    }
    let Some(presentation) = w.presenting.presentation.clone() else {
        return;
    };
    let total = presentation.timeline.total_secs(presentation.slides.len());
    let elapsed = w
        .presenting
        .timeline_start_epoch
        .map(|start| crate::ui::stage::now_epoch().saturating_sub(start))
        .unwrap_or(w.presenting.timeline_elapsed_secs);
    w.presenting.timeline_elapsed_secs = elapsed;

    if total == 0 {
        // A timeline with no length of its own does not advance the show.
        return;
    }
    if elapsed >= total {
        if presentation.timeline.loop_playback {
            w.presenting.timeline_elapsed_secs = 0;
            w.presenting.timeline_start_epoch = Some(crate::ui::stage::now_epoch());
            crate::ui::navigation::select_slide(w, 0, cx);
        } else {
            w.presenting.timeline_playing = false;
        }
        return;
    }

    // The slide whose time is up is the one that goes live.
    let mut at = 0u64;
    for index in 0..presentation.slides.len() {
        let seconds = presentation.timeline.duration_for(index);
        if seconds == 0 {
            // A slide with no time of its own waits for the operator.
            continue;
        }
        at += seconds;
        if elapsed < at {
            if index != w.presenting.slide_index {
                crate::ui::navigation::select_slide(w, index, cx);
            }
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slide's time adds up from the ones before it.
    #[test]
    fn a_slide_starts_when_the_ones_before_it_are_done() {
        let mut timeline = Timeline::default();
        timeline.set_duration(0, 10);
        timeline.set_duration(1, 20);
        assert_eq!(timeline.start_of(0), 0);
        assert_eq!(timeline.start_of(1), 10);
        assert_eq!(timeline.start_of(2), 30);
        assert_eq!(timeline.total_secs(3), 30);
    }

    /// A slide with no time of its own waits for the operator.
    #[test]
    fn a_slide_with_no_time_waits() {
        let mut timeline = Timeline::default();
        timeline.set_duration(0, 10);
        assert_eq!(timeline.duration_for(1), 0);
        assert_eq!(timeline.duration_for(0), 10);
    }

    /// The whole length of the timeline can be set on its own.
    #[test]
    fn the_whole_length_can_be_set() {
        let timeline = Timeline {
            duration_secs: 120,
            ..Timeline::default()
        };
        assert_eq!(timeline.total_secs(4), 120);
    }
}
