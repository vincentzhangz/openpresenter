//! Output system UI: the app settings ("Preferences") modal, the Looks &
//! Screens matrix modal, and the screens/outputs management logic.
//!
//! Ported from the iced-era `ui/output/mod.rs` + `ui/output/matrix.rs`. The
//! module owns:
//! - the multi-screen output manager wiring (add/remove window & NDI
//!   destinations, active toggles, content cycling, resolution presets),
//! - the settings modal with its five tabs (General / Screens / Network & NDI
//!   / Transitions / Advanced) persisting through [`crate::config::Config`],
//! - the output-window state (black screen, fullscreen flag, the real OS
//!   windows the output manager drives),
//! - the looks matrix (screens × layers routing table per [`Look`]).
//!
//! The remote-trigger port edits (`http_port_changed`/`osc_port_changed`)
//! deliberately mirror the same state the Triggers panel owns — the triggers
//! panel is the single editing surface for ports; the settings tab links to
//! it instead of duplicating the inputs.

use std::sync::Arc;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::slider::Slider;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::relative;
use std::collections::HashMap;

use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{Bounds, Context, Div, Pixels, SharedString, Styled, Window, div, point, px, size};

use crate::domain::{Look, SlideTheme, Transition};
use crate::output::{NamedOutput, OutputContentRoute, OutputType};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{LookLayerType, Message as RootMessage, SettingsTab, ViewMode};
use crate::ui::theme;

/// Messages owned by the Output feature module.
#[derive(Debug, Clone)]
pub enum Message {
    SettingsOpen,
    SettingsClose,
    SelectTab(SettingsTab),
    AddWindow,
    AddNdi,
    Remove(String),
    SetActive(String, bool),
    CycleContent(String),
    CycleMirror(String),
    SetResolution(String, u32, u32),
    /// Choose the screen the placement card edits.
    SelectPlacementTarget(String),
    /// Move the screen being placed to the next display.
    CycleDisplay,
    NewLabelChanged(String),
    NewNdiNameChanged(String),
    SetStartupMode(ViewMode),
    BrowseDbPath,
    RevealDbPath,
    NdiSourceNameChanged(String),
    NdiFrameRateChanged(u32),
    HttpPortChanged(String),
    OscPortChanged(String),
    SetGlobalTransitionType(Transition),
    SetGlobalTransitionDuration(u64),
    ToggleReduceMotion(bool),
    SetDefaultGridCols(usize),
    SaveConfig,
    ResetToDefaults,
    RefreshDisplays,
    ApplyDetectedDisplay(usize),
}

/// Dispatch an output message.
pub(crate) fn update(
    w: &mut MainWindow,
    msg: Message,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    match msg {
        Message::SettingsOpen => open_settings(w, window, cx),
        Message::SettingsClose => close_settings(w, cx),
        Message::SelectTab(tab) => select_tab(w, tab, cx),
        Message::AddWindow => add_window(w, window, cx),
        Message::AddNdi => add_ndi(w, window, cx),
        Message::Remove(id) => remove(w, &id, cx),
        Message::SetActive(id, v) => set_active(w, &id, v, cx),
        Message::CycleContent(id) => cycle_content(w, &id, cx),
        Message::CycleMirror(id) => cycle_mirror(w, &id, cx),
        Message::SetResolution(id, width, height) => set_resolution(w, &id, width, height, cx),
        Message::SelectPlacementTarget(id) => select_placement_target(w, &id, cx),
        Message::CycleDisplay => cycle_display(w, cx),
        Message::NewLabelChanged(s) => new_label_changed(w, s, cx),
        Message::NewNdiNameChanged(s) => new_ndi_name_changed(w, s, cx),
        Message::SetStartupMode(mode) => set_startup_mode(w, mode, cx),
        Message::BrowseDbPath => browse_db_path(w, cx),
        Message::RevealDbPath => reveal_db_path(w, cx),
        Message::NdiSourceNameChanged(s) => ndi_source_name_changed(w, s, cx),
        Message::NdiFrameRateChanged(fps) => ndi_frame_rate_changed(w, fps, cx),
        Message::HttpPortChanged(s) => http_port_changed(w, s, cx),
        Message::OscPortChanged(s) => osc_port_changed(w, s, cx),
        Message::SetGlobalTransitionType(t) => set_global_transition_type(w, t, cx),
        Message::SetGlobalTransitionDuration(d) => set_global_transition_duration(w, d, cx),
        Message::ToggleReduceMotion(v) => toggle_reduce_motion(w, v, cx),
        Message::SetDefaultGridCols(c) => set_default_grid_cols(w, c, cx),
        Message::SaveConfig => save_config(w, cx),
        Message::ResetToDefaults => reset_to_defaults(w, window, cx),
        Message::RefreshDisplays => refresh_displays(w, cx),
        Message::ApplyDetectedDisplay(idx) => apply_detected_display(w, idx, cx),
    }
}

// ---------------------------------------------------------------------------
// Settings modal open/close/tab
// ---------------------------------------------------------------------------

pub(crate) fn open_settings(
    w: &mut MainWindow,
    _window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    // A live settings window is the same window, focused rather than
    // opened again: the operator keeps the one preferences surface they
    // already have.
    if let Some(handle) = w.output.settings_window {
        if crate::ui::settings_window::focus(&handle, cx).is_ok() {
            w.output.settings_open = true;
            w.output.settings_status_message = None;
            cx.notify();
            return;
        }
        // The window the operator closed from the desktop leaves a dead
        // handle behind: opening is what notices and starts a fresh one.
        w.output.settings_window = None;
    }
    // The open draws the new window right away, and that draw updates the
    // main window. Queueing it as an app-level deferred effect keeps it
    // off the current update so the draw never leases the main window
    // twice.
    let main = cx.entity();
    cx.defer(
        move |cx| match crate::ui::settings_window::open(main.clone(), cx) {
            Ok(handle) => {
                main.update(cx, |w, _cx| {
                    w.output.settings_window = Some(handle);
                    w.output.settings_open = true;
                });
            }
            Err(e) => log::error!("[settings] could not open the preferences window: {e}"),
        },
    );
    w.output.settings_open = true;
    w.output.settings_status_message = None;
    cx.notify();
}

pub(crate) fn close_settings(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The settings window closes itself: its next render sees the flag
    // false and removes its own window. Removing a window through its
    // handle from the main context errors ("window not found") when the
    // window is already half-gone, so no removal happens here.
    w.output.settings_open = false;
    w.output.settings_status_message = None;
    cx.notify();
}

pub(crate) fn select_tab(w: &mut MainWindow, tab: SettingsTab, cx: &mut Context<MainWindow>) {
    w.output.settings_tab = tab;
    cx.notify();
}

// ---------------------------------------------------------------------------
// General tab logic
// ---------------------------------------------------------------------------

fn set_startup_mode(w: &mut MainWindow, mode: ViewMode, cx: &mut Context<MainWindow>) {
    w.app_config.general.startup_mode = match mode {
        ViewMode::Show => "show".to_string(),
        ViewMode::Edit => "edit".to_string(),
    };
    persist_config(w);
    w.output.settings_status_message = Some("Startup mode updated".to_string());
    cx.notify();
}

fn browse_db_path(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let dest = rfd::FileDialog::new()
        .set_title("Select OpenPresenter Database")
        .add_filter("SQLite Database", &["db", "sqlite"])
        .pick_file();
    if let Some(path) = dest {
        // The database is moved, not just pointed at: the operator's library
        // has to travel with the setting.
        match move_database(w, &path) {
            Ok(()) => {
                w.app_config.db_path = path;
                persist_config(w);
                w.output.settings_status_message = Some("Database location updated".to_string());
            }
            Err(e) => {
                w.set_error(format!("Could not move the database: {e}"));
            }
        }
    }
    cx.notify();
}

/// Move the live database to a new location, with the prop library beside it.
fn move_database(w: &mut MainWindow, dest: &std::path::Path) -> anyhow::Result<()> {
    let source = w.app_config.db_path.clone();
    if source == dest {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&source, dest)?;

    // The prop library lives beside the database, so it moves with it.
    let props_source = props_path_for(&source);
    if props_source.exists()
        && let Some(parent) = dest.parent()
    {
        let _ = std::fs::create_dir_all(parent);
        let _ = std::fs::copy(&props_source, parent.join("props.json"));
    }
    Ok(())
}

/// The prop library path that belongs to a database.
fn props_path_for(db_path: &std::path::Path) -> std::path::PathBuf {
    db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("props.json")
}

fn reveal_db_path(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let path = w.app_config.db_path.clone();
    let target = if path.exists() {
        path.clone()
    } else if let Some(p) = path.parent() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(".")
    };

    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(&target)
        .spawn();

    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,\"{}\"", target.display()))
        .spawn();

    #[cfg(target_os = "linux")]
    if let Some(parent) = target.parent().or(Some(&target)) {
        let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let _ = target;

    cx.notify();
}

// ---------------------------------------------------------------------------
// Network & NDI tab logic
// ---------------------------------------------------------------------------

pub(crate) fn ndi_source_name_changed(
    w: &mut MainWindow,
    name: String,
    cx: &mut Context<MainWindow>,
) {
    w.app_config.ndi.source_name = name;
    persist_config(w);
    cx.notify();
}

fn ndi_frame_rate_changed(w: &mut MainWindow, fps: u32, cx: &mut Context<MainWindow>) {
    w.app_config.ndi.frame_rate = fps;
    persist_config(w);
    // A running sender keeps the frame rate it was started with, so the
    // senders are restarted to pick up the new one. Otherwise the setting
    // would only take effect the next time the broadcast was started.
    let running: Vec<String> = w.presenting.ndi_outputs.keys().cloned().collect();
    for id in running {
        crate::ui::ndi::stop_ndi_output(w, &id);
        crate::ui::ndi::start_ndi_output(w, &id, cx);
    }
    w.output.settings_status_message = Some(format!("NDI frame rate set to {fps} FPS"));
    cx.notify();
}

/// Edit the HTTP trigger port. The Triggers panel owns the live editing
/// surface for ports; this mirrors the value into `triggers.http_port_str` so
/// both views always agree, and persists it to the config.
pub(crate) fn http_port_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.triggers.http_port_str = s;
    if let Ok(p) = w.triggers.http_port_str.trim().parse::<u16>() {
        w.app_config.triggers.http_port = p;
        persist_config(w);
        w.output.settings_status_message = Some(format!("HTTP trigger port set to {p}"));
    }
    cx.notify();
}

