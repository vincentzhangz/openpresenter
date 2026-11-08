//! Triggers (HTTP / OSC / MIDI / automation macros) panel.
//!
//! Ported from the iced-era `ui/triggers/mod.rs`: start/stop of the HTTP REST
//! listener and OSC listener with their port fields and at-launch toggles
//! (persisted through the app config), MIDI device selection, and the macro
//! lifecycle (add / run / stop / loop, with tokio join-handle bookkeeping).
//! The panel renders in the right dock's Triggers tab and as a modal opened
//! by `Message::ToggleTriggersPanel`. Incoming remote actions bypass this
//! enum entirely: they arrive as root `Message::TriggerFired` over the
//! trigger channel bridge and are applied by [`apply_action`].

use gpui_kit::base::Disableable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, Styled, Window, div, px};

use crate::triggers::Action;
use crate::triggers::automation::Macro;
use crate::ui::main_window::MainWindow;
use crate::ui::theme;

/// Mirror trigger state into the port / macro-name inputs.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let http_port = w.triggers.http_port_str.clone();
    crate::ui::components::sync_text_field(&w.triggers_http_port_input, &http_port, window, cx);
    let osc_port = w.triggers.osc_port_str.clone();
    crate::ui::components::sync_text_field(&w.triggers_osc_port_input, &osc_port, window, cx);
    let macro_name = w.triggers.new_macro_name.clone();
    crate::ui::components::sync_text_field(&w.triggers_macro_name_input, &macro_name, window, cx);
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpStart,
    HttpStop,
    HttpPortChanged(String),
    /// Toggle whether the HTTP listener starts automatically at launch.
    HttpAutoStartToggled(bool),
    OscStart,
    OscStop,
    OscPortChanged(String),
    /// Toggle whether the OSC listener starts automatically at launch.
    OscAutoStartToggled(bool),
    MacroAdd,
    MacroRemove(String),
    MacroRun(String),
    MacroStop(String),
    MacroToggleLoop(String),
    /// Add a step to a macro.
    MacroAddStep(String),
    /// Move a step to the next action.
    MacroCycleStepAction(String, usize),
    /// Move a step to the next delay.
    MacroCycleStepDelay(String, usize),
    /// Remove a step from a macro.
    MacroRemoveStep(String, usize),
    MacroNameChanged(String),
    MidiStart,
    MidiStop,
    MidiRefresh,
    SelectMidiPort(usize),
}

/// Dispatch a triggers message.
pub(crate) fn update(
    w: &mut MainWindow,
    msg: Message,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match msg {
        Message::HttpStart => http_start(w, cx),
        Message::HttpStop => http_stop(w, cx),
        Message::HttpPortChanged(s) => http_port_changed(w, s, cx),
        Message::HttpAutoStartToggled(v) => http_autostart_toggled(w, v, cx),
        Message::OscStart => osc_start(w, cx),
        Message::OscStop => osc_stop(w, cx),
        Message::OscPortChanged(s) => osc_port_changed(w, s, cx),
        Message::OscAutoStartToggled(v) => osc_autostart_toggled(w, v, cx),
        Message::MacroAdd => macro_add(w, window, cx),
        Message::MacroRemove(id) => macro_remove(w, &id, cx),
        Message::MacroRun(id) => macro_run(w, &id, cx),
        Message::MacroAddStep(id) => macro_add_step(w, &id, cx),
        Message::MacroCycleStepAction(id, index) => macro_cycle_step_action(w, &id, index, cx),
        Message::MacroCycleStepDelay(id, index) => macro_cycle_step_delay(w, &id, index, cx),
        Message::MacroRemoveStep(id, index) => macro_remove_step(w, &id, index, cx),
        Message::MacroStop(id) => macro_stop(w, &id, cx),
        Message::MacroToggleLoop(id) => macro_toggle_loop(w, &id, cx),
        Message::MacroNameChanged(s) => macro_name_changed(w, s, cx),
        Message::MidiStart => midi_start(w, cx),
        Message::MidiStop => midi_stop(w, cx),
        Message::MidiRefresh => midi_refresh(w, cx),
        Message::SelectMidiPort(p) => select_midi_port(w, p, cx),
    }
}

/// Open (or close) the triggers panel modal.
pub(crate) fn toggle_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.triggers.panel_open = !w.triggers.panel_open;
    cx.notify();
}

