use crate::domain::{Presentation, Slide, Transition};
use crate::ui::components::group_label_widget;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::presenter::TransitionState;
use crate::ui::presenter::canvas::{next_slide_canvas_panel, presenter_canvas_panel};
use crate::ui::theme;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Size, Task,
    widget::{Space, button, column, container, row, scrollable, text},
    window,
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::messages::ViewMode;
use std::time::{SystemTime, UNIX_EPOCH};

/// Messages owned by the Stage feature module.
///
/// `ToggleStageDisplay` (global display-mode toggle, emitted from the navbar
/// and Show view) and `ClockTick` (subscription-injected tick) stay as root
/// variants.
#[derive(Debug, Clone)]
pub enum Message {
    ToggleTimer,
    ResetTimer,
}

fn wrap(msg: Message) -> RootMessage {
    RootMessage::Stage(msg)
}

/// Render the stage display for the currently presenting presentation.
pub fn view<'a>(w: &'a MainWindow) -> Element<'a, RootMessage> {
    let Some(ref pres) = w.presenting.presentation else {
        return container(text("")).into();
    };
    stage_view(
        pres,
        w.presenting.slide_index,
        w.presenting.transition.as_ref(),
        StageClock::from_state(&w.presenting),
        false,
        w.presenting.stage_window_id.is_some(),
        w.props
            .manager
            .live_message
            .as_ref()
            .filter(|m| m.visible)
            .map(|m| m.text.as_str()),
    )
}

/// Dispatch a stage message.
pub fn update(w: &mut MainWindow, msg: Message) -> iced::Task<RootMessage> {
    match msg {
        Message::ToggleTimer => toggle_timer(w),
        Message::ResetTimer => reset_timer(w),
    }
}

/// Content of the dedicated stage-display OS window: the same speaker view as
/// the in-app stage display, rendered full-window with its own chrome.
pub fn stage_window_view(w: &MainWindow) -> Element<'_, RootMessage> {
    let Some(ref pres) = w.presenting.presentation else {
        return container(text("")).into();
    };
    stage_view(
        pres,
        w.presenting.slide_index,
        w.presenting.transition.as_ref(),
        StageClock::from_state(&w.presenting),
        true,
        true,
        w.props
            .manager
            .live_message
            .as_ref()
            .filter(|m| m.visible)
            .map(|m| m.text.as_str()),
    )
}

pub(crate) fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Remaining seconds until the armed countdown target, or `None` if no
/// countdown is set. `Some(0)` means the target has been reached.
pub(crate) fn countdown_remaining_secs(countdown_end_epoch: Option<u64>) -> Option<u64> {
    countdown_end_epoch.map(|end| end.saturating_sub(now_epoch()))
}

/// Extend the countdown by `secs`; arms it if it is not running yet, counting
/// from now. Adding time to an expired countdown restarts it from now.
pub fn countdown_add(w: &mut MainWindow, secs: u64) -> iced::Task<RootMessage> {
    let now = now_epoch();
    let base = w
        .presenting
        .countdown_end_epoch
        .map_or(now, |end| end.max(now));
    w.presenting.countdown_end_epoch = Some(base + secs);
    iced::Task::none()
}

pub fn countdown_clear(w: &mut MainWindow) -> iced::Task<RootMessage> {
    w.presenting.countdown_end_epoch = None;
    iced::Task::none()
}

pub fn toggle_stage_display(w: &mut MainWindow) -> iced::Task<RootMessage> {
    if w.shell.current_mode == ViewMode::Show {
        w.presenting.stage_display_active = !w.presenting.stage_display_active;
    }
    iced::Task::none()
}

/// Open (or focus) the dedicated stage-display OS window.
pub fn open_stage_window(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.presenting.stage_window_id {
        return window::gain_focus(id);
    }
    let (id, open_task) = window::open(window::Settings {
        size: Size::new(1280.0, 720.0),
        position: window::Position::Default,
        exit_on_close_request: false,
        ..Default::default()
    });
    w.presenting.stage_window_id = Some(id);
    open_task.map(|_| RootMessage::Noop)
}

pub fn close_stage_window(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(id) = w.presenting.stage_window_id.take() {
        return window::close(id);
    }
    Task::none()
}

/// Toggle the dedicated stage-display window.
pub fn toggle_stage_window(w: &mut MainWindow) -> Task<RootMessage> {
    if w.presenting.stage_window_id.is_some() {
        close_stage_window(w)
    } else {
        open_stage_window(w)
    }
}