/// OSC counterpart of [`http_port_changed`].
pub(crate) fn osc_port_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.triggers.osc_port_str = s;
    if let Ok(p) = w.triggers.osc_port_str.trim().parse::<u16>() {
        w.app_config.triggers.osc_port = p;
        persist_config(w);
        w.output.settings_status_message = Some(format!("OSC listener port set to {p}"));
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Transitions tab logic
// ---------------------------------------------------------------------------

fn set_global_transition_type(
    w: &mut MainWindow,
    transition: Transition,
    cx: &mut Context<MainWindow>,
) {
    w.presenting.global_transition = transition;
    w.app_config.transitions.default_type = match &w.presenting.global_transition {
        Transition::Cut => "cut".to_string(),
        Transition::Fade { .. } => "fade".to_string(),
        Transition::Push { direction, .. } => format!("push_{direction}").to_lowercase(),
        Transition::Wipe { angle_deg, .. } => format!("wipe_{angle_deg}").to_lowercase(),
        Transition::Zoom { .. } => "zoom".to_string(),
        Transition::Slide { .. } => "slide".to_string(),
        Transition::Dissolve { .. } => "dissolve".to_string(),
        Transition::Flip { .. } => "flip".to_string(),
        Transition::Clock { .. } => "clock".to_string(),
    };
    persist_config(w);
    cx.notify();
}

pub(crate) fn set_global_transition_duration(
    w: &mut MainWindow,
    duration_ms: u64,
    cx: &mut Context<MainWindow>,
) {
    let dur = duration_ms.clamp(50, 5000);
    w.app_config.transitions.default_duration_ms = dur;
    w.presenting.global_transition = w.presenting.global_transition.with_duration(dur);
    persist_config(w);
    cx.notify();
}

/// Slider-side entry point (values arrive as f32 from the slider state).
pub(crate) fn transition_duration_changed(
    w: &mut MainWindow,
    value: f32,
    cx: &mut Context<MainWindow>,
) {
    set_global_transition_duration(w, value as u64, cx);
}

// ---------------------------------------------------------------------------
// Advanced tab logic
// ---------------------------------------------------------------------------

pub(crate) fn toggle_reduce_motion(
    w: &mut MainWindow,
    enabled: bool,
    cx: &mut Context<MainWindow>,
) {
    w.ui.reduce_motion = enabled;
    w.app_config.ui.reduce_motion = enabled;
    persist_config(w);
    cx.notify();
}

pub(crate) fn set_default_grid_cols(w: &mut MainWindow, cols: usize, cx: &mut Context<MainWindow>) {
    let c = cols.clamp(2, 6);
    w.presenting.slide_grid_cols = c;
    w.app_config.ui.default_grid_cols = c;
    persist_config(w);
    cx.notify();
}

/// Slider-side entry point for the default slide-grid column count.
pub(crate) fn grid_cols_changed(w: &mut MainWindow, value: f32, cx: &mut Context<MainWindow>) {
    set_default_grid_cols(w, value as usize, cx);
}

fn save_config(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    match w.app_config.save() {
        Ok(_) => w.output.settings_status_message = Some("Settings saved successfully".to_string()),
        Err(e) => w.set_error(format!("Failed to save settings: {e}")),
    }
    cx.notify();
}

fn reset_to_defaults(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    // The database location is not a setting: the defaults point at a
    // different library, which would make the operator's work look gone.
    let db_path = w.app_config.db_path.clone();
    w.app_config = crate::config::Config::default();
    w.app_config.db_path = db_path;
    // The trigger manager is the running copy of the remote-control
    // settings, so the reset has to reach it: a panel that still listed
    // macros the file no longer had would bring them back on the next edit,
    // and the listeners would keep running on ports the file says are off.
    w.triggers.manager.stop_http();
    w.triggers.manager.stop_osc();
    w.triggers.manager.stop_midi();
    w.triggers.manager.macros.clear();
    w.triggers.manager.http_port = w.app_config.triggers.http_port;
    w.triggers.manager.osc_port = w.app_config.triggers.osc_port;
    w.triggers.manager.http_running = w.app_config.triggers.http_enabled;
    w.triggers.manager.osc_running = w.app_config.triggers.osc_enabled;
    w.triggers.http_port_str = w.app_config.triggers.http_port.to_string();
    w.triggers.osc_port_str = w.app_config.triggers.osc_port.to_string();
    // Every screen goes back to the default place, and the mirrors follow
    // the screen that is being placed.
    let ids: Vec<String> = w.output.manager.iter().map(|o| o.id.clone()).collect();
    for id in ids {
        w.output
            .manager
            .set_placement(&id, crate::output::OutputPlacement::default());
    }
    persist_config(w);
    let target = w.output.placement_target.clone();
    let placement = placement_of(w, &target);
    w.output.screen_x = placement.screen_x.to_string();
    w.output.screen_y = placement.screen_y.to_string();
    w.ui.reduce_motion = false;
    w.presenting.slide_grid_cols = 4;
    w.presenting.global_transition = Transition::Fade { duration_ms: 500 };
    w.output.settings_status_message = Some("Reset all settings to factory defaults".to_string());
    // The settings inputs and sliders mirror programmatic changes on the next
    // render via [`sync_inputs`]; typing state is dropped with the modal's
    // free-form fields here so a re-open starts clean.
    sync_inputs(w, window, cx);
    cx.notify();
}

/// Persist the config, surfacing failures as an error toast.
pub(crate) fn persist_config(w: &mut MainWindow) {
    // The outputs are the operator's environment — their destinations, their
    // routes and their resolutions — so they are saved with the rest of the
    // settings and survive a restart.
    w.app_config.outputs = w.output.manager.iter().cloned().collect();
    if let Err(e) = w.app_config.save() {
        w.set_error(format!("Failed to save settings: {e}"));
    }
}

// ---------------------------------------------------------------------------
// Detected displays
// ---------------------------------------------------------------------------

fn refresh_displays(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.output.detected_displays = crate::output::detect_displays();
    w.output.settings_status_message = Some(format!(
        "Detected {} display(s)",
        w.output.detected_displays.len()
    ));
    cx.notify();
}

fn apply_detected_display(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    if let Some(disp) = w.output.detected_displays.get(idx) {
        let (name, x, y, dw, dh) = (disp.name.clone(), disp.x, disp.y, disp.width, disp.height);
        // The display's own position is where the window belongs, so the
        // offset within it is zero. Storing the absolute position and then
        // adding the display's origin again put the window off the
        // projector.
        let display_index = cx
            .displays()
            .iter()
            .position(|display| {
                let origin = display.bounds().origin;
                f32::from(origin.x).round() as i32 == x && f32::from(origin.y).round() as i32 == y
            })
            .or(Some(idx));

        // The screen being placed moves onto this display.
        let id = w.output.placement_target.clone();
        let mut placement = placement_of(w, &id);
        placement.display_index = display_index;
        placement.screen_x = 0.0;
        placement.screen_y = 0.0;
        w.output.manager.set_placement(&id, placement);
        mirror_placement(w, &id);
        w.output.screen_x = "0".to_string();
        w.output.screen_y = "0".to_string();

        // The screen is drawn at the display's resolution. The audience
        // screen follows the operator's global output resolution, so it
        // needs no resolution of its own; the other screens carry one.
        if id == AUDIENCE_SCREEN_ID || id == "main" {
            w.app_config.output.width = dw;
            w.app_config.output.height = dh;
        } else {
            w.output.manager.set_resolution(&id, dw, dh);
        }
        persist_config(w);
        // The screen moves onto the display the operator applied.
        reopen_window_for_output(&id, w, cx);
        let status = format!("Applied display '{name}' ({dw}×{dh} at {x},{y})");
        w.output.settings_status_message = Some(status);
    }
    cx.notify();
}

// ---------------------------------------------------------------------------
// Screens matrix / output destinations
// ---------------------------------------------------------------------------

fn add_window(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let label = w.output.new_label.trim().to_string();
    if label.is_empty() {
        return;
    }
    let id = label.to_lowercase().replace(' ', "_");
    // The new output follows the operator's global output resolution until
    // a resolution of its own is picked.
    let added = w
        .output
        .manager
        .add(NamedOutput::new_window(id, label.clone()));
    if !added {
        w.set_error(format!(
            "Could not add '{label}': the limit of {} outputs is reached",
            crate::output::OutputManager::MAX_OUTPUTS
        ));
        return;
    }
    w.output.new_label.clear();
    clear_input(&w.output_label_input, window, cx);
    persist_config(w);
    cx.notify();
}

fn add_ndi(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    let label = w.output.new_label.trim().to_string();
    if label.is_empty() {
        return;
    }
    let stream_name = if w.output.new_ndi_name.trim().is_empty() {
        label.clone()
    } else {
        w.output.new_ndi_name.trim().to_string()
    };
    let id = label.to_lowercase().replace(' ', "_") + "_ndi";
    // The new output follows the operator's global output resolution until
    // a resolution of its own is picked.
    let added = w
        .output
        .manager
        .add(NamedOutput::new_ndi(id, label.clone(), stream_name));
    if !added {
        w.set_error(format!(
            "Could not add '{label}': the limit of {} outputs is reached",
            crate::output::OutputManager::MAX_OUTPUTS
        ));
        return;
    }
    w.output.new_label.clear();
    w.output.new_ndi_name.clear();
    clear_input(&w.output_label_input, window, cx);
    clear_input(&w.output_ndi_name_input, window, cx);
    persist_config(w);
    cx.notify();
}

fn remove(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    // The built-in screens are the app's own: the operator's window, the
    // projector, the broadcast and the stage monitor. Removing one of them
    // leaves the app's own controls pointing at a screen that is not there,
    // and the control does nothing without saying so.
    if is_builtin_output(id) {
        return;
    }
    w.output.manager.remove(id);
    persist_config(w);
    sync_output_windows(w, cx);
    cx.notify();
}

fn set_active(w: &mut MainWindow, id: &str, active: bool, cx: &mut Context<MainWindow>) {
    w.output.manager.set_active(id, active);
    persist_config(w);
    sync_output_windows(w, cx);
    cx.notify();
}

fn cycle_content(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    let next = w
        .output
        .manager
        .get(id)
        .map(|output| match &output.content {
            OutputContentRoute::LiveSlide => OutputContentRoute::Stage,
            OutputContentRoute::Stage => OutputContentRoute::Blank,
            OutputContentRoute::Blank => OutputContentRoute::LiveSlide,
            OutputContentRoute::Mirror { .. } => OutputContentRoute::LiveSlide,
        });
    if let Some(next) = next {
        w.output.manager.set_content(id, next);
        // The window of an output is opened for the kind of screen it shows,
        // so a route that changes the kind needs a new window.
        rebind_output_window(w, id, cx);
        persist_config(w);
        sync_output_windows(w, cx);
    }
    cx.notify();
}

/// Point an output at another output's picture.
///
/// The button cycles through the other outputs and then back to the live
/// slide, so the operator can turn a second monitor into a confidence
/// display that shows exactly what the projector shows.
fn cycle_mirror(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    // The other outputs, in the order they are listed, are the candidates.
    let candidates: Vec<String> = w
        .output
        .manager
        .iter()
        .filter(|output| output.id != id)
        .map(|output| output.id.clone())
        .collect();

    let next = if candidates.is_empty() {
        Some(OutputContentRoute::LiveSlide)
    } else {
        w.output.manager.get(id).map(|output| {
            let current = match &output.content {
                OutputContentRoute::Mirror { source_id } => {
                    candidates.iter().position(|c| c == source_id)
                }
                _ => None,
            };
            match current {
                Some(index) => {
                    let next_index = index + 1;
                    if next_index < candidates.len() {
                        OutputContentRoute::Mirror {
                            source_id: candidates[next_index].clone(),
                        }
                    } else {
                        OutputContentRoute::LiveSlide
                    }
                }
                None => OutputContentRoute::Mirror {
                    source_id: candidates[0].clone(),
                },
            }
        })
    };

    if let Some(next) = next {
        w.output.manager.set_content(id, next);
        rebind_output_window(w, id, cx);
        persist_config(w);
        sync_output_windows(w, cx);
    }
    cx.notify();
}

/// Re-open an output's window after its content route changed.
///
/// The window is opened for the kind of screen the output shows: a live
/// slide gets an output window, the stage gets the stage monitor's window.
/// A route that changes the kind therefore needs a new window, or the
/// projector would show an empty box where the slide was.
fn rebind_output_window(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    if w.output.windows.contains_key(id) {
        // The next sync opens it again, for the kind the output now shows.
        close_window_for_output(id, w, cx);
    }
}

fn set_resolution(
    w: &mut MainWindow,
    id: &str,
    width: u32,
    height: u32,
    cx: &mut Context<MainWindow>,
) {
    // The audience and the main output are rendered at the resolution the
    // operator picked, so the setting has to reach the configuration the
    // compositor, the canvas, the broadcast and the recorder read. They
    // keep following it, so they carry no resolution of their own.
    if id == AUDIENCE_SCREEN_ID || id == "main" {
        w.app_config.output.width = width;
        w.app_config.output.height = height;
        persist_config(w);
    } else {
        // Every other output is composed at the resolution it is given, so
        // a second projector can run at 720p while the first runs at 1080p.
        w.output.manager.set_resolution(id, width, height);
    }
    // The window is as big as the picture it shows, so an open window has
    // to follow the resolution it was given.
    resize_output_window(w, id, cx);
    // A running sender keeps the resolution it was started with, so it has
    // to be restarted: the row would say one size while the stream sent
    // another.
    sync_output_windows(w, cx);
    persist_config(w);
    cx.notify();
}

/// Resize an output's open window to the resolution it is composed at.
fn resize_output_window(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    let Some(handle) = w.output.windows.get(id).copied() else {
        return;
    };
    let (width, height) = crate::ui::compositor::screen_resolution(w, id);
    let _ = cx.update_window(handle, |_view, window, _cx| {
        window.resize(size(px(width as f32), px(height as f32)));
    });
}

pub(crate) fn new_label_changed(w: &mut MainWindow, label: String, cx: &mut Context<MainWindow>) {
    w.output.new_label = label;
    cx.notify();
}

pub(crate) fn new_ndi_name_changed(w: &mut MainWindow, name: String, cx: &mut Context<MainWindow>) {
    w.output.new_ndi_name = name;
    cx.notify();
}

// ---------------------------------------------------------------------------
// Output window
// ---------------------------------------------------------------------------

/// The id of the audience screen: the output the operator sends to the
/// projector.
pub(crate) const AUDIENCE_SCREEN_ID: &str = "audience";

/// Whether an output is one of the app's own screens.
///
/// The main window, the projector, the broadcast and the stage monitor are
/// the app's own: the operator's controls drive them, so they cannot be
/// removed.
pub(crate) fn is_builtin_output(id: &str) -> bool {
    matches!(
        id,
        "main" | AUDIENCE_SCREEN_ID | "stream" | crate::ui::stage::STAGE_SCREEN_ID
    )
}

/// The ids of every screen in the output manager.
pub(crate) fn screen_ids(w: &MainWindow) -> Vec<String> {
    w.output
        .manager
        .iter()
        .map(|output| output.id.clone())
        .collect()
}

/// Open the dedicated audience output window.
///
/// The window is placed on the configured display (`app_config.output`)
/// and shows the composited slide, so the audience sees exactly what the
/// operator previews and what the NDI sender transmits.
pub(crate) fn open_output_window(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if output_window_is_open(w, cx) {
        return;
    }
    // The audience screen is an entry in the output manager like any other,
    // so the operator can configure it like one.
    w.output.manager.set_active(AUDIENCE_SCREEN_ID, true);
    sync_output_windows(w, cx);
    cx.notify();
}

/// Close the dedicated audience output window and reset its operator state.
pub(crate) fn close_output_window(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.output.manager.set_active(AUDIENCE_SCREEN_ID, false);
    sync_output_windows(w, cx);
    // The black goes with the window: the broadcast was blacked with the
    // audience, so it comes back with it. Leaving the stream blacked would
    // leave the "Live" switch dark while the stream is live, and pressing it
    // would black the stream once more instead of restoring it.
    if w.output.black_screen {
        crate::ui::ndi::set_black_screen(w, false, cx);
    }
    w.output.black_screen = false;
    w.output.is_fullscreen = false;
    cx.notify();
}

/// Open the real window for one of the output manager's screens.
///
/// Every window output in the manager can have a window: that is what makes
/// a second projector or a second screen possible.
fn open_window_for_output(id: &str, w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if w.output.windows.contains_key(id) {
        return;
    }
    let Some(output) = w.output.manager.get(id).cloned() else {
        return;
    };
    if !matches!(output.output_type, OutputType::Window) || id == "main" {
        return;
    }

    // The stage route has its own window: it carries the notes and the next
    // slide, which do not fit into a plain output window.
    let routes = w.output.route_slot.read();
    let kind = output.content.resolve(&routes);
    if kind == crate::output::ScreenKind::Stage {
        let slot = Arc::clone(&w.presenting.stage_slot);
        let main = cx.entity();
        let bounds = output_bounds(w, id, cx);
        match crate::ui::stage_window::open(slot, main, bounds, cx) {
            Ok(handle) => {
                w.output.windows.insert(id.to_string(), handle);
                log::info!("[output] the '{id}' stage window is open");
            }
            Err(e) => log::error!("[output] could not open the '{id}' window: {e}"),
        }
        return;
    }

    let bounds = output_bounds(w, id, cx);
    let slot = Arc::clone(&w.compositor.slot);
    let route_slot = Arc::clone(&w.output.route_slot);
    let screen_id = id.to_string();
    // The window is fullscreen when the operator put it there; the setting
    // only decides how it opens.
    let fullscreen = id == AUDIENCE_SCREEN_ID && w.output.is_fullscreen;
    let main = cx.entity();

    match crate::ui::output_window::open(slot, route_slot, screen_id, main, bounds, fullscreen, cx)
    {
        Ok((handle, _entity)) => {
            if id == AUDIENCE_SCREEN_ID {
                w.output.window_id = Some(handle.window_id());
            }
            w.output.windows.insert(id.to_string(), handle);
            log::info!("[output] the '{id}' window is open");
        }
        Err(e) => {
            log::error!("[output] could not open the '{id}' window: {e}");
            w.set_error(format!("Could not open the output window: {e}"));
        }
    }
}

/// Close the real window of one of the output manager's screens.
fn close_window_for_output(id: &str, w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(handle) = w.output.windows.remove(id) else {
        return;
    };
    if id == AUDIENCE_SCREEN_ID {
        w.output.window_id = None;
    }
    // `update` fails when the window was already closed by the operator.
    let _ = handle.update(cx, |_view, window, _cx| window.remove_window());
}

/// Bring the real windows in line with the output manager.
///
/// Every active window output gets a window; every window whose output was
/// deactivated or removed is closed.
pub(crate) fn sync_output_windows(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The routes are read by the windows to know what to show.
    let routes: HashMap<String, OutputContentRoute> = w
        .output
        .manager
        .iter()
        .map(|output| (output.id.clone(), output.content.clone()))
        .collect();
    w.output.route_slot.publish(routes);

    let wanted: Vec<String> = w
        .output
        .manager
        .iter()
        .filter(|output| {
            // The main output is the operator's own window; it never gets a
            // window of its own.
            output.id != "main" && output.active && matches!(output.output_type, OutputType::Window)
        })
        .map(|output| output.id.clone())
        .collect();

    // Close the windows whose output is no longer wanted.
    let stale: Vec<String> = w
        .output
        .windows
        .keys()
        .filter(|id| !wanted.contains(id))
        .cloned()
        .collect();
    for id in stale {
        close_window_for_output(&id, w, cx);
    }

    // Open the windows that are missing.
    for id in wanted {
        if !w.output.windows.contains_key(&id) {
            open_window_for_output(&id, w, cx);
        }
    }

    // The broadcasts run with the outputs: every active NDI output has a
    // sender of its own, bound to its own screen.
    let ndi_outputs: Vec<(String, bool)> = w
        .output
        .manager
        .iter()
        .filter(|output| matches!(output.output_type, OutputType::Ndi { .. }))
        .map(|output| (output.id.clone(), output.active))
        .collect();

    // An output the operator deleted leaves its sender behind: the manager
    // no longer knows the id, so the loop below would never stop it and the
    // broadcast would keep running after the output was gone.
    let managed: Vec<String> = w
        .output
        .manager
        .iter()
        .map(|output| output.id.clone())
        .collect();
    let orphans: Vec<String> = w
        .presenting
        .ndi_outputs
        .keys()
        .filter(|id| !managed.contains(id))
        .cloned()
        .collect();
    for id in orphans {
        crate::ui::ndi::stop_ndi_output(w, &id);
    }

    for (id, active) in ndi_outputs {
        let running = w.presenting.ndi_outputs.contains_key(&id);
        if active && !running {
            crate::ui::ndi::start_ndi_output(w, &id, cx);
        } else if !active && running {
            crate::ui::ndi::stop_ndi_output(w, &id);
        } else if active && running {
            // The broadcast follows the resolution of its own output, and
            // the pipeline is built with it: a changed resolution has to
            // reach the sender, or the stream stays at the size it was
            // started with while the projector has moved on.
            let (width, height) = crate::ui::compositor::screen_resolution(w, &id);
            let current = w
                .presenting
                .ndi_outputs
                .get(&id)
                .map(|ndi| ndi.resolution());
            if current != Some((width, height)) {
                crate::ui::ndi::stop_ndi_output(w, &id);
                crate::ui::ndi::start_ndi_output(w, &id, cx);
            }
        }
    }
}

/// Toggle the dedicated audience output window (open when closed, close
/// otherwise).
pub(crate) fn toggle_output_window(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if output_window_is_open(w, cx) {
        close_output_window(w, cx);
    } else {
        // The operator may have closed the window from the desktop; drop the
        // stale handle before opening a new one.
        w.output.windows.remove(AUDIENCE_SCREEN_ID);
        w.output.window_id = None;
        open_output_window(w, cx);
    }
}

/// Whether the audience output window is still open.
///
/// The operator can close the window from the desktop, which leaves the
/// handle behind; a failed read means the window is gone.
fn output_window_is_open(w: &MainWindow, cx: &mut Context<MainWindow>) -> bool {
    let Some(handle) = w.output.windows.get(AUDIENCE_SCREEN_ID) else {
        return false;
    };
    cx.update_window(*handle, |_view, _window, _app| ()).is_ok()
}

/// Toggle fullscreen on the dedicated output window.
///
/// The window is reopened in the new mode, because the platform owns the
/// fullscreen transition.
pub(crate) fn output_fullscreen_toggled(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if !output_window_is_open(w, cx) {
        return;
    }
    // The platform owns the fullscreen transition, so the window is reopened
    // in the new mode.
    w.output.is_fullscreen = !w.output.is_fullscreen;
    // The window is closed without resetting the mode, so the reopen uses
    // the one the operator just chose.
    w.output.manager.set_active(AUDIENCE_SCREEN_ID, false);
    sync_output_windows(w, cx);
    w.output.manager.set_active(AUDIENCE_SCREEN_ID, true);
    sync_output_windows(w, cx);
    cx.notify();
}

/// Where an output's window is placed on the desktop.
///
/// The placement belongs to the output, so a second projector goes to the
/// display the operator picked for it instead of landing on top of the
/// first one. The window is as big as the picture it shows, so an output
/// with a resolution of its own gets a window of that size.
fn output_bounds(w: &MainWindow, id: &str, cx: &mut Context<MainWindow>) -> Bounds<Pixels> {
    let (width, height) = crate::ui::compositor::screen_resolution(w, id);
    let placement = placement_of(w, id);

    let displays = cx.displays();
    let display = placement
        .display_index
        .and_then(|index| displays.get(index))
        .or_else(|| displays.first());

    let origin = match display {
        Some(display) => {
            let bounds = display.bounds();
            point(
                px(f32::from(bounds.origin.x) + placement.screen_x),
                px(f32::from(bounds.origin.y) + placement.screen_y),
            )
        }
        None => point(px(placement.screen_x), px(placement.screen_y)),
    };

    Bounds {
        origin,
        size: size(px(width as f32), px(height as f32)),
    }
}

/// The placement of an output, or the default one when it has none.
fn placement_of(w: &MainWindow, id: &str) -> crate::output::OutputPlacement {
    w.output.manager.placement(id).unwrap_or_default()
}

/// Mirror the audience screen's placement into the global settings.
///
/// The audience screen is the projector, and its placement has always lived
/// in the global settings. The mirror keeps a configuration file written by
/// an older build describing where the projector is.
fn mirror_placement(w: &mut MainWindow, id: &str) {
    if id != AUDIENCE_SCREEN_ID {
        return;
    }
    if let Some(placement) = w.output.manager.placement(id) {
        w.app_config.output.display_index = placement.display_index;
        w.app_config.output.screen_x = placement.screen_x;
        w.app_config.output.screen_y = placement.screen_y;
    }
}

/// Move the screen being placed and save the result.
fn apply_placement(w: &mut MainWindow, placement: crate::output::OutputPlacement) {
    let id = w.output.placement_target.clone();
    w.output.manager.set_placement(&id, placement);
    mirror_placement(w, &id);
    persist_config(w);
}

/// Reopen a screen's window, so a new placement takes effect.
///
/// The platform decides where a window sits when it is opened, so a screen
/// that was moved has to be opened again at its new place.
fn reopen_window_for_output(id: &str, w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if !w.output.windows.contains_key(id) {
        return;
    }
    close_window_for_output(id, w, cx);
    open_window_for_output(id, w, cx);
}

/// Commit the placement of the screen being placed.
///
/// The window is reopened once the operator is done typing, so a half-typed
/// coordinate never moves the projector around.
pub(crate) fn commit_placement(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let id = w.output.placement_target.clone();
    reopen_window_for_output(&id, w, cx);
    cx.notify();
}

/// Choose the screen the placement card edits.
fn select_placement_target(w: &mut MainWindow, id: &str, cx: &mut Context<MainWindow>) {
    if w.output.manager.get(id).is_none() {
        return;
    }
    w.output.placement_target = id.to_string();
    // The inputs show the placement of the screen that was just picked.
    let placement = placement_of(w, id);
    w.output.screen_x = placement.screen_x.to_string();
    w.output.screen_y = placement.screen_y.to_string();
    cx.notify();
}

/// Move the screen being placed to the next display.
fn cycle_display(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let count = cx.displays().len();
    if count == 0 {
        return;
    }
    let id = w.output.placement_target.clone();
    let mut placement = placement_of(w, &id);
    placement.display_index = match placement.display_index {
        Some(index) if index + 1 < count => Some(index + 1),
        Some(_) => None,
        None => Some(0),
    };
    apply_placement(w, placement);
    // The window moves to the display the operator picked.
    reopen_window_for_output(&id, w, cx);
    cx.notify();
}

pub(crate) fn output_screen_x_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    if let Ok(x) = v.parse::<f32>() {
        let mut placement = placement_of(w, &w.output.placement_target.clone());
        placement.screen_x = x;
        apply_placement(w, placement);
    }
    w.output.screen_x = v;
    cx.notify();
}

