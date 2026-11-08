//! The service calendar.
//!
//! The calendar is the church's week: the services that happen, and the
//! playlist that plays at each one. The operator looks at it before a
//! service to see what is coming, and during one to run it and to record it.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::domain::calendar::{Repeat, ServiceEvent, Weekday, local_time, now, time_until};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// The two lists the operator looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarTab {
    /// What is coming.
    #[default]
    Upcoming,
    /// What has been.
    Past,
}

impl CalendarTab {
    /// The name the tab is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Upcoming => "Upcoming",
            Self::Past => "Past",
        }
    }

    /// Every tab.
    pub const ALL: [CalendarTab; 2] = [CalendarTab::Upcoming, CalendarTab::Past];
}

/// The state the calendar panel holds.
#[derive(Debug, Clone, Default)]
pub struct CalendarState {
    /// Whether the panel is on the screen.
    pub open: bool,
    /// The list the operator is looking at.
    pub tab: CalendarTab,
    /// The name of the service the operator is adding.
    pub new_name: String,
    /// The days of the week the service repeats on.
    pub repeat_days: Vec<Weekday>,
    /// Whether the service is recorded.
    pub capture: bool,
    /// The playlist that plays at the service.
    pub playlist_id: Option<String>,
    /// The services on the calendar, as they were last read.
    pub events: Vec<ServiceEvent>,
}

impl CalendarState {
    /// The state the app starts with.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Messages owned by the Calendar feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the calendar.
    Open,
    /// Close the calendar.
    Close,
    /// The operator picked the list they are looking at.
    SelectTab(CalendarTab),
    /// The operator typed the name of the service they are adding.
    NameChanged(String),
    /// The operator turned a repeat day on or off.
    ToggleRepeatDay(Weekday),
    /// The operator turned the capture on or off.
    ToggleCapture(bool),
    /// The operator picked the playlist that plays at the service.
    SetPlaylist(Option<String>),
    /// The operator added the service.
    Add,
    /// The operator took a service off the calendar.
    Delete(String),
    /// The operator is running the service.
    Start(String),
    /// The operator is recording the service.
    Capture(String),
}

/// Apply a message from the calendar.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Open => {
            w.calendar.open = true;
            reload(w);
        }
        Message::Close => {
            w.calendar.open = false;
        }
        Message::SelectTab(tab) => {
            w.calendar.tab = tab;
        }
        Message::NameChanged(name) => {
            w.calendar.new_name = name;
        }
        Message::ToggleRepeatDay(day) => {
            if let Some(position) = w.calendar.repeat_days.iter().position(|d| *d == day) {
                w.calendar.repeat_days.remove(position);
            } else {
                w.calendar.repeat_days.push(day);
                w.calendar.repeat_days.sort_by_key(|d| d.days_from_monday());
            }
        }
        Message::ToggleCapture(capture) => {
            w.calendar.capture = capture;
        }
        Message::SetPlaylist(id) => {
            w.calendar.playlist_id = id;
        }
        Message::Add => {
            add_event(w, cx);
        }
        Message::Delete(id) => {
            if let Err(e) = w.services.calendar.delete(&id) {
                w.set_error(format!("Could not remove the service: {e}"));
            }
            reload(w);
        }
        Message::Start(id) => {
            start_service(w, &id, cx);
        }
        Message::Capture(id) => {
            capture_service(w, &id, cx);
        }
    }
    cx.notify();
}

/// Put the service the operator is adding on the calendar.
fn add_event(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let name = w.calendar.new_name.trim().to_string();
    if name.is_empty() {
        return;
    }
    // The service starts at the next hour: an operator adding a service is
    // telling the app when it happens, and the hour is the part they mean.
    let start = now() - (now() % 3600) + 3600;
    let repeat = if w.calendar.repeat_days.is_empty() {
        Repeat::Never
    } else {
        Repeat::Weekly {
            days: w.calendar.repeat_days.clone(),
        }
    };
    let event = ServiceEvent {
        name,
        description: String::new(),
        starts_at: start,
        duration_mins: 60,
        repeat,
        playlist_id: w.calendar.playlist_id.clone(),
        capture: w.calendar.capture,
        ..ServiceEvent::default()
    };
    if let Err(e) = w.services.calendar.save(&event) {
        w.set_error(format!("Could not add the service: {e}"));
        return;
    }
    w.calendar.new_name.clear();
    w.calendar.repeat_days.clear();
    reload(w);
    cx.notify();
}

/// Run the service: its playlist is loaded and the show begins.
fn start_service(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    let Some(playlist_id) = w
        .services
        .calendar
        .list()
        .unwrap_or_default()
        .into_iter()
        .find(|event| event.id == id)
        .and_then(|event| event.playlist_id)
    else {
        w.set_error(String::from("That service has no plan attached."));
        cx.notify();
        return;
    };
    let Ok(playlist) = w.services.playlists.get(&playlist_id) else {
        w.set_error(String::from(
            "The plan for that service could not be loaded.",
        ));
        cx.notify();
        return;
    };
    w.service.editing = Some(playlist);
    crate::ui::playlist::start_service(w, cx);
    cx.notify();
}

/// Record the service.
///
/// The recording is the app's own: the capture goes to the file the
/// operator chose, and the service is what is on the output.
fn capture_service(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    // The service runs first: a recording of a service that is not on the
    // output is a recording of nothing.
    start_service(w, id, cx);
    crate::ui::recording::update(w, crate::ui::recording::Message::Start, cx);
    cx.notify();
}