pub fn clock_tick(w: &mut MainWindow) -> iced::Task<RootMessage> {
    w.presenting.clock_secs = now_epoch() % 86400;
    if w.presenting.timer_running {
        w.presenting.timer_secs = now_epoch().saturating_sub(w.presenting.timer_start_epoch);
    }
    iced::Task::none()
}

pub fn toggle_timer(w: &mut MainWindow) -> iced::Task<RootMessage> {
    if w.presenting.timer_running {
        stop_timer(w);
    } else {
        start_timer(w);
    }
    iced::Task::none()
}

/// Start (or resume) the elapsed timer from the real epoch clock, preserving
/// any accumulated seconds. All timer-start paths — toolbar, dock buttons, and
/// remote `Action::StartTimer` — go through here so the epoch bookkeeping has a
/// single source of truth.
pub fn start_timer(w: &mut MainWindow) {
    if w.presenting.timer_running {
        return;
    }
    w.presenting.timer_start_epoch = now_epoch().saturating_sub(w.presenting.timer_secs);
    w.presenting.timer_running = true;
}

pub fn stop_timer(w: &mut MainWindow) {
    w.presenting.timer_running = false;
}

pub fn reset_timer(w: &mut MainWindow) -> iced::Task<RootMessage> {
    w.presenting.timer_secs = 0;
    w.presenting.timer_running = false;
    w.presenting.timer_start_epoch = now_epoch();
    iced::Task::none()
}

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