pub(crate) fn output_screen_y_changed(w: &mut MainWindow, v: String, cx: &mut Context<MainWindow>) {
    if let Ok(y) = v.parse::<f32>() {
        let mut placement = placement_of(w, &w.output.placement_target.clone());
        placement.screen_y = y;
        apply_placement(w, placement);
    }
    w.output.screen_y = v;
    cx.notify();
}

pub(crate) fn output_auto_fullscreen_toggled(
    w: &mut MainWindow,
    enabled: bool,
    cx: &mut Context<MainWindow>,
) {
    w.app_config.output.auto_fullscreen = enabled;
    // The setting decides how the window opens, and the window follows it
    // right away.
    w.output.is_fullscreen = enabled;
    persist_config(w);
    cx.notify();
}

// ---------------------------------------------------------------------------
// Looks & Screens matrix logic
// ---------------------------------------------------------------------------

pub(crate) fn look_name_input_changed(
    w: &mut MainWindow,
    val: String,
    cx: &mut Context<MainWindow>,
) {
    w.output.editing_look_name = val;
    cx.notify();
}

pub(crate) fn select_look(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    // Making a look live brings its mask and its props with it: the look
    // is the whole picture the operator saved, not only its name.
    w.props.manager.apply_look(&id);
    w.output.selected_look_id = Some(id);
    w.save_props();
    // The broadcast composites the slide itself, so it has to be told about
    // the new look.
    crate::ui::props::push_overlay(w);
    cx.notify();
}

