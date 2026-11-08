//! The cue inspector.
//!
//! A cue is an automation: when the slide goes live, the cue fires after
//! its delay and runs its action. The inspector is where the operator sees
//! what a cue will do and changes it — a cue that fires a black screen
//! thirty seconds later is a cue the operator has to be able to read.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::domain::Action;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use crate::ui::theme;

/// The tabs of the cue inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CueTab {
    /// What the cue does and when.
    #[default]
    Properties,
    /// The sound the cue carries.
    Audio,
    /// What the cue is, in words.
    Info,
}

impl CueTab {
    /// The name the tab is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Properties => "Properties",
            Self::Audio => "Audio",
            Self::Info => "Info",
        }
    }

    /// Every tab, in the order the inspector shows them.
    pub const ALL: [CueTab; 3] = [CueTab::Properties, CueTab::Audio, CueTab::Info];
}

/// The actions a cue can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    NextSlide,
    PreviousSlide,
    BlackScreen,
    ClearOutput,
    ClearSlide,
    ClearMedia,
    ClearProps,
    ClearMessages,
    ClearAudio,
    GoToSlide,
    StartTimer,
    StopTimer,
    ResetTimer,
}

impl ActionKind {
    /// The name the action is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::NextSlide => "Next Slide",
            Self::PreviousSlide => "Previous Slide",
            Self::BlackScreen => "Black Screen",
            Self::ClearOutput => "Clear Output",
            Self::ClearSlide => "Clear Slide",
            Self::ClearMedia => "Clear Media",
            Self::ClearProps => "Clear Props",
            Self::ClearMessages => "Clear Messages",
            Self::ClearAudio => "Clear Audio",
            Self::GoToSlide => "Go To Slide",
            Self::StartTimer => "Start Timer",
            Self::StopTimer => "Stop Timer",
            Self::ResetTimer => "Reset Timer",
        }
    }

    /// Every action, in the order the editor shows them.
    pub const ALL: [ActionKind; 13] = [
        ActionKind::NextSlide,
        ActionKind::PreviousSlide,
        ActionKind::GoToSlide,
        ActionKind::BlackScreen,
        ActionKind::ClearOutput,
        ActionKind::ClearSlide,
        ActionKind::ClearMedia,
        ActionKind::ClearProps,
        ActionKind::ClearMessages,
        ActionKind::ClearAudio,
        ActionKind::StartTimer,
        ActionKind::StopTimer,
        ActionKind::ResetTimer,
    ];

    /// The model's action for this choice.
    pub fn to_action(&self) -> Action {
        match self {
            Self::NextSlide => Action::NextSlide,
            Self::PreviousSlide => Action::PrevSlide,
            Self::BlackScreen => Action::BlackScreen(true),
            Self::ClearOutput => Action::ClearOutput,
            Self::ClearSlide => Action::ClearSlide,
            Self::ClearMedia => Action::ClearMedia,
            Self::ClearProps => Action::ClearProps,
            Self::ClearMessages => Action::ClearMessages,
            Self::ClearAudio => Action::ClearAudio,
            // The slide the cue goes to is set by the operator on the cue
            // itself, so the action is built with the first slide.
            Self::GoToSlide => Action::GotoSlide(0),
            Self::StartTimer => Action::StartTimer,
            Self::StopTimer => Action::StopTimer,
            Self::ResetTimer => Action::ResetTimer,
        }
    }

    /// The choice that matches a model's action.
    ///
    /// An action the editor has no choice for — a prop, a look, a message —
    /// is not one of them: the operator picks the action from the list, so
    /// an action that is not on the list is an action they cannot choose.
    pub fn from_action(action: &Action) -> Option<Self> {
        Some(match action {
            Action::NextSlide => Self::NextSlide,
            Action::PrevSlide => Self::PreviousSlide,
            Action::BlackScreen(_) => Self::BlackScreen,
            Action::ClearOutput => Self::ClearOutput,
            Action::ClearSlide => Self::ClearSlide,
            Action::ClearMedia => Self::ClearMedia,
            Action::ClearProps => Self::ClearProps,
            Action::ClearMessages => Self::ClearMessages,
            Action::ClearAudio => Self::ClearAudio,
            Action::GotoSlide(_) => Self::GoToSlide,
            Action::StartTimer => Self::StartTimer,
            Action::StopTimer => Self::StopTimer,
            Action::ResetTimer => Self::ResetTimer,
            Action::TriggerProp(_)
            | Action::ApplyLook(_)
            | Action::ShowMessage(_)
            | Action::HideMessage => return None,
        })
    }
}

/// The cue the inspector is showing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CueSelection {
    /// The index of the slide's cue being inspected.
    pub cue: Option<usize>,
    /// The tab the inspector is showing.
    pub tab: CueTab,
}

impl CueSelection {
    /// Whether the inspector is showing a cue.
    pub fn is_open(&self) -> bool {
        self.cue.is_some()
    }
}

