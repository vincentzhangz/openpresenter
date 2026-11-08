//! The settings pages.
//!
//! The pages are the ones the app's own menu names: General, Audio,
//! Devices, Groups & Labels, Import, Inputs, Network, Screens, Updates,
//! Media and Slide Labels, beside the two the app adds for itself. Every
//! page holds settings that are really wired: a page that shows a number
//! the operator cannot change is a page they will not read.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, Div, SharedString, div, px};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::{Message as RootMessage, SettingsTab};
use crate::ui::theme;

/// A card of settings.
fn card(title: &'static str, content: Div) -> Div {
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
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(title),
        )
        .child(content)
}

/// A line of text under a card's title.
fn description(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .child(text.into())
}

/// A row that shows a setting's value.
fn value_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(label),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(value.into()),
        )
}

/// The page the operator is on.
pub fn page(w: &mut MainWindow, tab: SettingsTab, cx: &mut Context<MainWindow>) -> Div {
    match tab {
        SettingsTab::General => crate::ui::output::tab_general(w, cx),
        SettingsTab::Audio => tab_audio(w, cx),
        SettingsTab::Devices => tab_devices(w, cx),
        SettingsTab::GroupsAndLabels => tab_groups(w, cx),
        SettingsTab::Import => tab_import(w, cx),
        SettingsTab::Inputs => tab_inputs(w, cx),
        SettingsTab::Network => crate::ui::output::tab_network(w, cx),
        SettingsTab::Screens => crate::ui::output::tab_screens(w, cx),
        SettingsTab::Updates => tab_updates(w, cx),
        SettingsTab::Media => tab_media(w, cx),
        SettingsTab::SlideLabels => tab_slide_labels(w, cx),
        SettingsTab::Transitions => crate::ui::output::tab_transitions(w, cx),
        SettingsTab::Advanced => crate::ui::output::tab_advanced(w, cx),
    }
}

/// The sound the app plays.
///
/// The master volume is the level the room hears: a clip is played at its
/// own volume, and the master volume is on top of it, so the whole service
/// can be turned down without touching every clip.
fn tab_audio(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let volume = w.app_config.audio.master_volume;

    let mut level = div().flex().flex_row().items_center().gap(px(6.));
    for step in [-20i32, -5, 5, 20] {
        let next = (volume as i32 + step).clamp(0, 100) as u8;
        level = level.child(
            Button::new(SharedString::from(format!("settings-audio-volume-{step}")))
                .label(match step {
                    -20 => "--",
                    -5 => "-",
                    5 => "+",
                    _ => "++",
                })
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |w, _, _window, cx| {
                    w.app_config.audio.master_volume = next;
                    let _ = w.app_config.save();
                    crate::ui::audio::apply_master_volume(w);
                    cx.notify();
                })),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "MASTER VOLUME",
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(description(
                    "The level the room hears. A clip plays at its own volume, and the master volume is on top of it.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_PRIMARY))
                                .child(format!("{volume}%")),
                        )
                        .child(level),
                ),
        ))
        .child(card(
            "OUTPUT",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Device", "System default"))
                .child(description(
                    "The app plays through the system's default output, so the operator's mixer stays in charge of the room.",
                )),
        ))
}

/// The devices the app shows on.
///
/// The devices are the operator's environment, so they are listed with what
/// each one is doing: a window on a screen, or a broadcast on the network.
fn tab_devices(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let mut list = div().flex().flex_col().gap(px(6.));
    for output in w.output.manager.iter() {
        let id = output.id.clone();
        let name = output.label.clone();
        let kind = match output.output_type {
            crate::output::OutputType::Window => "Window",
            crate::output::OutputType::Ndi { .. } => "Broadcast",
        };
        let resolution = format!("{}×{}", output.width, output.height);
        list = list.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child(format!("{kind} · {resolution}")),
                )
                .child(
                    Button::new(SharedString::from(format!("settings-device-{id}")))
                        .label(if output.active { "On" } else { "Off" })
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener({
                            let id = id.clone();
                            move |w, _, window, cx| {
                                w.handle_message(
                                    RootMessage::Output(crate::ui::output::Message::SetActive(
                                        id.clone(),
                                        !w.output
                                            .manager
                                            .get(&id)
                                            .map(|o| o.active)
                                            .unwrap_or(false),
                                    )),
                                    window,
                                    cx,
                                );
                            }
                        })),
                ),
        );
    }
    if w.output.manager.iter().count() == 0 {
        list = list.child(description("No outputs are set up yet."));
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card("OUTPUT DEVICES", list))
        .child(card(
            "STAGE",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row(
                    "Stage display",
                    if w.presenting.stage_display_active {
                        "On"
                    } else {
                        "Off"
                    },
                ))
                .child(description(
                    "The stage monitor is the speaker's own screen: it shows the live slide, the next slide and the notes.",
                )),
        ))
}