pub(crate) fn create_look(
    w: &mut MainWindow,
    name: String,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let look = Look::with_default_screens(name);
    let id = look.id.clone();
    w.props.manager.add_look(look);
    // The new look goes live with its mask and its props.
    w.props.manager.apply_look(&id);
    w.output.selected_look_id = Some(id);
    w.output.editing_look_name.clear();
    clear_input(&w.look_name_input, window, cx);
    w.save_props();
    crate::ui::props::push_overlay(w);
    cx.notify();
}

pub(crate) fn delete_look(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    // The domain's own rule: removing the live look takes everything it put
    // on the output with it. A look that is deleted while it is live would
    // otherwise leave its lower third and its mask on the projector, under
    // another look that is marked LIVE.
    w.props.manager.remove_look(&id);
    if w.output.selected_look_id.as_deref() == Some(&id) {
        w.output.selected_look_id = w.props.manager.active_look_id.clone();
    }
    w.save_props();
    // The live look changed, so the broadcast has to be told.
    crate::ui::props::push_overlay(w);
    cx.notify();
}

pub(crate) fn toggle_look_screen_layer(
    w: &mut MainWindow,
    look_id: String,
    screen_id: String,
    layer_type: LookLayerType,
    cx: &mut Context<MainWindow>,
) {
    if let Some(look) = w.props.manager.looks.iter_mut().find(|l| l.id == look_id) {
        let mut target = look.target_or_default(&screen_id);
        match layer_type {
            LookLayerType::Media => target.media_enabled = !target.media_enabled,
            LookLayerType::Slide => target.slide_enabled = !target.slide_enabled,
            LookLayerType::Props => target.props_enabled = !target.props_enabled,
            LookLayerType::Messages => target.messages_enabled = !target.messages_enabled,
            LookLayerType::Mask => target.mask_enabled = !target.mask_enabled,
        }
        look.set_screen_target(target);
        w.save_props();
    }
    // The screen's theme changed, so the broadcast has to be told.
    crate::ui::props::push_overlay(w);
    cx.notify();
}

pub(crate) fn set_look_screen_theme(
    w: &mut MainWindow,
    look_id: String,
    screen_id: String,
    theme_id: Option<String>,
    cx: &mut Context<MainWindow>,
) {
    if let Some(look) = w.props.manager.looks.iter_mut().find(|l| l.id == look_id) {
        let mut target = look.target_or_default(&screen_id);
        target.theme_id = theme_id;
        look.set_screen_target(target);
        w.save_props();
    }
    // The screen's theme changed, so the broadcast has to be told.
    crate::ui::props::push_overlay(w);
    cx.notify();
}