fn http_start(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Ok(port) = w.triggers.http_port_str.parse::<u16>() {
        w.triggers.manager.http_port = port;
    }
    w.triggers.manager.start_http();
    cx.notify();
}

fn http_stop(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.triggers.manager.stop_http();
    cx.notify();
}

pub(crate) fn http_port_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.triggers.http_port_str = s.clone();
    // The port is a setting: it is saved so the listener comes back on the
    // same port after a restart.
    if let Ok(port) = s.parse::<u16>() {
        w.app_config.triggers.http_port = port;
        // The manager is the listener's own copy of the port, so it has to
        // follow the field: otherwise the panel shows the new port while
        // the server keeps the old one.
        if w.triggers.manager.http_port != port {
            let was_running = w.triggers.manager.http_running;
            w.triggers.manager.stop_http();
            w.triggers.manager.http_port = port;
            if was_running {
                w.triggers.manager.start_http();
            }
        }
        persist_autostart(w);
    } else if !s.trim().is_empty() {
        w.set_error("The port has to be a number between 1 and 65535.".to_string());
    }
    cx.notify();
}

fn http_autostart_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.app_config.triggers.http_enabled = v;
    persist_autostart(w);
    cx.notify();
}

fn osc_start(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Ok(port) = w.triggers.osc_port_str.parse::<u16>() {
        w.triggers.manager.osc_port = port;
    }
    w.triggers.manager.start_osc();
    cx.notify();
}

fn osc_stop(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.triggers.manager.stop_osc();
    cx.notify();
}

pub(crate) fn osc_port_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.triggers.osc_port_str = s.clone();
    // The port is a setting: it is saved so the listener comes back on the
    // same port after a restart.
    if let Ok(port) = s.parse::<u16>() {
        w.app_config.triggers.osc_port = port;
        // The manager is the listener's own copy of the port, so it has to
        // follow the field: otherwise the panel shows the new port while
        // the listener keeps the old one.
        if w.triggers.manager.osc_port != port {
            let was_running = w.triggers.manager.osc_running;
            w.triggers.manager.stop_osc();
            w.triggers.manager.osc_port = port;
            if was_running {
                w.triggers.manager.start_osc();
            }
        }
        persist_autostart(w);
    } else if !s.trim().is_empty() {
        w.set_error("The port has to be a number between 1 and 65535.".to_string());
    }
    cx.notify();
}

fn osc_autostart_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.app_config.triggers.osc_enabled = v;
    persist_autostart(w);
    cx.notify();
}

/// Persist the at-launch toggles to the config file.
fn persist_autostart(w: &mut MainWindow) {
    if let Err(e) = w.app_config.save() {
        w.set_error(format!("Failed to save settings: {e}"));
    }
}

// ---------------------------------------------------------------------------
// Automation macros
// ---------------------------------------------------------------------------

/// The actions a macro step can carry.
///
/// The step button cycles through them, so a macro can be built without a
/// picker per step.
const MACRO_ACTIONS: [crate::domain::Action; 6] = [
    crate::domain::Action::NextSlide,
    crate::domain::Action::PrevSlide,
    crate::domain::Action::BlackScreen(true),
    crate::domain::Action::BlackScreen(false),
    crate::domain::Action::ClearOutput,
    crate::domain::Action::ClearMessages,
];

/// The delays a macro step can wait, in milliseconds.
const MACRO_DELAYS: [u64; 5] = [0, 1_000, 3_000, 5_000, 10_000];

fn macro_add(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    if w.triggers.new_macro_name.trim().is_empty() {
        return;
    }
    let name = w.triggers.new_macro_name.trim().to_string();
    let m = Macro::new(name);
    w.triggers.manager.macros.push(m);
    w.triggers.new_macro_name.clear();
    // Mirror the cleared name into the input; `set_value` emits no Change, so
    // the subscription does not loop.
    w.triggers_macro_name_input.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
    save_macros(w);
    cx.notify();
}

fn macro_remove(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    stop_macro(w, id);
    w.triggers.manager.macros.retain(|m| m.id != id);
    save_macros(w);
    cx.notify();
}