/// Messages owned by the Cue Inspector feature module.
#[derive(Debug, Clone)]
pub enum Message {
    /// Open the inspector on one cue of the slide being edited.
    Open(usize),
    /// Close the inspector.
    Close,
    /// The operator picked a tab.
    SelectTab(CueTab),
    /// The operator renamed the cue.
    NameChanged(String),
    /// The operator changed the delay.
    DelayChanged(u64),
    /// The operator turned the cue on or off.
    EnabledChanged(bool),
    /// The operator picked a different action for the cue.
    ActionChanged(ActionKind),
    /// Open the list of the cues on one slide.
    OpenList(usize),
    /// Close the list of the cues.
    CloseList,
}

/// The cue the inspector is showing.
pub fn selection(w: &MainWindow) -> CueSelection {
    w.cue_inspector
}

/// Open the inspector on one cue.
pub fn open(w: &mut MainWindow, cue: usize) {
    w.cue_inspector.cue = Some(cue);
}

/// Close the inspector.
pub fn close(w: &mut MainWindow) {
    w.cue_inspector.cue = None;
}

/// Apply a message from the cue inspector.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Open(cue) => {
            w.cue_inspector.cue = Some(cue);
        }
        Message::Close => {
            w.cue_inspector.cue = None;
        }
        Message::OpenList(slide) => {
            w.shell.cue_list_slide = Some(slide);
        }
        Message::CloseList => {
            w.shell.cue_list_slide = None;
        }
        Message::SelectTab(tab) => {
            w.cue_inspector.tab = tab;
        }
        Message::NameChanged(name) => {
            edit_cue(w, |cue| cue.name = name, cx);
        }
        Message::DelayChanged(delay) => {
            edit_cue(w, |cue| cue.delay_ms = delay, cx);
        }
        Message::EnabledChanged(enabled) => {
            edit_cue(w, |cue| cue.enabled = enabled, cx);
        }
        Message::ActionChanged(kind) => {
            edit_cue(w, |cue| cue.action = kind.to_action(), cx);
        }
    }
    cx.notify();
}

/// Change the cue the inspector is showing.
///
/// The change is undone and saved like any other change to the slide: a
/// cue that the operator cannot take back is a cue they will not touch.
fn edit_cue(
    w: &mut MainWindow,
    change: impl FnOnce(&mut crate::domain::Cue),
    cx: &mut Context<MainWindow>,
) {
    let Some(index) = w.cue_inspector.cue else {
        return;
    };
    // The cue is read before the undo snapshot is taken, so a change that
    // does nothing does not fill the undo stack.
    let Some(cue) = w
        .get_current_slide()
        .and_then(|slide| slide.cues.get(index))
    else {
        return;
    };
    let mut updated = cue.clone();
    change(&mut updated);
    if updated.name == cue.name
        && updated.action == cue.action
        && updated.delay_ms == cue.delay_ms
        && updated.enabled == cue.enabled
    {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && let Some(cue) = slide.cues.get_mut(index)
    {
        *cue = updated;
    }
    crate::ui::layers::save_current_slide(w, cx);
}

/// The list of the cues on a slide.
///
/// The list is how the operator reaches a cue: a cue is an automation, and
/// an automation the operator cannot see is an automation they will not
/// trust.
pub fn list_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let Some(index) = w.shell.cue_list_slide else {
        return div()
            .w(px(320.))
            .p(px(12.))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("No slide selected");
    };
    let Some(slide) = w
        .editor
        .editing
        .as_ref()
        .and_then(|presentation| presentation.slides.get(index))
    else {
        return div()
            .w(px(320.))
            .p(px(12.))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("No slide selected");
    };
    let cues = slide.cues.clone();

    let mut panel = div()
        .w(px(320.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    if cues.is_empty() {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("This slide has no cues"),
        );
        return panel;
    }

    for (cue_index, cue) in cues.iter().enumerate() {
        let action = ActionKind::from_action(&cue.action)
            .map(|kind| kind.label())
            .unwrap_or("Prop, look or message")
            .to_string();
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(cue.name.clone()),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("{} ms · {}", cue.delay_ms, action)),
                )
                .child(
                    Button::new(SharedString::from(format!("cue-open-{cue_index}")))
                        .label("Inspect")
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::CueInspector(Message::Open(cue_index)),
                                window,
                                cx,
                            );
                        })),
                ),
        );
    }
    panel
}