/// Read the calendar again.
fn reload(w: &mut MainWindow) {
    w.calendar.events = w.services.calendar.list().unwrap_or_default();
}

/// The calendar panel.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let tab = w.calendar.tab;
    let events = w.calendar.events.clone();
    let playlists = w.service.plans.clone();
    let now = now();

    let mut panel = div()
        .w(px(460.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // The two lists.
    let mut tabs = div().flex().flex_row().gap(px(4.));
    for candidate in CalendarTab::ALL {
        let active = tab == candidate;
        tabs = tabs.child(
            Button::new(SharedString::from(format!(
                "calendar-tab-{}",
                candidate.label()
            )))
            .label(candidate.label())
            .ghost()
            .xsmall()
            .when(active, |button| button.primary())
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Calendar(Message::SelectTab(candidate)),
                    window,
                    cx,
                );
            })),
        );
    }
    panel = panel.child(tabs);

    // The services.
    let mut list = div().flex().flex_col().gap(px(6.));
    let mut shown = 0;
    for event in &events {
        let occurrences = event.occurrences_between(now - 30 * 86_400, now + 90 * 86_400);
        for at in occurrences {
            let upcoming = at >= now;
            if upcoming != (tab == CalendarTab::Upcoming) {
                continue;
            }
            shown += 1;
            list = list.child(event_row(event, at, upcoming, cx));
        }
    }
    if shown == 0 {
        list = list.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(if tab == CalendarTab::Upcoming {
                    "No services are coming up"
                } else {
                    "No services have been"
                }),
        );
    }
    panel = panel.child(list);

    // The form for a new service.
    panel = panel.child(
        div()
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("NEW SERVICE"),
    );
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .w(px(220.))
                    .child(gpui_kit::component::input::Input::new(
                        &w.calendar_name_input.clone(),
                    )),
            )
            .child(
                Button::new("calendar-add")
                    .label("Add")
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(RootMessage::Calendar(Message::Add), window, cx);
                    })),
            ),
    );

    // The days the service repeats on.
    let mut days = div().flex().flex_row().gap(px(3.));
    for day in Weekday::ALL {
        let on = w.calendar.repeat_days.contains(&day);
        days = days.child(
            Button::new(SharedString::from(format!(
                "calendar-day-{}",
                day.short_name()
            )))
            .label(day.short_name())
            .ghost()
            .xsmall()
            .when(on, |button| button.primary())
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    RootMessage::Calendar(Message::ToggleRepeatDay(day)),
                    window,
                    cx,
                );
            })),
        );
    }
    panel = panel.child(days);

    // The playlist that plays at the service. The operator picks the plan
    // the service runs, so "Run" has something to run.
    if !playlists.is_empty() {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("PLAN"),
        );
        let mut plans = div().flex().flex_wrap().gap(px(4.));
        for playlist in playlists {
            let id = playlist.id.clone();
            let active = w.calendar.playlist_id.as_deref() == Some(id.as_str());
            plans = plans.child(
                Button::new(SharedString::from(format!("calendar-plan-{id}")))
                    .label(playlist.name.clone())
                    .ghost()
                    .xsmall()
                    .when(active, |button| button.primary())
                    .on_click(cx.listener({
                        let id = id.clone();
                        move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::Calendar(Message::SetPlaylist(Some(id.clone()))),
                                window,
                                cx,
                            );
                        }
                    })),
            );
        }
        panel = panel.child(plans);
    }

    // The capture.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child("Record the service"),
            )
            .child(
                Button::new("calendar-capture")
                    .label(if w.calendar.capture { "On" } else { "Off" })
                    .ghost()
                    .xsmall()
                    .when(w.calendar.capture, |button| button.primary())
                    .on_click(cx.listener(|w, _, window, cx| {
                        let capture = !w.calendar.capture;
                        w.handle_message(
                            RootMessage::Calendar(Message::ToggleCapture(capture)),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    panel
}

/// One service on the calendar.
fn event_row(
    event: &ServiceEvent,
    at: i64,
    _upcoming: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let id = event.id.clone();
    let when = local_time(at);
    let date = when.format("%a %d %b").to_string();
    let time = when.format("%H:%M").to_string();
    let away = time_until(at);
    let name = event.name.clone();
    let repeat = event.repeat.label();
    let capture = event.capture;

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .py(px(4.))
        .border_b_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .w(px(96.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(date),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(time),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("{repeat} · {away}")),
                ),
        )
        .child(
            Button::new(SharedString::from(format!("calendar-start-{id}")))
                .label("Run")
                .ghost()
                .xsmall()
                .on_click(cx.listener({
                    let id = id.clone();
                    move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Calendar(Message::Start(id.clone())),
                            window,
                            cx,
                        );
                    }
                })),
        )
        .when(capture, |el| {
            el.child(
                Button::new(SharedString::from(format!("calendar-capture-{id}")))
                    .label("Record")
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener({
                        let id = id.clone();
                        move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::Calendar(Message::Capture(id.clone())),
                                window,
                                cx,
                            );
                        }
                    })),
            )
        })
        .child(
            Button::new(SharedString::from(format!("calendar-delete-{id}")))
                .label("Remove")
                .ghost()
                .xsmall()
                .on_click(cx.listener({
                    let id = id.clone();
                    move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Calendar(Message::Delete(id.clone())),
                            window,
                            cx,
                        );
                    }
                })),
        )
}