/// Stop a running macro, if one is running.
///
/// The handles live in `TriggersState`; the panel and the buttons always
/// agree about which macro is running.
fn stop_macro(w: &mut MainWindow, id: &str) {
    if let Some(handle) = w.triggers.macro_running_handles.remove(id) {
        handle.abort();
    }
    w.triggers.macro_running_ids.remove(id);
}

fn macro_run(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    let tx = w.triggers.manager.sender();
    // Stop a run of the same macro first, so the old task cannot keep
    // firing its actions behind the new one.
    stop_macro(w, id);
    if let Some(handle) = w
        .triggers
        .manager
        .macros
        .iter()
        .find(|m| m.id == id)
        .map(|m| m.spawn(tx))
    {
        w.triggers
            .macro_running_handles
            .insert(id.to_string(), handle);
        w.triggers.macro_running_ids.insert(id.to_string());
    }
    cx.notify();
}

fn macro_stop(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    stop_macro(w, id);
    cx.notify();
}

fn macro_toggle_loop(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    let looping = w
        .triggers
        .manager
        .macros
        .iter()
        .find(|m| m.id == id)
        .is_some_and(|m| !m.looping);
    // The flag is shared with a running macro, so the switch takes effect
    // at the end of the current pass.
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id) {
        m.looping = looping;
    }
    save_macros(w);
    cx.notify();
}

/// Add a step to a macro.
///
/// A macro without a step does nothing, so a macro could never fire: the
/// steps are what the panel edits.
fn macro_add_step(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id) {
        m.add_step(0, MACRO_ACTIONS[0].clone());
    }
    save_macros(w);
    cx.notify();
}

/// Move a step to the next action.
fn macro_cycle_step_action(
    w: &mut MainWindow,
    id: &str,
    index: usize,
    cx: &mut Context<MainWindow>,
) {
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id)
        && let Some(step) = m.steps.get(index)
    {
        let current = MACRO_ACTIONS
            .iter()
            .position(|action| action == &step.action)
            .unwrap_or(0);
        let next = MACRO_ACTIONS[(current + 1) % MACRO_ACTIONS.len()].clone();
        m.steps[index].action = next;
    }
    save_macros(w);
    cx.notify();
}

/// Move a step to the next delay.
fn macro_cycle_step_delay(
    w: &mut MainWindow,
    id: &str,
    index: usize,
    cx: &mut Context<MainWindow>,
) {
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id)
        && let Some(step) = m.steps.get(index)
    {
        let current = MACRO_DELAYS
            .iter()
            .position(|delay| *delay == step.delay_ms)
            .unwrap_or(0);
        let next = MACRO_DELAYS[(current + 1) % MACRO_DELAYS.len()];
        m.steps[index].delay_ms = next;
    }
    save_macros(w);
    cx.notify();
}

fn macro_remove_step(w: &mut MainWindow, id: &str, index: usize, cx: &mut Context<MainWindow>) {
    if let Some(m) = w.triggers.manager.macros.iter_mut().find(|m| m.id == id) {
        m.remove_step(index);
    }
    save_macros(w);
    cx.notify();
}

/// Save the macros with the rest of the settings.
///
/// A macro is the operator's automation, so it belongs with the rest of the
/// configuration: a macro that is gone the next morning is a macro that
/// cannot be used.
fn save_macros(w: &mut MainWindow) {
    w.app_config.triggers.macros = w.triggers.manager.macros.clone();
    if let Err(e) = w.app_config.save() {
        w.set_error(format!("Failed to save settings: {e}"));
    }
}

pub(crate) fn macro_name_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.triggers.new_macro_name = s;
    cx.notify();
}

// ---------------------------------------------------------------------------
// MIDI
// ---------------------------------------------------------------------------

fn midi_start(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let port = w.triggers.selected_midi_port.unwrap_or(0);
    if let Err(e) = w.triggers.manager.start_midi(port) {
        w.set_error(format!("MIDI error: {e}"));
    }
    cx.notify();
}

fn midi_stop(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.triggers.manager.stop_midi();
    cx.notify();
}

fn midi_refresh(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.triggers.midi_ports = crate::triggers::midi::list_midi_inputs();
    if w.triggers.selected_midi_port.is_none() && !w.triggers.midi_ports.is_empty() {
        w.triggers.selected_midi_port = Some(0);
    }
    cx.notify();
}