pub fn stage_view<'a>(
    presentation: &'a Presentation,
    slide_index: usize,
    transition: Option<&'a TransitionState>,
    clock: StageClock,
    dedicated_window: bool,
    stage_window_open: bool,
    live_message: Option<&'a str>,
) -> Element<'a, RootMessage> {
    let top_bar = stage_toolbar(clock, dedicated_window, stage_window_open);

    if presentation.slides.is_empty() {
        return container(column![
            top_bar,
            container(
                text("No slides in this presentation")
                    .size(18)
                    .color(theme::TEXT_MUTED),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }

    let idx = slide_index.min(presentation.slides.len().saturating_sub(1));
    let current = &presentation.slides[idx];
    let next_slide = presentation.slides.get(idx + 1);

    let (from_slide, trans_type, trans_progress) = match transition {
        Some(ts) => (Some(&ts.from_slide), ts.transition, ts.progress),
        None => (None, Transition::Cut, 1.0),
    };

    let current_preview = container(presenter_canvas_panel(
        Some(current),
        from_slide,
        trans_type,
        trans_progress,
        None,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::canvas_bg_style);

    let next_col = next_preview_col(next_slide);

    let top_panels = row![current_preview, next_col]
        .width(Length::Fill)
        .height(Length::FillPortion(3));

    let notes_panel = notes_section(current);

    let mut body = column![top_panels, notes_panel]
        .width(Length::Fill)
        .height(Length::Fill);

    if let Some(msg) = live_message {
        let alert_banner = container(
            row![
                fa_icon_solid("bullhorn")
                    .size(16.0_f32)
                    .color(theme::WARNING_AMBER),
                Space::new().width(10),
                text(msg).size(16).color(theme::WARNING_AMBER),
            ]
            .align_y(Alignment::Center)
            .padding([8, 16]),
        )
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba(0.95, 0.65, 0.1, 0.2))),
            border: Border {
                color: theme::WARNING_AMBER,
                width: 1.5,
                radius: 4.0.into(),
            },
            ..Default::default()
        });

        body = column![alert_banner, body].spacing(6);
    }

    container(column![top_bar, body])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn stage_toolbar(
    clock: StageClock,
    dedicated_window: bool,
    stage_window_open: bool,
) -> Element<'static, RootMessage> {
    let clock_secs = clock.clock_secs;
    let timer_secs = clock.timer_secs;
    let hh = clock_secs / 3600;
    let mm = (clock_secs % 3600) / 60;
    let ss = clock_secs % 60;
    let clock_str = format!("{hh:02}:{mm:02}:{ss:02}");

    let th = timer_secs / 3600;
    let tm = (timer_secs % 3600) / 60;
    let ts_val = timer_secs % 60;
    let timer_str = format!("{th:02}:{tm:02}:{ts_val:02}");

    let timer_icon: Element<'_, RootMessage> = if clock.timer_running {
        fa_icon_solid("pause").size(13.0_f32).into()
    } else {
        fa_icon_solid("play").size(13.0_f32).into()
    };

    let countdown_widget: Element<'_, RootMessage> =
        match countdown_remaining_secs(clock.countdown_end_epoch) {
            Some(remaining) => {
                let cm = remaining / 60;
                let cs = remaining % 60;
                let (countdown_color, countdown_label) = if remaining == 0 {
                    (theme::DANGER_RED, "00:00".to_string())
                } else if remaining < 60 {
                    (theme::DANGER_RED, format!("{cm}:{cs:02}"))
                } else {
                    (theme::TEXT_PRIMARY, format!("{cm:02}:{cs:02}"))
                };
                row![
                    fa_icon_solid("hourglass-half")
                        .size(13.0_f32)
                        .color(countdown_color),
                    text(countdown_label).size(18).color(countdown_color),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            }
            None => Space::new().width(0).into(),
        };

    // In the dedicated OS window the in-app "Show" swap is meaningless; show a
    // window toggle instead so the speaker screen stays speaker-focused.
    let leading: Element<'static, RootMessage> = if dedicated_window {
        button(
            row![
                fa_icon_solid("up-right-from-square")
                    .size(12.0_f32)
                    .color(theme::TEXT_SECONDARY),
                text(" Close Window").size(12).color(theme::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(RootMessage::ToggleStageWindow)
        .padding([6, 14])
        .style(theme::ghost_button)
        .into()
    } else {
        button(
            row![
                fa_icon_solid("arrow-left")
                    .size(12.0_f32)
                    .color(theme::TEXT_SECONDARY),
                text(" Show").size(12).color(theme::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(RootMessage::ToggleStageDisplay)
        .padding([6, 14])
        .style(theme::ghost_button)
        .into()
    };

    // In the in-app toolbar offer a one-click "pop out to its own window";
    // the dedicated window already has its Close Window button.
    let window_toggle: Element<'static, RootMessage> = if dedicated_window {
        Space::new().width(0).into()
    } else {
        button(
            row![
                fa_icon_solid("up-right-from-square")
                    .size(12.0_f32)
                    .color(if stage_window_open {
                        theme::LIVE_GREEN
                    } else {
                        theme::TEXT_SECONDARY
                    }),
                text(" Stage Window").size(12).color(theme::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(RootMessage::ToggleStageWindow)
        .padding([6, 14])
        .style(theme::ghost_button)
        .into()
    };

    container(
        row![
            leading,
            Space::new().width(12),
            text("STAGE DISPLAY").size(13).color(theme::TEXT_MUTED),
            window_toggle,
            Space::new().width(Length::Fill),
            container(text(clock_str).size(20).color(theme::TEXT_PRIMARY)).padding([4, 12]),
            countdown_widget,
            button(timer_icon)
                .on_press(wrap(Message::ToggleTimer))
                .padding([6, 10])
                .style(theme::secondary_button),
            container(text(timer_str).size(16).color(theme::WARNING_AMBER)).padding([4, 8]),
            button(fa_icon_solid("rotate-right").size(13.0_f32))
                .on_press(wrap(Message::ResetTimer))
                .padding([6, 10])
                .style(theme::ghost_button),
        ]
        .spacing(6)
        .padding([8, 14])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::toolbar_style)
    .into()
}

fn next_preview_col<'a>(next: Option<&'a Slide>) -> Element<'a, RootMessage> {
    let header = container(text("NEXT").size(11).color(theme::LIVE_GREEN))
        .padding([8, 12])
        .width(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::BG_DARKEST)),
            ..Default::default()
        });

    let preview: Element<RootMessage> = match next {
        Some(slide) => container(next_slide_canvas_panel(Some(slide)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::canvas_bg_style)
            .into(),
        None => container(
            text("— End of presentation —")
                .size(13)
                .color(theme::TEXT_MUTED),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into(),
    };

    let group: Element<RootMessage> = match next.and_then(|s| s.group.as_deref()) {
        Some(lbl) => group_label_widget(lbl),
        None => Space::new().height(0).into(),
    };

    container(
        column![header, preview, group]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(300)
    .height(Length::Fill)
    .style(theme::dark_panel_style)
    .into()
}

fn notes_section<'a>(slide: &'a Slide) -> Element<'a, RootMessage> {
    let notes_text = slide.notes.as_deref().unwrap_or("");
    let is_empty = notes_text.is_empty();

    let inner: Element<RootMessage> = if is_empty {
        text("(no notes for this slide)")
            .size(14)
            .color(theme::TEXT_MUTED)
            .into()
    } else {
        scrollable(
            text(notes_text)
                .size(22)
                .color(theme::TEXT_PRIMARY)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    };

    container(
        column![
            container(row![text("NOTES").size(10).color(theme::TEXT_MUTED),].padding([0, 0]),),
            inner,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([14, 18]),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(2))
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(theme::BG_DARK)),
        border: Border {
            color: theme::BORDER_PANEL,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}