/// Cycle through the theme-override chain for one screen row:
/// none → first theme → … → last theme → none. Returns `None` when no themes
/// exist (the override stays cleared).
fn cycle_theme(current_id: Option<&str>, themes: &[SlideTheme]) -> Option<String> {
    if themes.is_empty() {
        return None;
    }
    match current_id {
        None => themes.first().map(|t| t.id.clone()),
        Some(id) => {
            if let Some(idx) = themes.iter().position(|t| t.id == id) {
                if idx + 1 < themes.len() {
                    Some(themes[idx + 1].id.clone())
                } else {
                    None
                }
            } else {
                themes.first().map(|t| t.id.clone())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Input sync (called from `MainWindow::render`)
// ---------------------------------------------------------------------------

/// Mirror programmatic state changes into the output feature's input entities.
/// Typing keeps entity and state equal (subscriptions mirror the value), so
/// the diverged check never resets the cursor mid-edit.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    sync_text(&w.output_screen_x_input, &w.output.screen_x, window, cx);
    sync_text(&w.output_screen_y_input, &w.output.screen_y, window, cx);
    sync_text(
        &w.ndi_source_input,
        &w.app_config.ndi.source_name,
        window,
        cx,
    );
    // The slider's range is fixed at construction (50–2000); clamping the
    // synced value keeps the thumb stable for configs stored above the range
    // without fighting set_value every frame.
    let duration = w.presenting.global_transition.duration_ms() as f32;
    sync_slider(
        &w.transition_duration_slider,
        duration.clamp(50., 2000.),
        window,
        cx,
    );
    sync_slider(
        &w.default_grid_cols_slider,
        w.presenting.slide_grid_cols as f32,
        window,
        cx,
    );
}

fn sync_text(
    entity: &gpui_kit::Entity<gpui_kit::component::input::InputState>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    crate::ui::components::sync_text_field(entity, value, window, cx);
}

fn sync_slider(
    entity: &gpui_kit::Entity<gpui_kit::component::slider::SliderState>,
    value: f32,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    let current = entity.read(cx).value().start();
    if (current - value).abs() > 0.0001 {
        entity.update(cx, |state, slider_cx| {
            state.set_value(value, window, slider_cx);
        });
    }
}

fn clear_input(
    entity: &gpui_kit::Entity<gpui_kit::component::input::InputState>,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    entity.update(cx, |state, input_cx| {
        state.set_value("", window, input_cx);
    });
}

// ===========================================================================
// Settings ("Preferences") window
// ===========================================================================

/// Render the global preferences card (opened from the navbar gear or
/// `RootMessage::Output(SettingsOpen)`). A 780×580 centred card with a header,
/// tab bar wrapping across two rows, scrollable tab body, and a footer with
/// the config path and save/close actions.
///
/// The card lives in its own window (`settings_window`), the way the
/// reference opens its Preferences; inside that window the card owns its
/// size and the stop-propagation rules.
pub(crate) fn settings_card(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let current_tab = w.output.settings_tab;
    let status = w.output.settings_status_message.clone();

    let mut header = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .child(
            Icon::new(IconName::Settings)
                .text_color(theme::c(theme::ACCENT_ORANGE))
                .with_size(px(16.)),
        )
        .child(
            div()
                .text_size(px(15.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child("PREFERENCES"),
        );

    if let Some(msg) = status {
        header = header.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .py(px(3.))
                .px(px(8.))
                .rounded(px(4.))
                .border_1()
                .border_color(theme::c(theme::LIVE_GREEN).opacity(0.4))
                .bg(theme::c(theme::LIVE_GREEN).opacity(0.12))
                .child(
                    Icon::new(IconName::CircleCheck)
                        .text_color(theme::c(theme::LIVE_GREEN))
                        .with_size(px(11.)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::LIVE_GREEN))
                        .child(msg),
                ),
        );
    }

    let header = header
        .child(div().flex_1())
        .child(
            Button::new("settings-close")
                .icon(IconName::Close)
                .ghost()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Output(Message::SettingsClose), window, cx);
                })),
        )
        .px(px(14.))
        .py(px(8.));

    // The tab bar holds every page, so it wraps: a page that is off the
    // bottom of the window is a page the operator cannot reach.
    let mut tab_bar = div()
        .w_full()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(4.))
        .px(px(14.))
        .py(px(4.));
    for tab in SettingsTab::ALL {
        tab_bar = tab_bar.child(settings_tab_btn(tab, current_tab, cx));
    }

    let body: Div = crate::ui::settings::page(w, current_tab, cx);

    let config_path_str = crate::config::Config::config_path()
        .to_string_lossy()
        .into_owned();

    let card = div()
        .id("settings-card")
        .test_support()
        .bg(theme::c(theme::BG_PANEL))
        // The Preferences window is its own window, so the card fills
        // whatever surface the window gives it. A small card centered in a
        // large window is a window that looks unfinished.
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(6.))
        .overflow_hidden()
        // A click inside the card belongs to the card, not the backdrop.
        // The backdrop closes on mouse-down, so without both handlers the
        // backdrop closes the settings window before the control inside the
        // card sees its click.
        .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .on_click(|_, _, cx| {
            cx.stop_propagation();
        })
        .child(header)
        .child(tab_bar)
        .child(div().w_full().h(px(1.)).bg(theme::c(theme::BORDER_PANEL)))
        .child(
            div()
                .id("settings-body")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(body),
        )
        .child(div().w_full().h(px(1.)).bg(theme::c(theme::BORDER_PANEL)))
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .px(px(14.))
                .py(px(10.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            Icon::new(IconName::FileText)
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .with_size(px(11.)),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .child(format!("Config: {config_path_str}")),
                        ),
                )
                .child(div().flex_1())
                .child(
                    Button::new("settings-save")
                        .label("Save Settings")
                        .primary()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(RootMessage::Output(Message::SaveConfig), window, cx);
                        })),
                )
                .child(
                    Button::new("settings-done")
                        .label("Close")
                        .ghost()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                RootMessage::Output(Message::SettingsClose),
                                window,
                                cx,
                            );
                        })),
                ),
        );

    // The settings card fills the settings window, like the
    // Preferences card the reference centers. Clicks inside the card stop
    // before they reach the root, so tab switches and controls never act as
    // backdrops.
    div().size_full().child(card)
}

fn settings_tab_btn(
    tab: SettingsTab,
    current: SettingsTab,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let label = tab.label();
    let is_active = tab == current;
    div()
        .id(SharedString::from(format!(
            "settings-tab-{}",
            match tab {
                SettingsTab::General => "general",
                SettingsTab::Audio => "audio",
                SettingsTab::Devices => "devices",
                SettingsTab::GroupsAndLabels => "groups",
                SettingsTab::Import => "import",
                SettingsTab::Inputs => "inputs",
                SettingsTab::Network => "network",
                SettingsTab::Screens => "screens",
                SettingsTab::Updates => "updates",
                SettingsTab::Media => "media",
                SettingsTab::SlideLabels => "slide-labels",
                SettingsTab::Transitions => "transitions",
                SettingsTab::Advanced => "advanced",
            }
        )))
        .test_support()
        .flex()
        .flex_row()
        .items_center()
        .py(px(6.))
        .px(px(12.))
        .rounded(px(5.))
        .text_size(px(12.))
        .text_color(if is_active {
            theme::c(theme::TEXT_PRIMARY)
        } else {
            theme::c(theme::TEXT_SECONDARY)
        })
        .bg(if is_active {
            theme::c(theme::ACCENT_ORANGE).opacity(0.16)
        } else {
            gpui_kit::transparent_black()
        })
        .border_1()
        .border_color(if is_active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(RootMessage::Output(Message::SelectTab(tab)), window, cx);
        }))
        .child(label)
}

/// Titled section card used across the settings tabs.
fn section_card(title: &'static str, content: Div) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(12.))
        .bg(theme::c(theme::BG_PANEL))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .rounded(px(6.))
        .child(
            div().flex().flex_row().items_center().gap(px(6.)).child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::ACCENT_ORANGE))
                    .child(title),
            ),
        )
        .child(content)
}

fn helper_text(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child(text.into())
}

fn description_text(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .child(text.into())
}

// ---------------------------------------------------------------------------
// General tab
// ---------------------------------------------------------------------------

pub(crate) fn tab_general(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let mode_str = w.app_config.general.startup_mode.clone();

    let startup_card = section_card(
        "STARTUP WORKSPACE",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(description_text(
                "Choose which workspace opens when launching OpenPresenter:",
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(mode_picker_btn(
                        "startup-show",
                        "Show (Live Presentation)",
                        ViewMode::Show,
                        mode_str == "show",
                        cx,
                    ))
                    .child(mode_picker_btn(
                        "startup-edit",
                        "Edit (Slide Editor)",
                        ViewMode::Edit,
                        mode_str == "edit",
                        cx,
                    )),
            ),
    );

    let db_path_str = w.app_config.db_path.to_string_lossy().into_owned();
    let presentations = w.editor.presentations.len();
    let songs = w.song.songs.len();
    let assets = w.library.assets.len();

    let db_card = section_card(
        "DATABASE & LIBRARY STORAGE",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(description_text(
                "SQLite storage file holding all presentations, songs, playlists, and metadata:",
            ))
            .child(
                div()
                    .w_full()
                    .py(px(6.))
                    .px(px(10.))
                    .rounded(px(4.))
                    .bg(theme::c(theme::BG_DARKEST))
                    .border_1()
                    .border_color(theme::c(theme::BORDER_PANEL))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_PRIMARY))
                    .child(db_path_str),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        Button::new("db-browse")
                            .label("Change Location...")
                            .secondary()
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(RootMessage::Output(Message::BrowseDbPath), window, cx);
                            })),
                    )
                    .child(
                        Button::new("db-reveal")
                            .label("Reveal in File Manager")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(RootMessage::Output(Message::RevealDbPath), window, cx);
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.))
                            .child(badge_pill("badge-sqlite", "SQLite Connected", theme::LIVE_GREEN))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::c(theme::TEXT_MUTED))
                                    .child(format!(
                                        "{presentations} Presentations • {songs} Songs • {assets} Media Assets"
                                    )),
                            ),
                    ),
            ),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(14.))
        .child(startup_card)
        .child(db_card)
}

fn mode_picker_btn(
    id: &'static str,
    label: &'static str,
    mode: ViewMode,
    is_active: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .py(px(6.))
        .px(px(12.))
        .rounded(px(4.))
        .text_size(px(11.))
        .text_color(if is_active {
            theme::c(theme::TEXT_PRIMARY)
        } else {
            theme::c(theme::TEXT_SECONDARY)
        })
        .bg(if is_active {
            theme::c(theme::ACCENT_ORANGE).opacity(0.12)
        } else {
            gpui_kit::transparent_black()
        })
        .border_1()
        .border_color(if is_active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::Output(Message::SetStartupMode(mode)),
                window,
                cx,
            );
        }))
        .child(if is_active {
            Icon::new(IconName::CircleCheck)
                .text_color(theme::c(theme::ACCENT_ORANGE))
                .with_size(px(11.))
                .into_any_element()
        } else {
            div()
                .w(px(11.))
                .h(px(11.))
                .rounded_full()
                .border_1()
                .border_color(theme::c(theme::TEXT_MUTED))
                .into_any_element()
        })
        .child(label)
}

// ---------------------------------------------------------------------------
// Screens tab
// ---------------------------------------------------------------------------