fn select_midi_port(w: &mut MainWindow, port: usize, cx: &mut Context<MainWindow>) {
    w.triggers.selected_midi_port = Some(port);
    if w.triggers.manager.midi_running
        && let Err(e) = w.triggers.manager.start_midi(port)
    {
        // The port did not open: the operator has to know, or the MIDI
        // control is simply dead and they are looking for the reason.
        w.set_error(format!("MIDI error: {e}"));
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Remote actions
// ---------------------------------------------------------------------------

/// Apply a remote-control action to presenting state. Ported from the
/// iced-era `triggers::apply_action`; every presenting mutation routes
/// through `navigation`, which also pushes the NDI output state.
pub(crate) fn apply_action(w: &mut MainWindow, action: Action, cx: &mut Context<MainWindow>) {
    use crate::ui::navigation;
    match action {
        Action::NextSlide => navigation::next_slide(w, cx),
        Action::PrevSlide => navigation::prev_slide(w, cx),
        Action::GotoSlide(idx) => navigation::select_slide(w, idx, cx),
        Action::BlackScreen(on) => {
            w.output.black_screen = on;
            // The broadcast composites the slide itself, so it has to be
            // blacked (or restored) as well.
            crate::ui::ndi::set_black_screen(w, on, cx);
            cx.notify();
        }
        Action::ClearOutput => navigation::clear_output(w, cx),
        Action::ClearSlide => navigation::clear_slide(w, cx),
        Action::ClearMedia => navigation::clear_media(w, cx),
        Action::ClearProps => navigation::clear_props(w, cx),
        Action::ClearMessages => navigation::clear_messages(w, cx),
        Action::ClearAudio => navigation::clear_audio(w, cx),
        Action::TriggerProp(id) => {
            w.props.manager.toggle_prop(&id);
            // The broadcast composites the slide itself, so it has to be
            // told about the change.
            crate::ui::props::push_overlay(w);
            cx.notify();
        }
        Action::ApplyLook(id) => {
            // The same path the operator's button takes: making a look live
            // brings its mask and its props with it.
            crate::ui::output::select_look(w, id, cx);
        }
        Action::ShowMessage(text) => {
            w.props.manager.set_message(text);
            crate::ui::props::push_overlay(w);
            cx.notify();
        }
        Action::HideMessage => {
            w.props.manager.clear_message();
            crate::ui::props::push_overlay(w);
            cx.notify();
        }
        Action::StartTimer => navigation::start_timer(w, cx),
        Action::StopTimer => navigation::stop_timer(w, cx),
        Action::ResetTimer => navigation::reset_timer(w, cx),
    }
}

// ---------------------------------------------------------------------------
// View (right dock Triggers tab + modal)
// ---------------------------------------------------------------------------

/// Render the triggers panel body (used both in the dock tab and the modal).
/// The dock wraps it with [`dock_view`]; the modal wraps it in a titled
/// dialog overlay.
pub(crate) fn triggers_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let http_running = w.triggers.manager.http_running;
    let http_port = w.triggers.http_port_str.clone();
    let osc_running = w.triggers.manager.osc_running;
    let osc_port = w.triggers.osc_port_str.clone();
    let midi_running = w.triggers.manager.midi_running;
    let midi_ports = w.triggers.midi_ports.clone();
    let selected_midi_port = w.triggers.selected_midi_port;
    let macros = w.triggers.manager.macros.clone();
    // The handles live in `TriggersState`, so the panel and the buttons
    // always agree about which macro is running.
    let running_ids: std::collections::HashSet<String> = w.triggers.macro_running_ids.clone();
    let http_autostart = w.app_config.triggers.http_enabled;
    let osc_autostart = w.app_config.triggers.osc_enabled;

    // --- HTTP REST ---------------------------------------------------------
    let http_label = if http_running {
        format!("HTTP server running on :{http_port}")
    } else {
        format!("HTTP server (port {http_port})")
    };
    let http_section = section(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(section_label("HTTP REST"))
            .child(div().flex_1())
            .child(
                div()
                    .w(px(70.))
                    .child(Input::new(&w.triggers_http_port_input.clone())),
            )
            .child(
                Button::new("triggers-http-toggle")
                    .label(if http_running { "Stop" } else { "Start" })
                    .when(http_running, |b| b.secondary())
                    .when(!http_running, |b| b.primary())
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        let running = w.triggers.manager.http_running;
                        let msg = if running {
                            Message::HttpStop
                        } else {
                            Message::HttpStart
                        };
                        w.handle_message(crate::ui::messages::Message::Triggers(msg), window, cx);
                    })),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(http_label),
            ),
    )
    .child(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(div().flex_1())
            .child(
                Switch::new("triggers-http-autostart")
                    .checked(http_autostart)
                    .on_click(cx.listener(|w, checked: &bool, window, cx| {
                        w.handle_message(
                            crate::ui::messages::Message::Triggers(Message::HttpAutoStartToggled(
                                *checked,
                            )),
                            window,
                            cx,
                        );
                    })),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child("At launch"),
            ),
    )
    .child(
        div()
            .text_size(px(10.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("Accepts unauthenticated commands; trusted networks only."),
    );

    // --- OSC ---------------------------------------------------------------
    let osc_label = if osc_running {
        format!("OSC listening on :{osc_port}")
    } else {
        format!("OSC listener (port {osc_port})")
    };
    let osc_section = section(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(section_label("OSC"))
            .child(div().flex_1())
            .child(
                div()
                    .w(px(70.))
                    .child(Input::new(&w.triggers_osc_port_input.clone())),
            )
            .child(
                Button::new("triggers-osc-toggle")
                    .label(if osc_running { "Stop" } else { "Start" })
                    .when(osc_running, |b| b.secondary())
                    .when(!osc_running, |b| b.primary())
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        let running = w.triggers.manager.osc_running;
                        let msg = if running {
                            Message::OscStop
                        } else {
                            Message::OscStart
                        };
                        w.handle_message(crate::ui::messages::Message::Triggers(msg), window, cx);
                    })),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(osc_label),
            ),
    )
    .child(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(div().flex_1())
            .child(
                Switch::new("triggers-osc-autostart")
                    .checked(osc_autostart)
                    .on_click(cx.listener(|w, checked: &bool, window, cx| {
                        w.handle_message(
                            crate::ui::messages::Message::Triggers(Message::OscAutoStartToggled(
                                *checked,
                            )),
                            window,
                            cx,
                        );
                    })),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child("At launch"),
            ),
    );

    // --- MIDI --------------------------------------------------------------
    let midi_label = if midi_running {
        let port_name = selected_midi_port
            .and_then(|idx| midi_ports.iter().find(|p| p.index == idx))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| String::from("Connected"));
        format!("MIDI active: {port_name} (Note 60/59 Next/Prev, CC 64 Pedal)")
    } else if midi_ports.is_empty() {
        String::from("No MIDI devices detected")
    } else {
        format!("{} MIDI device(s) ready", midi_ports.len())
    };

    let midi_startable = !midi_running && !midi_ports.is_empty();
    let midi_section = section(
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(section_label("MIDI Remote"))
            .child(div().flex_1())
            .child(
                Button::new("triggers-midi-refresh")
                    .icon(gpui_kit::component::Icon::new(
                        gpui_kit::component::IconName::RefreshCw,
                    ))
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            crate::ui::messages::Message::Triggers(Message::MidiRefresh),
                            window,
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("triggers-midi-toggle")
                    .label(if midi_running { "Stop" } else { "Start" })
                    .when(midi_running, |b| b.secondary())
                    .when(!midi_running, |b| b.primary())
                    .when(midi_startable, |b| {
                        b.on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(Message::MidiStart),
                                window,
                                cx,
                            );
                        }))
                    })
                    .when(midi_running, |b| {
                        b.on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(Message::MidiStop),
                                window,
                                cx,
                            );
                        }))
                    }),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(midi_label),
            ),
    )
    .child(midi_port_buttons(midi_ports, selected_midi_port, cx))
    // The keys the remote listens for: the operator has to know which key to
    // press, and a mapping that is only in the source is a mapping they have
    // to guess.
    .child(midi_key_map());

    // --- Macros ------------------------------------------------------------
    let macro_section = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(section_label("Automation macros"))
        .child(macro_list(macros, running_ids, cx))
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&w.triggers_macro_name_input.clone())),
                )
                .child(
                    Button::new("triggers-macro-add")
                        .label("+ Macro")
                        .secondary()
                        .small()
                        // The macro needs a name, so the button is out of
                        // reach until one is typed. A button that does
                        // nothing when it is pressed is a dead control.
                        .disabled(w.triggers.new_macro_name.trim().is_empty())
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(Message::MacroAdd),
                                window,
                                cx,
                            );
                        })),
                ),
        );

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(http_section)
        .child(osc_section)
        .child(midi_section)
        .child(macro_section)
        .child(routes_section(&http_port))
}