/// The groups a slide belongs to and the labels it carries.
fn tab_groups(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(crate::ui::groups::view(w, cx))
}

/// What happens when something is imported.
fn tab_import(w: &mut MainWindow, _cx: &mut Context<MainWindow>) -> Div {
    let media_dir = crate::ui::import_export::media_dir(w);
    let media_dir_str = media_dir.to_string_lossy().into_owned();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "IMPORTED MEDIA",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Media folder", media_dir_str))
                .child(description(
                    "The media that travels with an imported package is unpacked here, so the slides point at files that exist.",
                )),
        ))
        .child(card(
            "FORMATS",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Presentation package", ".opp"))
                .child(value_row("PowerPoint", ".pptx"))
                .child(value_row("Song", "OpenLyrics XML"))
                .child(value_row("Bible", "SQLBML / CSV")),
        ))
}

/// The remote-control inputs the app listens on.
///
/// The inputs are off by default: an app that accepts commands from the
/// network on every interface is an app that anyone can drive.
fn tab_inputs(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let http_port = w.triggers.http_port_str.clone();
    let osc_port = w.triggers.osc_port_str.clone();
    let http_on = w.app_config.triggers.http_enabled;
    let osc_on = w.app_config.triggers.osc_enabled;

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "REMOTE CONTROL",
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_SECONDARY))
                                .child("HTTP REST API"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .child(format!("port {http_port}")),
                        )
                        .child(
                            Button::new("settings-http-toggle")
                                .label(if http_on { "On" } else { "Off" })
                                .ghost()
                                .xsmall()
                                .when(http_on, |b| b.primary())
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    let http_on = http_on;
                                    w.app_config.triggers.http_enabled = !http_on;
                                    let _ = w.app_config.save();
                                    w.handle_message(
                                        RootMessage::Output(crate::ui::output::Message::SaveConfig),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(theme::c(theme::TEXT_SECONDARY))
                                .child("OSC"),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::c(theme::TEXT_MUTED))
                                .child(format!("port {osc_port}")),
                        )
                        .child(
                            Button::new("settings-osc-toggle")
                                .label(if osc_on { "On" } else { "Off" })
                                .ghost()
                                .xsmall()
                                .when(osc_on, |b| b.primary())
                                .on_click(cx.listener(move |w, _, window, cx| {
                                    let osc_on = osc_on;
                                    w.app_config.triggers.osc_enabled = !osc_on;
                                    let _ = w.app_config.save();
                                    w.handle_message(
                                        RootMessage::Output(crate::ui::output::Message::SaveConfig),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
                .child(description(
                    "The listeners accept commands without a password, so they are off until the operator turns them on.",
                )),
        ))
}

/// Where the app looks for updates.
fn tab_updates(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let channel = w.app_config.updates.channel.clone();
    let version = env!("CARGO_PKG_VERSION").to_string();
    let checking = w.ui.update_checking;
    let status = w.ui.update_check_status.clone();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "VERSION",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Installed", version))
                .child(value_row(
                    "Update channel",
                    if channel == "beta" { "Beta" } else { "Stable" },
                )),
        ))
        .child(card(
            "UPDATE CHANNEL",
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(description(
                    "The beta channel carries the builds that are still being made: they are the ones with the newest features.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.))
                        .child(
                            Button::new("settings-update-stable")
                                .label("Stable")
                                .ghost()
                                .xsmall()
                                .when(channel != "beta", |b| b.primary())
                                .on_click(cx.listener(|w, _, _window, cx| {
                                    w.app_config.updates.channel = String::from("stable");
                                    let _ = w.app_config.save();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("settings-update-beta")
                                .label("Beta")
                                .ghost()
                                .xsmall()
                                .when(channel == "beta", |b| b.primary())
                                .on_click(cx.listener(|w, _, _window, cx| {
                                    w.app_config.updates.channel = String::from("beta");
                                    let _ = w.app_config.save();
                                    cx.notify();
                                })),
                        ),
                ),
        ))
        .child(card(
            "CHECK FOR UPDATES",
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(description(
                    "Ask the release feed for the newest build on the selected channel and compare it with the installed version.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            Button::new("settings-update-check")
                                .label(if checking {
                                    "Checking…"
                                } else {
                                    "Check for Updates"
                                })
                                .ghost()
                                .xsmall()
                                .when(!checking, |b| b.primary())
                                .on_click(cx.listener(move |w, _, _window, cx| {
                                    check_for_updates(w, cx);
                                })),
                        )
                        .child(description(status)),
                ),
        ))
}

/// The media library.
fn tab_media(w: &mut MainWindow, _cx: &mut Context<MainWindow>) -> Div {
    let assets = w.library.assets.len();
    let media_dir = crate::ui::import_export::media_dir(w);
    let media_dir_str = media_dir.to_string_lossy().into_owned();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "LIBRARY",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Media items", assets.to_string()))
                .child(value_row("Media folder", media_dir_str)),
        ))
        .child(card(
            "PLAYBACK",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Audio output", "System default"))
                .child(description(
                    "A clip carries its own sound, and the sound stops with the picture: an output that is cleared but still playing is not cleared.",
                )),
        ))
}

/// The label a new slide is given.
fn tab_slide_labels(w: &mut MainWindow, _cx: &mut Context<MainWindow>) -> Div {
    let default_label = w.app_config.slide_labels.default_label.clone();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(card(
            "DEFAULT LABEL",
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(description(
                    "The label a new slide is given. A slide that is a repeat of the chorus before it is marked as one.",
                ))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.)),
                ),
        ))
        .child(card(
            "LABELS",
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(value_row("Default label", default_label))
                .child(description(
                    "The labels the app knows: blank slide, instrumental, repeat and spoken word.",
                )),
        ))
}