pub(crate) fn tab_screens(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    // --- detected hardware monitors ----------------------------------------
    let displays = w.output.detected_displays.clone();
    let mut monitor_items = div().flex().flex_col().gap(px(6.));
    if displays.is_empty() {
        monitor_items = monitor_items.child(helper_text(
            "No physical monitors detected. Connect an external display and click Refresh.",
        ));
    } else {
        for (idx, disp) in displays.iter().enumerate() {
            let badge_label = if disp.is_primary {
                "PRIMARY"
            } else {
                "SECONDARY"
            };
            let badge_color = if disp.is_primary {
                theme::c(theme::LIVE_GREEN)
            } else {
                theme::c(0x66B3FF)
            };
            monitor_items = monitor_items.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .py(px(6.))
                    .px(px(10.))
                    .rounded(px(4.))
                    .bg(theme::c(theme::BG_DARK).opacity(0.6))
                    .border_1()
                    .border_color(theme::c(theme::BORDER_PANEL))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(theme::c(theme::TEXT_PRIMARY))
                                            .child(disp.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .py(px(2.))
                                            .px(px(6.))
                                            .rounded(px(3.))
                                            .bg(badge_color.opacity(0.15))
                                            .text_size(px(9.))
                                            .text_color(badge_color)
                                            .child(badge_label),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme::c(theme::TEXT_SECONDARY))
                                    .child(format!(
                                        "Resolution: {}×{}  |  Origin: ({}, {})",
                                        disp.width, disp.height, disp.x, disp.y
                                    )),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new(SharedString::from(format!("use-display-{idx}")))
                            .label("Use Display")
                            .primary()
                            .small()
                            .on_click(cx.listener(move |w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(Message::ApplyDetectedDisplay(idx)),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            );
        }
    }

    let detected_card = section_card(
        "DETECTED HARDWARE MONITORS",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(monitor_items)
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .child(div().flex_1())
                    .child(
                        Button::new("refresh-displays")
                            .label("Refresh Displays")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(Message::RefreshDisplays),
                                    window,
                                    cx,
                                );
                            })),
                    ),
            ),
    );

    // --- physical display placement ----------------------------------------
    let auto_fullscreen = w.app_config.output.auto_fullscreen;
    // Every output with a window of its own is placed on a display of its
    // own, so the operator picks the screen the placement applies to. A
    // second projector is a second screen, not a second copy of the first.
    let placeable: Vec<(String, String)> = w
        .output
        .manager
        .iter()
        .filter(|output| matches!(output.output_type, OutputType::Window) && output.id != "main")
        .map(|output| (output.id.clone(), output.label.clone()))
        .collect();
    let target = w.output.placement_target.clone();
    let placement_target = target.clone();
    let mut target_row = div().flex().flex_row().items_center().gap(px(6.)).child(
        div()
            .text_size(px(12.))
            .text_color(theme::c(theme::TEXT_SECONDARY))
            .child("Screen:"),
    );
    for (id, label) in &placeable {
        let id_for_target = id.clone();
        target_row = target_row.child(
            Button::new(SharedString::from(format!("place-target-{id}")))
                .label(label.clone())
                .ghost()
                .small()
                .toggled(target == *id)
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Output(Message::SelectPlacementTarget(id_for_target.clone())),
                        window,
                        cx,
                    );
                })),
        );
    }
    // The display the screen is on: the window opens on it.
    let display_count = cx.displays().len();
    let display_label = match placement_of(w, &target).display_index {
        Some(index) => format!("Display {}", index + 1),
        None => "Primary display".to_string(),
    };
    target_row = target_row
        .child(div().w(px(12.)))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child("Display:"),
        )
        .child(
            Button::new("place-display")
                .label(display_label)
                .ghost()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Output(Message::CycleDisplay), window, cx);
                })),
        );
    if display_count > 1 {
        target_row = target_row.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("{display_count} display(s) attached")),
        );
    }
    let placement_card = section_card(
        "PHYSICAL DISPLAY PLACEMENT",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(target_row)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Screen X:"),
                    )
                    .child(
                        div()
                            .w(px(90.))
                            .child(Input::new(&w.output_screen_x_input.clone())),
                    )
                    .child(div().w(px(12.)))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Screen Y:"),
                    )
                    .child(
                        div()
                            .w(px(90.))
                            .child(Input::new(&w.output_screen_y_input.clone())),
                    )
                    .child(div().w(px(16.)))
                    .child(
                        Switch::new("output-auto-fullscreen")
                            .checked(auto_fullscreen)
                            .on_click(cx.listener(|w, checked: &bool, window, cx| {
                                w.handle_message(
                                    RootMessage::OutputAutoFullscreenToggled(*checked),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Auto-Fullscreen secondary display"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child("Quick Resolution Presets:"),
                    )
                    .child(res_preset_btn(
                        "res-1080",
                        "1080p (1920×1080)",
                        1920,
                        1080,
                        placement_target.clone(),
                        cx,
                    ))
                    .child(res_preset_btn(
                        "res-720",
                        "720p (1280×720)",
                        1280,
                        720,
                        placement_target.clone(),
                        cx,
                    ))
                    .child(res_preset_btn(
                        "res-4k",
                        "4K (3840×2160)",
                        3840,
                        2160,
                        placement_target.clone(),
                        cx,
                    )),
            ),
    );

    // --- output destinations & routing --------------------------------------
    let outputs = w.output.manager.iter().cloned().collect::<Vec<_>>();
    let mut output_rows = div().flex().flex_col().gap(px(4.));
    for output in &outputs {
        // The audience screen follows the operator's global output
        // resolution, so the row shows that one rather than the size it
        // was last given.
        let (width, height) = crate::ui::compositor::screen_resolution(w, &output.id);
        output_rows = output_rows.child(output_row(output, (width, height), cx));
    }

    let routing_card = section_card(
        "OUTPUT DESTINATIONS",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            // Which layers a screen shows, and which theme it draws the
            // slide in, belongs to the screen, so it lives in the Looks &
            // Screens matrix. Say where it is, rather than promising a
            // routing that is not here.
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(
                        "Each output shows a window or a broadcast. Which layers it shows — \
                         the media, the slide, the props, the messages, the mask — and which theme \
                         it draws in is set per screen in the Looks & Screens matrix.",
                    ),
            )
            .child(output_rows)
            .child(add_output_form(w, cx)),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(14.))
        .child(detected_card)
        .child(placement_card)
        .child(routing_card)
}

/// A resolution preset for the screen the card is about.
///
/// The card has a screen selector above it, so the preset belongs to the
/// screen that is selected: a preset that always changed the audience would
/// change a screen the operator was not looking at.
fn res_preset_btn(
    id: &'static str,
    label: &'static str,
    width: u32,
    height: u32,
    target: String,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    Button::new(id)
        .label(label)
        .ghost()
        .small()
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::Output(Message::SetResolution(target.clone(), width, height)),
                window,
                cx,
            );
        }))
}

fn output_row(output: &NamedOutput, size: (u32, u32), cx: &mut Context<MainWindow>) -> Div {
    let type_label = match &output.output_type {
        OutputType::Window => "Window".to_string(),
        OutputType::Ndi { stream_name } => format!("NDI: {stream_name}"),
    };
    let resolution = format!("{}×{}", size.0, size.1);
    // The display the output's window opens on: two projectors are two
    // screens on two displays, and the row says which is which.
    let display_label = match output.placement.display_index {
        Some(index) => format!("Display {}", index + 1),
        None => "Primary".to_string(),
    };
    let content_label = output.content.to_string();
    // The mirror button names the output whose picture this one shows.
    let mirror_label = match &output.content {
        OutputContentRoute::Mirror { source_id } => format!("Mirror: {source_id}"),
        _ => String::from("Mirror"),
    };
    // A broadcast composites the slide itself, so it cannot show another
    // output's picture and it cannot route anywhere else. The row says so
    // rather than offering a control that does nothing.
    let is_window = matches!(output.output_type, OutputType::Window);
    let id = output.id.clone();
    let id_for_content = output.id.clone();
    let id_for_mirror = output.id.clone();
    let id_for_720 = output.id.clone();
    let id_for_1080 = output.id.clone();
    let id_for_remove = output.id.clone();

    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .py(px(4.))
        .bg(theme::c(theme::BG_DARK))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            Switch::new(SharedString::from(format!("out-active-{}", output.id)))
                .checked(output.active)
                .on_click(cx.listener(move |w, checked: &bool, window, cx| {
                    w.handle_message(
                        RootMessage::Output(Message::SetActive(id.clone(), *checked)),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            div()
                .w(px(120.))
                .text_size(px(13.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(output.label.clone()),
        )
        .child(
            div()
                .w(px(100.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(type_label),
        )
        .child(
            div()
                .w(px(70.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(resolution),
        )
        .child(
            div()
                .w(px(70.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(display_label),
        )
        // A window can route to another picture; a broadcast always carries
        // the live slide, so it gets a plain label instead of a control that
        // would do nothing.
        .when(is_window, |el| {
            el.child(
                Button::new(SharedString::from(format!("out-content-{}", output.id)))
                    .label(content_label)
                    .ghost()
                    .small()
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Output(Message::CycleContent(id_for_content.clone())),
                            window,
                            cx,
                        );
                    })),
            )
            // A mirror shows another output's picture, so a confidence
            // monitor can show exactly what the projector shows.
            .child(
                Button::new(SharedString::from(format!("out-mirror-{}", output.id)))
                    .label(mirror_label)
                    .ghost()
                    .small()
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Output(Message::CycleMirror(id_for_mirror.clone())),
                            window,
                            cx,
                        );
                    })),
            )
        })
        .when(!is_window, |el| {
            el.child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("Always the live slide"),
            )
        })
        .child(div().flex_1())
        .child(
            Button::new(SharedString::from(format!("out-720-{}", output.id)))
                .label("720p")
                .ghost()
                .small()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Output(Message::SetResolution(id_for_720.clone(), 1280, 720)),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new(SharedString::from(format!("out-1080-{}", output.id)))
                .label("1080p")
                .ghost()
                .small()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        RootMessage::Output(Message::SetResolution(
                            id_for_1080.clone(),
                            1920,
                            1080,
                        )),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new(SharedString::from(format!("out-remove-{}", output.id)))
                .label("X")
                .danger()
                .small()
                // The app's own screens cannot be removed: the operator's
                // controls drive them.
                .when(!is_builtin_output(&output.id), |b| {
                    b.on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::Output(Message::Remove(id_for_remove.clone())),
                            window,
                            cx,
                        );
                    }))
                }),
        )
}