/// The list of routes the remote control surface serves.
///
/// The operator configures a controller from this list, so it has to be the
/// list the server actually serves — the routes live in one place and a test
/// holds the two together.
fn routes_section(http_port: &str) -> Div {
    let mut rows = div().flex().flex_col().gap(px(2.));
    for route in crate::triggers::http::ROUTES {
        rows = rows.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(44.))
                        .text_size(px(9.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(route.method),
                )
                .child(
                    div()
                        .w(px(170.))
                        .text_size(px(10.))
                        .text_color(theme::c(theme::ACCENT_ORANGE))
                        .child(route.path),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(10.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(route.what),
                ),
        );
    }

    section(
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(section_label("REMOTE ROUTES"))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(9.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child(format!("POST http://<this machine>:{http_port}<path>")),
                    ),
            )
            .child(rows),
    )
}

/// Render the triggers tab content: a titled header row above the panel body.
pub(crate) fn dock_view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child("Triggers & Automation"),
        )
        .child(triggers_panel(w, cx))
}

/// A bordered section wrapper for one listener group.
fn section(header_row: impl IntoElement) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(8.))
        .rounded(px(5.))
        .bg(theme::c(theme::BG_DARK))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(header_row)
}

fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .child(text)
}

/// The MIDI device port pills (or the "no devices" hint).
/// The keys the remote listens for, and what they do.
fn midi_key_map() -> Div {
    let mut rows = div().flex().flex_col().gap(px(1.)).px(px(8.)).pb(px(6.));
    for (note, what, _) in crate::triggers::midi::NOTE_KEYS {
        rows = rows.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(74.))
                        .text_size(px(9.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("Note {note}")),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(9.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(*what),
                ),
        );
    }
    for (control, _, what, _) in crate::triggers::midi::CONTROL_KEYS {
        rows = rows.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(74.))
                        .text_size(px(9.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("CC {control}")),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(9.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(*what),
                ),
        );
    }
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(section_label("KEY MAPPING"))
        .child(rows)
}