/// The outcome of an update check, posted back to the UI thread.
#[derive(Debug, Clone)]
pub(crate) struct UpdateCheckOutcome {
    pub message: String,
}

/// Compare two dotted versions (missing parts count as zero): True when
/// `candidate` is newer than `installed`.
fn is_newer(candidate: &str, installed: &str) -> bool {
    fn parts(version: &str) -> Vec<u64> {
        version
            .trim_start_matches('v')
            .split('.')
            .map(|part| part.trim().parse::<u64>().unwrap_or(0))
            .collect()
    }
    let new = parts(candidate);
    let current = parts(installed);
    let len = new.len().max(current.len());
    for i in 0..len {
        let n = new.get(i).copied().unwrap_or(0);
        let c = current.get(i).copied().unwrap_or(0);
        if n != c {
            return n > c;
        }
    }
    false
}

/// Fetch the latest release tag for the update channel from GitHub.
///
/// The app ships no HTTP client crate, so the check shells out to curl —
/// present on macOS and Windows by default; offline or curl-less machines
/// report the check as failed instead of a version that was never read.
fn fetch_latest_release(channel: &str) -> Result<String, String> {
    let repo = "vincentzhangz/openpresenter";
    let url = if channel == "beta" {
        format!("https://api.github.com/repos/{repo}/releases?per_page=20")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    };
    let output = std::process::Command::new("curl")
        .args([
            "-sS",
            "--fail",
            "-m",
            "8",
            "-A",
            "OpenPresenter",
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ])
        .output()
        .map_err(|e| format!("could not run the update check: {e}"))?;
    if !output.status.success() {
        return Err(String::from("could not reach the update server"));
    }
    let body = String::from_utf8(output.stdout)
        .map_err(|_| String::from("the update server answered with unreadable data"))?;
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| String::from("the update server answered with unreadable data"))?;
    // The list form (beta) picks the newest non-draft release; the latest
    // endpoint answers with one.
    let latest = if channel == "beta" {
        json.as_array().and_then(|releases| {
            releases.iter().find(|release| {
                release.get("draft").and_then(|draft| draft.as_bool()) != Some(true)
            })
        })
    } else {
        Some(&json)
    };
    let tag = latest
        .and_then(|release| release.get("tag_name"))
        .and_then(|tag| tag.as_str())
        .ok_or_else(|| String::from("the update server returned no release"))?;
    Ok(tag.trim_start_matches('v').to_string())
}

/// Start an update check on a worker thread; the outcome is posted to the
/// UI thread's channel when it lands.
pub(crate) fn check_for_updates(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(tx) = w.update_check_tx.clone() else {
        w.ui.update_check_status = String::from("The update check is unavailable.");
        cx.notify();
        return;
    };
    if w.ui.update_checking {
        return;
    }
    let channel = w.app_config.updates.channel.clone();
    w.ui.update_checking = true;
    w.ui.update_check_status = String::from("Checking for updates…");
    std::thread::spawn(move || {
        let message = match fetch_latest_release(&channel) {
            Ok(latest) => {
                let installed = env!("CARGO_PKG_VERSION");
                if is_newer(&latest, installed) {
                    format!("A new version is available: {latest} (installed: {installed})")
                } else {
                    format!("OpenPresenter is up to date (installed: {installed})")
                }
            }
            Err(error) => format!("Update check failed: {error}"),
        };
        let _ = tx.blocking_send(UpdateCheckOutcome { message });
    });
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_release_is_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(is_newer("v0.1.1", "0.1.0"));
    }
}