fn add_output_form(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .w(px(160.))
                .child(Input::new(&w.output_label_input.clone())),
        )
        .child(
            div()
                .w(px(200.))
                .child(Input::new(&w.output_ndi_name_input.clone())),
        )
        .child(
            Button::new("add-window-output")
                .label("+ Window Output")
                .primary()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Output(Message::AddWindow), window, cx);
                })),
        )
        .child(
            Button::new("add-ndi-output")
                .label("+ NDI Output")
                .secondary()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Output(Message::AddNdi), window, cx);
                })),
        )
}

// ---------------------------------------------------------------------------
// Network & NDI tab
// ---------------------------------------------------------------------------

pub(crate) fn tab_network(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let fps_30_active = w.app_config.ndi.frame_rate == 30;
    let fps_60_active = w.app_config.ndi.frame_rate == 60;
    let ndi_is_live = !w.presenting.ndi_outputs.is_empty();

    let ndi_card = section_card(
        "NDI NETWORK VIDEO BROADCAST",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Source Name:"),
                    )
                    .child(
                        div()
                            .w(px(180.))
                            .child(Input::new(&w.ndi_source_input.clone())),
                    )
                    .child(div().w(px(12.)))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Frame Rate:"),
                    )
                    .child(
                        Button::new("ndi-fps-30")
                            .label("30 FPS")
                            .when(fps_30_active, |b| b.primary())
                            .when(!fps_30_active, |b| b.ghost())
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(Message::NdiFrameRateChanged(30)),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new("ndi-fps-60")
                            .label("60 FPS")
                            .when(fps_60_active, |b| b.primary())
                            .when(!fps_60_active, |b| b.ghost())
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(Message::NdiFrameRateChanged(60)),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(div().flex_1())
                    .child(if ndi_is_live {
                        badge_pill("ndi-badge", "NDI BROADCASTING", theme::LIVE_GREEN)
                            .into_any_element()
                    } else {
                        badge_pill("ndi-badge", "NDI STANDBY", theme::TEXT_MUTED).into_any_element()
                    })
                    .child(
                        Button::new("ndi-toggle")
                            .label(if ndi_is_live {
                                "Stop NDI Broadcast"
                            } else {
                                "Start NDI Broadcast"
                            })
                            .when(ndi_is_live, |b| b.danger())
                            .when(!ndi_is_live, |b| b.primary())
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(RootMessage::ToggleNdi, window, cx);
                            })),
                    ),
            )
            .child(helper_text(
                "Zero-configuration broadcast stream discoverable on the local network by OBS \
                 Studio, vMix, TriCaster, and standard video switchers.",
            )),
    );

    let http_port = w.triggers.http_port_str.clone();
    let osc_port = w.triggers.osc_port_str.clone();

    let triggers_card = section_card(
        "REMOTE CONTROL & AUTOMATION TRIGGERS",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child(format!("HTTP REST API Port: {http_port}")),
                    )
                    .child(div().w(px(16.)))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child(format!("OSC UDP Port: {osc_port}")),
                    )
                    .child(div().flex_1())
                    .child(badge_pill("triggers-badge", "TRIGGERS", theme::LIVE_GREEN))
                    .child(
                        Button::new("open-triggers")
                            .label("Configure in Triggers & Automation…")
                            .secondary()
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(Message::SettingsClose),
                                    window,
                                    cx,
                                );
                                w.handle_message(RootMessage::ToggleTriggersPanel, window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(helper_text("HTTP REST API Endpoints:"))
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child(
                                "  • POST /action/next — Advance to next slide\n  \
                                 • POST /action/prev — Return to previous slide\n  \
                                 • POST /action/clear — Clear live slide output\n  \
                                 • POST /action/slide/:index — Jump directly to slide index\n  \
                                 • The same actions live under /api/slides/… and /api/clear",
                            ),
                    )
                    .child(div().h(px(4.)))
                    .child(helper_text("OSC (Open Sound Control) UDP Commands:"))
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child(
                                "  • /next, /prev, /clear, /black, /slide/N\n  \
                                 • The same actions live under /slide/next, /slide/prev, …",
                            ),
                    ),
            ),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(14.))
        .child(ndi_card)
        .child(triggers_card)
}

// ---------------------------------------------------------------------------
// Transitions tab
// ---------------------------------------------------------------------------

pub(crate) fn tab_transitions(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let dur = w.presenting.global_transition.duration_ms();

    let transitions_list: &[(&str, Transition)] = &[
        ("Cut", Transition::Cut),
        (
            "Fade",
            Transition::Fade {
                duration_ms: dur.max(100),
            },
        ),
        (
            "Push Left",
            Transition::Push {
                duration_ms: dur.max(100),
                direction: 0,
            },
        ),
        (
            "Push Right",
            Transition::Push {
                duration_ms: dur.max(100),
                direction: 1,
            },
        ),
        (
            "Wipe Left",
            Transition::Wipe {
                duration_ms: dur.max(100),
                angle_deg: 0,
            },
        ),
        (
            "Wipe Right",
            Transition::Wipe {
                duration_ms: dur.max(100),
                angle_deg: 180,
            },
        ),
        (
            "Zoom",
            Transition::Zoom {
                duration_ms: dur.max(100),
            },
        ),
    ];

    let mut trans_buttons = div().flex().flex_row().items_center().gap(px(6.));
    for (name, trans) in transitions_list {
        let is_selected = match (&w.presenting.global_transition, trans) {
            (Transition::Cut, Transition::Cut) => true,
            (Transition::Fade { .. }, Transition::Fade { .. }) => true,
            (Transition::Push { direction: d1, .. }, Transition::Push { direction: d2, .. }) => {
                d1 == d2
            }
            (Transition::Wipe { angle_deg: a1, .. }, Transition::Wipe { angle_deg: a2, .. }) => {
                a1 == a2
            }
            (Transition::Zoom { .. }, Transition::Zoom { .. }) => true,
            _ => false,
        };
        let msg = Message::SetGlobalTransitionType(*trans);
        trans_buttons = trans_buttons.child(
            Button::new(SharedString::from(format!(
                "trans-{}",
                name.replace(' ', "-")
            )))
            .label(*name)
            .when(is_selected, |b| b.primary())
            .when(!is_selected, |b| b.ghost())
            .small()
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(RootMessage::Output(msg.clone()), window, cx);
            })),
        );
    }

    let transition_card = section_card(
        "GLOBAL SLIDE TRANSITION",
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(description_text(
                "Default transition applied when advancing slides in Show and Presenter views:",
            ))
            .child(trans_buttons)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Duration:"),
                    )
                    .child(
                        div()
                            .w(px(260.))
                            .child(Slider::new(&w.transition_duration_slider.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_PRIMARY))
                            .child(format!("{dur} ms")),
                    ),
            ),
    );

    let media_card = section_card(
        "MEDIA PLAYBACK DEFAULTS",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.))
                    .child(badge_pill(
                        "badge-gpu",
                        "GPU TEXT PIPELINE ACTIVE",
                        theme::ACCENT_ORANGE,
                    ))
                    .child(badge_pill(
                        "badge-audio",
                        "RODIO AUDIO ENGINE (44.1/48kHz)",
                        theme::LIVE_GREEN,
                    )),
            )
            .child(helper_text(
                "Media playback supports MP4, WebM, MOV video via FFmpeg and MP3, WAV, FLAC, \
                 Vorbis audio.",
            )),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(14.))
        .child(transition_card)
        .child(media_card)
}

// ---------------------------------------------------------------------------
// Advanced tab
// ---------------------------------------------------------------------------

pub(crate) fn tab_advanced(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let reduce_motion = w.ui.reduce_motion;
    let cols = w.presenting.slide_grid_cols;

    let ui_card = section_card(
        "ACCESSIBILITY & PERFORMANCE",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        Switch::new("reduce-motion")
                            .checked(reduce_motion)
                            .on_click(cx.listener(|w, checked: &bool, _window, cx| {
                                crate::ui::output::toggle_reduce_motion(w, *checked, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Reduce Motion animations"),
                    ),
            )
            .child(helper_text(
                "Suppresses decorative sliding animations for lower CPU/GPU overhead.",
            ))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child("Default Slide Grid Zoom:"),
                    )
                    .child(
                        div()
                            .w(px(180.))
                            .child(Slider::new(&w.default_grid_cols_slider.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme::c(theme::TEXT_PRIMARY))
                            .child(format!("{cols} columns")),
                    ),
            ),
    );

    let system_card = section_card(
        "SYSTEM & FACTORY RESET",
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(description_text(
                "Restore all configuration options in config.toml back to factory default values.",
            ))
            .child(
                Button::new("reset-defaults")
                    .label("Reset All Settings to Factory Defaults")
                    .danger()
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(RootMessage::Output(Message::ResetToDefaults), window, cx);
                    })),
            )
            .child(div().w_full().h(px(1.)).bg(theme::c(theme::BORDER_PANEL)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(format!(
                        "OpenPresenter v0.1.0 • Target OS: {} • Arch: {}",
                        std::env::consts::OS,
                        std::env::consts::ARCH
                    )),
            ),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .p(px(14.))
        .child(ui_card)
        .child(system_card)
}

fn badge_pill(id: &'static str, label: &'static str, color: u32) -> impl IntoElement {
    div()
        .id(id)
        .py(px(2.))
        .px(px(6.))
        .rounded(px(3.))
        .bg(theme::c(color).opacity(0.12))
        .border_1()
        .border_color(theme::c(color))
        .text_size(px(9.))
        .text_color(theme::c(color))
        .child(label)
}

// ===========================================================================
// Looks & Screens matrix modal
// ===========================================================================