fn midi_port_buttons(
    ports: Vec<crate::triggers::midi::MidiPortInfo>,
    selected: Option<usize>,
    cx: &mut Context<MainWindow>,
) -> Div {
    if ports.is_empty() {
        return div().child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Connect a USB MIDI keyboard or foot pedal and refresh."),
        );
    }
    let mut row = div().w_full().flex().flex_row().flex_wrap().gap(px(4.));
    for p in ports {
        let is_sel = selected == Some(p.index);
        row = row.child(
            div()
                .id(SharedString::from(format!("midi-port-{}", p.index)))
                .py(px(3.))
                .px(px(8.))
                .rounded(px(4.))
                .text_size(px(11.))
                .text_color(if is_sel {
                    theme::c(theme::TEXT_PRIMARY)
                } else {
                    theme::c(theme::TEXT_SECONDARY)
                })
                .bg(if is_sel {
                    theme::c(theme::ACCENT_ORANGE).opacity(0.16)
                } else {
                    gpui_kit::transparent_black()
                })
                .border_1()
                .border_color(if is_sel {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BORDER_PANEL)
                })
                .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Triggers(Message::SelectMidiPort(p.index)),
                        window,
                        cx,
                    );
                }))
                .child(p.name),
        );
    }
    row
}

/// The macro rows (or the "no macros" hint).
fn macro_list(
    macros: Vec<Macro>,
    running_ids: std::collections::HashSet<String>,
    cx: &mut Context<MainWindow>,
) -> Div {
    if macros.is_empty() {
        return div().child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No macros defined."),
        );
    }
    let mut list = div().w_full().flex().flex_col().gap(px(4.));
    for m in macros {
        let run_id = m.id.clone();
        let remove_id = m.id.clone();
        let loop_id = m.id.clone();
        let is_running = running_ids.contains(&m.id);
        let looping = m.looping;
        list = list.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(m.name),
                )
                .child(
                    Switch::new(SharedString::from(format!("macro-loop-{}", m.id)))
                        .checked(looping)
                        .on_click(cx.listener(move |w, _checked: &bool, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(Message::MacroToggleLoop(
                                    loop_id.clone(),
                                )),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("loop"),
                )
                .child(
                    Button::new(SharedString::from(format!("macro-run-{}", m.id)))
                        .label(if is_running { "Stop" } else { "Run" })
                        .when(is_running, |b| b.secondary())
                        .when(!is_running, |b| b.primary())
                        .small()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            let msg = if is_running {
                                Message::MacroStop(run_id.clone())
                            } else {
                                Message::MacroRun(run_id.clone())
                            };
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(msg),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new(SharedString::from(format!("macro-remove-{}", m.id)))
                        .label("×")
                        .ghost()
                        .small()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Triggers(Message::MacroRemove(
                                    remove_id.clone(),
                                )),
                                window,
                                cx,
                            );
                        })),
                ),
        );

        // The steps are what a macro runs. A macro without a step does
        // nothing, so they are edited here: the action and the delay of a
        // step are cycled by clicking them.
        let mut steps = div().w_full().flex().flex_col().gap(px(2.)).pl(px(8.));
        if m.steps.is_empty() {
            steps = steps.child(
                div()
                    .text_size(px(10.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("No steps yet — add one to make the macro do something"),
            );
        }
        for (index, step) in m.steps.iter().enumerate() {
            let id = m.id.clone();
            let id_for_action = id.clone();
            let id_for_delay = id.clone();
            let id_for_remove = id.clone();
            let action_label = macro_action_label(&step.action);
            let delay_label = macro_delay_label(step.delay_ms);
            steps = steps.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        Button::new(SharedString::from(format!(
                            "macro-step-action-{id}-{index}"
                        )))
                        .label(action_label)
                        .ghost()
                        .small()
                        .on_click(cx.listener(
                            move |w, _, window, cx| {
                                w.handle_message(
                                    crate::ui::messages::Message::Triggers(
                                        Message::MacroCycleStepAction(id_for_action.clone(), index),
                                    ),
                                    window,
                                    cx,
                                );
                            },
                        )),
                    )
                    .child(
                        Button::new(SharedString::from(format!("macro-step-delay-{id}-{index}")))
                            .label(delay_label)
                            .ghost()
                            .small()
                            .on_click(cx.listener(move |w, _, window, cx| {
                                w.handle_message(
                                    crate::ui::messages::Message::Triggers(
                                        Message::MacroCycleStepDelay(id_for_delay.clone(), index),
                                    ),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!(
                            "macro-step-remove-{id}-{index}"
                        )))
                        .label("×")
                        .ghost()
                        .small()
                        .on_click(cx.listener(
                            move |w, _, window, cx| {
                                w.handle_message(
                                    crate::ui::messages::Message::Triggers(
                                        Message::MacroRemoveStep(id_for_remove.clone(), index),
                                    ),
                                    window,
                                    cx,
                                );
                            },
                        )),
                    ),
            );
        }
        let add_id = m.id.clone();
        steps = steps.child(
            Button::new(SharedString::from(format!("macro-step-add-{}", m.id)))
                .label("+ Step")
                .ghost()
                .small()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Triggers(Message::MacroAddStep(
                            add_id.clone(),
                        )),
                        window,
                        cx,
                    );
                })),
        );
        list = list.child(steps);
    }
    list
}

/// The label of a macro step's action.
fn macro_action_label(action: &crate::domain::Action) -> &'static str {
    match action {
        crate::domain::Action::NextSlide => "Next",
        crate::domain::Action::PrevSlide => "Prev",
        crate::domain::Action::BlackScreen(true) => "Black",
        crate::domain::Action::BlackScreen(false) => "Unblack",
        crate::domain::Action::ClearOutput => "Clear all",
        crate::domain::Action::ClearMessages => "Clear msg",
        _ => "Action",
    }
}

/// The label of a macro step's delay.
fn macro_delay_label(delay_ms: u64) -> String {
    if delay_ms == 0 {
        return String::from("No wait");
    }
    if delay_ms.is_multiple_of(1000) {
        format!("{}s", delay_ms / 1000)
    } else {
        format!("{delay_ms}ms")
    }
}