/// The cue inspector.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let Some(index) = w.cue_inspector.cue else {
        return div()
            .w(px(300.))
            .p(px(12.))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("No cue selected");
    };
    let Some(cue) = w
        .get_current_slide()
        .and_then(|slide| slide.cues.get(index))
        .cloned()
    else {
        return div()
            .w(px(300.))
            .p(px(12.))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("No cue selected");
    };
    let tab = w.cue_inspector.tab;

    let mut panel = div()
        .w(px(300.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL));

    // The tabs.
    let mut tabs = div().flex().flex_row().gap(px(4.));
    for candidate in CueTab::ALL {
        let active = tab == candidate;
        tabs = tabs.child(
            Button::new(SharedString::from(format!("cue-tab-{}", candidate.label())))
                .label(candidate.label())
                .ghost()
                .xsmall()
                .when(active, |button| button.primary())
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::CueInspector(Message::SelectTab(candidate)),
                        window,
                        cx,
                    );
                })),
        );
    }
    panel = panel.child(tabs);

    match tab {
        CueTab::Properties => {
            panel = panel.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Name"),
                    )
                    .child(
                        div()
                            .w(px(150.))
                            .child(Input::new(&w.cue_name_input.clone())),
                    ),
            );
            panel = panel.child(delay_row(cue.delay_ms, cx));
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
                            .child("Enabled"),
                    )
                    .child(
                        Button::new("cue-enabled")
                            .label(if cue.enabled { "On" } else { "Off" })
                            .ghost()
                            .xsmall()
                            .when(cue.enabled, |button| button.primary())
                            .on_click(cx.listener(|w, _, window, cx| {
                                let enabled = !w
                                    .get_current_slide()
                                    .and_then(|slide| {
                                        w.cue_inspector.cue.and_then(|index| slide.cues.get(index))
                                    })
                                    .map(|cue| cue.enabled)
                                    .unwrap_or(false);
                                w.handle_message(
                                    RootMessage::CueInspector(Message::EnabledChanged(enabled)),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            );
            panel = panel.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("Action"),
            );
            let mut actions = div().flex().flex_wrap().gap(px(4.));
            for kind in ActionKind::ALL {
                let active = ActionKind::from_action(&cue.action) == Some(kind);
                actions = actions.child(
                    Button::new(SharedString::from(format!("cue-action-{}", kind.label())))
                        .label(kind.label())
                        .ghost()
                        .xsmall()
                        .when(active, |button| button.primary())
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                RootMessage::CueInspector(Message::ActionChanged(kind)),
                                window,
                                cx,
                            );
                        })),
                );
            }
            panel = panel.child(actions);
        }
        CueTab::Audio => {
            // The cue carries one delay, and the sound waits for it: a cue
            // that makes a noise at the same time as it changes the picture
            // is a cue whose noise arrives with the old picture. The delay
            // is therefore read on both sides, and the sound's is the
            // picture's plus what the operator sets here.
            panel = panel.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child("Delay"),
            );
            panel = panel.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_PRIMARY))
                    .child(format!("{} ms", cue.delay_ms)),
            );
            panel = panel.child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("The sound waits for the cue, so it arrives with the picture it belongs to."),
            );
        }
        CueTab::Info => {
            panel = panel.child(info_row("Name", &cue.name));
            panel = panel.child(info_row("Type", "Cue"));
            panel = panel.child(info_row("Delay", &format!("{} ms", cue.delay_ms)));
            panel = panel.child(info_row("Enabled", if cue.enabled { "Yes" } else { "No" }));
            panel = panel.child(info_row(
                "Action",
                ActionKind::from_action(&cue.action)
                    .map(|kind| kind.label())
                    .unwrap_or("Prop, look or message"),
            ));
        }
    }

    panel
}

/// The row that sets the cue's delay.
///
/// The delay is a step rather than a field: a cue that fires 30 seconds
/// later is a cue the operator tunes by ear, and a field that swallows the
/// keystroke is a field they fight.
fn delay_row(delay: u64, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child("Delay"),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(format!("{delay} ms")),
        );
    for step in [-1000i64, -100, 100, 1000] {
        let next = (delay as i64 + step).max(0) as u64;
        row = row.child(
            Button::new(SharedString::from(format!("cue-delay-{step}")))
                .label(match step {
                    -1000 => "--",
                    -100 => "-",
                    100 => "+",
                    _ => "++",
                })
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::CueInspector(Message::DelayChanged(next)),
                        window,
                        cx,
                    );
                })),
        );
    }
    row
}

/// A read-only row of the info tab.
fn info_row(label: &'static str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(value.to_string()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every action the editor offers is an action the model has.
    #[test]
    fn every_action_maps_to_the_model() {
        for kind in ActionKind::ALL {
            let action = kind.to_action();
            assert_eq!(
                ActionKind::from_action(&action),
                Some(kind),
                "{} does not map back to itself",
                kind.label()
            );
        }
    }

    /// An action the editor has no choice for is not offered.
    #[test]
    fn an_action_the_editor_has_no_choice_for_is_not_offered() {
        assert_eq!(
            ActionKind::from_action(&Action::TriggerProp(String::from("logo"))),
            None
        );
        assert_eq!(
            ActionKind::from_action(&Action::ApplyLook(String::from("default"))),
            None
        );
    }
}