/// Render the Looks & Screens matrix modal (opened via
/// `RootMessage::ToggleLooksMatrixModal`): the look selector strip, per-screen
/// layer routing table for the selected look, and the add / make-live /
/// delete look controls.
pub(crate) fn matrix_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let looks = w.props.manager.looks.clone();
    let active_id = w.props.manager.active_look_id.clone();
    let selected_id = w
        .output
        .selected_look_id
        .clone()
        .or(active_id.clone())
        .or_else(|| looks.first().map(|l| l.id.clone()));
    let current_look = selected_id
        .as_ref()
        .and_then(|id| looks.iter().find(|l| &l.id == id))
        .cloned();

    // Top bar.
    let top_bar = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(10.))
        .child(
            Icon::new(IconName::Settings2)
                .text_color(theme::c(theme::ACCENT_ORANGE))
                .with_size(px(16.)),
        )
        .child(
            div()
                .text_size(px(15.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child("LOOKS & SCREENS MATRIX"),
        )
        .child(div().flex_1())
        .child(
            Button::new("matrix-configure-screens")
                .label("Configure Screens")
                .secondary()
                .small()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::Output(Message::SettingsOpen), window, cx);
                })),
        )
        .child(
            Button::new("matrix-close")
                .icon(IconName::Close)
                .ghost()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(RootMessage::ToggleLooksMatrixModal, window, cx);
                })),
        );

    // Looks selector strip.
    let mut looks_strip = div().flex_1().flex().flex_row().items_center().gap(px(6.));
    for look in &looks {
        let is_selected = selected_id.as_deref() == Some(look.id.as_str());
        let is_live = active_id.as_deref() == Some(look.id.as_str());
        let id = look.id.clone();
        looks_strip = looks_strip.child(
            Button::new(SharedString::from(format!("matrix-look-{}", look.id)))
                .label(look.name.clone())
                .when(is_selected, |b| b.primary())
                .when(!is_selected, |b| b.secondary())
                .small()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(RootMessage::SelectLook(id.clone()), window, cx);
                }))
                .when(is_live, |b| {
                    b.icon(
                        Icon::new(IconName::CircleCheck)
                            .text_color(theme::c(theme::LIVE_GREEN))
                            .with_size(px(11.)),
                    )
                }),
        );
    }

    let new_look_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .w(px(160.))
                .child(Input::new(&w.look_name_input.clone())),
        )
        .child(
            Button::new("matrix-add-look")
                .label("+ Add")
                .secondary()
                .small()
                .when(!w.output.editing_look_name.trim().is_empty(), |b| {
                    b.on_click(cx.listener(|w, _, window, cx| {
                        let name = w.output.editing_look_name.trim().to_string();
                        w.handle_message(RootMessage::CreateLook(name), window, cx);
                    }))
                }),
        );

    // Matrix table.
    let table = if let Some(look) = current_look.clone() {
        matrix_table(w, &look, cx)
    } else {
        div()
            .p(px(20.))
            .text_size(px(13.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child("No Looks configured. Create one above to start routing layers.")
    };

    // Footer.
    let footer = if let Some(look) = current_look.clone() {
        let is_live = active_id.as_deref() == Some(look.id.as_str());
        let look_id = look.id.clone();
        let look_id_delete = look.id.clone();
        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.))
            .child(if is_live {
                div()
                    .text_size(px(12.))
                    .text_color(theme::c(theme::LIVE_GREEN))
                    .child("This Look is currently LIVE on all outputs")
            } else {
                div()
                    .text_size(px(12.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("Look is selected for editing")
            })
            .child(div().flex_1())
            .child(if !is_live {
                Button::new("matrix-make-live")
                    .label("Make Live")
                    .primary()
                    .small()
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(RootMessage::SelectLook(look_id.clone()), window, cx);
                    }))
                    .into_any_element()
            } else {
                Button::new("matrix-active")
                    .label("Active")
                    .ghost()
                    .small()
                    .into_any_element()
            })
            .child(if looks.len() > 1 {
                Button::new("matrix-delete-look")
                    .label("Delete Look")
                    .danger()
                    .small()
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::DeleteLook(look_id_delete.clone()),
                            window,
                            cx,
                        );
                    }))
                    .into_any_element()
            } else {
                Button::new("matrix-default-look")
                    .label("Default")
                    .ghost()
                    .small()
                    .into_any_element()
            })
    } else {
        div()
    };

    // Backdrop click closes the modal; the card above it swallows clicks.
    div()
        .absolute()
        .inset_0()
        .child(div().absolute().inset_0().on_mouse_down(
            gpui_kit::MouseButton::Left,
            cx.listener(|w, _, window, cx| {
                w.handle_message(RootMessage::ToggleLooksMatrixModal, window, cx);
            }),
        ))
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(860.))
                        .max_h(px(600.))
                        .max_h(relative(0.9))
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .p(px(18.))
                        .bg(theme::c(theme::BG_DARKEST))
                        .border_1()
                        .border_color(theme::c(theme::ACCENT_ORANGE))
                        .rounded(px(8.))
                        .child(top_bar)
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(12.))
                                .child(looks_strip)
                                .child(new_look_row),
                        )
                        .child(table)
                        .child(footer),
                ),
        )
}

/// The screens × layers routing table for one look.
fn matrix_table(w: &mut MainWindow, look: &Look, cx: &mut Context<MainWindow>) -> Div {
    let themes = w.theme_state.list.clone();
    let outputs = w.output.manager.iter().cloned().collect::<Vec<_>>();

    let header = |label: &'static str, width: f32| {
        div()
            .w(px(width))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child(label)
    };

    let mut rows = div().flex().flex_col().gap(px(4.));
    for output in &outputs {
        // The main output is the operator's own control window: it does not
        // show a composite, so it has no screen configuration.
        if output.id == "main" {
            continue;
        }
        let screen_id = output.id.clone();
        let target = look.target_or_default(&screen_id);

        let type_label = match &output.output_type {
            OutputType::Window => "Window",
            OutputType::Ndi { .. } => "NDI Stream",
        };

        let theme_label = target
            .theme_id
            .as_deref()
            .and_then(|id| themes.iter().find(|t| t.id == id))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| String::from("(Original Slide Style)"));
        let theme_is_override = target.theme_id.is_some();

        let next_theme = cycle_theme(target.theme_id.as_deref(), &themes);
        let look_id_theme = look.id.clone();
        let screen_id_theme = screen_id.clone();

        rows = rows.child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .py(px(6.))
                .px(px(12.))
                .bg(theme::c(theme::BG_DARK))
                .border_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .rounded(px(4.))
                .child(
                    div()
                        .w(px(150.))
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(output.label.clone()),
                )
                .child(
                    div()
                        .w(px(80.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(type_label),
                )
                .child(div().w(px(56.)).flex().justify_center().child(toggle_cell(
                    look.id.clone(),
                    screen_id.clone(),
                    LookLayerType::Media,
                    target.media_enabled,
                    cx,
                )))
                .child(div().w(px(56.)).flex().justify_center().child(toggle_cell(
                    look.id.clone(),
                    screen_id.clone(),
                    LookLayerType::Slide,
                    target.slide_enabled,
                    cx,
                )))
                .child(div().w(px(56.)).flex().justify_center().child(toggle_cell(
                    look.id.clone(),
                    screen_id.clone(),
                    LookLayerType::Props,
                    target.props_enabled,
                    cx,
                )))
                .child(div().w(px(64.)).flex().justify_center().child(toggle_cell(
                    look.id.clone(),
                    screen_id.clone(),
                    LookLayerType::Messages,
                    target.messages_enabled,
                    cx,
                )))
                .child(div().w(px(56.)).flex().justify_center().child(toggle_cell(
                    look.id.clone(),
                    screen_id.clone(),
                    LookLayerType::Mask,
                    target.mask_enabled,
                    cx,
                )))
                .child(
                    Button::new(SharedString::from(format!(
                        "matrix-theme-{}-{}",
                        look.id, screen_id
                    )))
                    .label(theme_label)
                    .secondary()
                    .small()
                    .flex_1()
                    .when(theme_is_override, |b| {
                        b.icon(
                            Icon::new(IconName::CircleCheck)
                                .text_color(theme::c(theme::ACCENT_ORANGE))
                                .with_size(px(11.)),
                        )
                    })
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            RootMessage::SetLookScreenTheme(
                                look_id_theme.clone(),
                                screen_id_theme.clone(),
                                next_theme.clone(),
                            ),
                            window,
                            cx,
                        );
                    })),
                ),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(8.))
        .rounded(px(6.))
        .bg(theme::c(theme::BG_DARKEST))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .child(
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .py(px(8.))
                .px(px(12.))
                .bg(theme::c(theme::BG_DARK))
                .rounded(px(4.))
                .child(header("SCREEN OUTPUT", 150.))
                .child(header("TYPE", 80.))
                .child(
                    div()
                        .w(px(56.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .flex()
                        .justify_center()
                        .child("MEDIA"),
                )
                .child(
                    div()
                        .w(px(56.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .flex()
                        .justify_center()
                        .child("SLIDE"),
                )
                .child(
                    div()
                        .w(px(56.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .flex()
                        .justify_center()
                        .child("PROPS"),
                )
                .child(
                    div()
                        .w(px(64.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .flex()
                        .justify_center()
                        .child("MESSAGES"),
                )
                .child(
                    div()
                        .w(px(56.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .flex()
                        .justify_center()
                        .child("MASK"),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("THEME OVERRIDE"),
                ),
        )
        .child(
            div()
                .id("matrix-rows")
                .max_h(px(240.))
                .overflow_y_scroll()
                .child(rows),
        )
}

/// One on/off cell in the matrix: a check/cross chip toggling one layer of one
/// screen target for the selected look.
fn toggle_cell(
    look_id: String,
    screen_id: String,
    layer_type: LookLayerType,
    active: bool,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let layer_name = match layer_type {
        LookLayerType::Media => "media",
        LookLayerType::Slide => "slide",
        LookLayerType::Props => "props",
        LookLayerType::Messages => "messages",
        LookLayerType::Mask => "mask",
    };
    let look_id_for_click = look_id.clone();
    let screen_id_for_click = screen_id.clone();

    div()
        .id(SharedString::from(format!(
            "matrix-{layer_name}-{look_id}-{screen_id}"
        )))
        .flex()
        .items_center()
        .justify_center()
        .py(px(3.))
        .px(px(10.))
        .rounded(px(4.))
        .bg(if active {
            theme::c(theme::ACCENT_ORANGE).opacity(0.2)
        } else {
            gpui_kit::transparent_black()
        })
        .border_1()
        .border_color(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BORDER_PANEL)
        })
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                RootMessage::ToggleLookScreenLayer(
                    look_id_for_click.clone(),
                    screen_id_for_click.clone(),
                    layer_type,
                ),
                window,
                cx,
            );
        }))
        .child(if active {
            Icon::new(IconName::Check)
                .text_color(theme::c(theme::LIVE_GREEN))
                .with_size(px(11.))
                .into_any_element()
        } else {
            Icon::new(IconName::Close)
                .text_color(theme::c(theme::TEXT_MUTED))
                .with_size(px(11.))
                .into_any_element()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_theme_behavior() {
        let mut t1 = SlideTheme::new(
            "Theme 1".into(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        t1.id = "t1".into();
        let mut t2 = SlideTheme::new(
            "Theme 2".into(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        t2.id = "t2".into();
        let themes = vec![t1, t2];

        // None -> first theme
        assert_eq!(cycle_theme(None, &themes), Some("t1".into()));
        // First theme -> second theme
        assert_eq!(cycle_theme(Some("t1"), &themes), Some("t2".into()));
        // Last theme -> None
        assert_eq!(cycle_theme(Some("t2"), &themes), None);
        // Unknown id -> first theme
        assert_eq!(cycle_theme(Some("unknown"), &themes), Some("t1".into()));
        // Empty themes -> None
        assert_eq!(cycle_theme(None, &[]), None);
    }
}
