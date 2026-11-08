//! Modal overlays rendered as absolute children of the shell root: keyboard
//! shortcuts, delete confirmation, new presentation, the Looks & Screens
//! matrix, triggers, recording, and the app preferences dialog.

use gpui_kit::TestSupportExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{Context, IntoElement, div, px};

use crate::ui::components;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::theme;

/// Render every currently-open modal, topmost last.
pub fn view(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let mut stack: Vec<gpui_kit::AnyElement> = Vec::new();

    if w.output.matrix_open {
        stack.push(crate::ui::output::matrix_modal(w, cx).into_any_element());
    }
    if w.triggers.panel_open {
        stack.push(triggers_modal(w, cx).into_any_element());
    }
    if w.recording.panel_open {
        stack.push(recording_modal(w, cx).into_any_element());
    }
    if w.shell.test_pattern_open {
        stack.push(test_pattern_modal(w, cx).into_any_element());
    }
    if w.shell.groups_open {
        stack.push(groups_modal(w, cx).into_any_element());
    }
    if w.shell.clear_groups_open {
        stack.push(clear_groups_modal(w, cx).into_any_element());
    }
    if w.calendar.open {
        stack.push(calendar_modal(w, cx).into_any_element());
    }
    if w.shell.capture_settings_open {
        stack.push(capture_settings_modal(w, cx).into_any_element());
    }
    if w.recording.confirm_stop_pending {
        stack.push(
            components::confirm_dialog(
                "Stop Recording",
                String::from("Stop the recording now? The take is written out as it stands."),
                "Stop",
                std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Recording(crate::ui::recording::Message::ConfirmStop),
                        window,
                        cx,
                    );
                })),
                std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Recording(crate::ui::recording::Message::CancelStop),
                        window,
                        cx,
                    );
                })),
                true,
            )
            .into_any_element(),
        );
    }
    if w.shell.cue_list_slide.is_some() {
        stack.push(cue_list_modal(w, cx).into_any_element());
    }
    if w.cue_inspector.is_open() {
        stack.push(cue_inspector_modal(w, cx).into_any_element());
    }
    if w.editor.new_presentation_open {
        stack.push(new_presentation_modal(w, cx).into_any_element());
    }
    if w.editor.show_delete_confirmation {
        stack.push(delete_confirm_modal(w, cx).into_any_element());
    }
    if w.ui.shortcuts_open {
        stack.push(shortcuts_modal(w, cx).into_any_element());
    }
    if w.ui.about_open {
        stack.push(about_modal(w, cx).into_any_element());
    }

    div().absolute().inset_0().children(stack)
}

/// Triggers & Automation modal: the same panel body as the dock tab.
fn triggers_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::triggers::triggers_panel(w, cx).into_any_element();
    let close = Button::new("triggers-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleTriggersPanel, window, cx);
        }));
    components::dialog_overlay(
        "Triggers & Automation",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleTriggersPanel, window, cx);
        }))),
    )
}

/// Recording modal: output path / FPS fields plus the status readout.
fn recording_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::recording::recording_panel(w, cx).into_any_element();
    let close = Button::new("recording-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleRecordingPanel, window, cx);
        }));
    components::dialog_overlay(
        "Recording",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleRecordingPanel, window, cx);
        }))),
    )
}

/// The test pattern window.
///
/// The pattern is on the projector, so the operator has to be able to see
/// what they turned on while they turn it on.
fn test_pattern_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::test_pattern::view(w, cx).into_any_element();
    let close = Button::new("test-pattern-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::TestPattern(crate::ui::test_pattern::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Test Patterns",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::TestPattern(crate::ui::test_pattern::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// The groups and labels window.
///
/// The group of a slide is what the operator navigates by during a show,
/// so the groups the app knows and the groups the operator has added are
/// both in one place.
fn groups_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::groups::view(w, cx).into_any_element();
    let close = Button::new("groups-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::Groups(crate::ui::groups::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Groups & Labels",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::Groups(crate::ui::groups::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// The clear groups window.
///
/// A clear group is a set of layers, so the operator can clear the slide and
/// the sound and leave the props up.
fn clear_groups_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::clear_groups::view(w, cx).into_any_element();
    let close = Button::new("clear-groups-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::ClearGroups(crate::ui::clear_groups::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Clear Groups",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::ClearGroups(crate::ui::clear_groups::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// The service calendar.
///
/// The calendar is where the operator looks before a service: what is
/// coming, and what plays at it.
fn calendar_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::calendar::view(w, cx).into_any_element();
    let close = Button::new("calendar-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::Calendar(crate::ui::calendar::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Service Calendar",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::Calendar(crate::ui::calendar::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// The capture settings.
///
/// The settings a service is recorded with, chosen once and used every
/// week.
fn capture_settings_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::capture_settings::view(w, cx).into_any_element();
    let close = Button::new("capture-settings-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CaptureSettings(crate::ui::capture_settings::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Capture Settings",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CaptureSettings(crate::ui::capture_settings::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// The list of the cues on a slide.
///
/// A cue is an automation, so the operator has to be able to see the
/// automations a slide carries before they go live.
fn cue_list_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::cue_inspector::list_view(w, cx).into_any_element();
    let close = Button::new("cue-list-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CueInspector(crate::ui::cue_inspector::Message::CloseList),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Cues",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CueInspector(crate::ui::cue_inspector::Message::CloseList),
                window,
                cx,
            );
        }))),
    )
}

/// The cue inspector.
///
/// The inspector is where the operator reads what a cue will do and changes
/// it.
fn cue_inspector_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = crate::ui::cue_inspector::view(w, cx).into_any_element();
    let close = Button::new("cue-inspector-modal-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CueInspector(crate::ui::cue_inspector::Message::Close),
                window,
                cx,
            );
        }));
    components::dialog_overlay(
        "Cue",
        content,
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::CueInspector(crate::ui::cue_inspector::Message::Close),
                window,
                cx,
            );
        }))),
    )
}

/// Shortcuts overlay: closes on backdrop click and via its Close button.
fn shortcuts_modal(_w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let shortcut = |key: &'static str, desc: &'static str| {
        div()
            .flex()
            .flex_row()
            .gap(px(8.))
            .child(
                div()
                    .w(px(150.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child(key),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(desc),
            )
    };

    let content = div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .max_h(px(430.))
        .id("shortcuts-content")
        .overflow_y_scroll()
        .child(section("EDITOR"))
        .child(shortcut("?", "Toggle shortcuts"))
        .child(shortcut("Cmd+Z / Ctrl+Z", "Undo"))
        .child(shortcut("Cmd+Shift+Z / Ctrl+Y", "Redo"))
        .child(shortcut("Cmd+C / Ctrl+C", "Copy the selected object"))
        .child(shortcut("Cmd+V / Ctrl+V", "Paste the copied object"))
        .child(shortcut("Cmd+D / Ctrl+D", "Duplicate the selected object"))
        .child(shortcut("Delete / Backspace", "Remove the selected object"))
        .child(shortcut(
            "Arrow keys",
            "Move the selected object one pixel (Shift: ten)",
        ))
        .child(shortcut("Cmd+, / Ctrl+,", "Open settings"))
        .child(shortcut(
            "Esc",
            "Close a menu, a dialog or the text being edited",
        ))
        .child(div().h(px(10.)))
        .child(section("PRESENTER (SHOW MODE)"))
        .child(shortcut("Right / Space / PgDn / Down", "Next slide"))
        .child(shortcut("Left / PgUp / Up", "Previous slide"))
        .child(shortcut("Home / End", "First / Last slide"))
        .child(shortcut("1–9", "Go to slide n"))
        .child(shortcut("V", "Next Verse group"))
        .child(shortcut("C", "Next Chorus group (Clear Slide if none)"))
        .child(shortcut("F1", "Clear All"))
        .child(shortcut("F2", "Clear Slide"))
        .child(shortcut("F3", "Clear Media"))
        .child(shortcut("F4", "Clear Props"))
        .child(shortcut("F5", "Clear Audio"))
        .child(shortcut("F6", "Clear Messages"))
        .child(shortcut(
            "Cmd/Ctrl + 1–6",
            "Clear All / Slide / Media / Props / Audio / Messages",
        ))
        .child(shortcut("B", "Toggle Black Screen"));

    let close = Button::new("shortcuts-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, _, cx| {
            w.ui.shortcuts_open = false;
            cx.notify();
        }));

    components::dialog_overlay(
        "Keyboard Shortcuts",
        content.into_any_element(),
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, _, cx| {
            w.ui.shortcuts_open = false;
            cx.notify();
        }))),
    )
}

/// About dialog: app name, this build's own version, and what the app is.
///
/// The version is the app's own (`CARGO_PKG_VERSION`), never the reference
/// product's numbers.
fn about_modal(_w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let version = env!("CARGO_PKG_VERSION");
    let content = div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .id("about-name")
                .test_support()
                .text_size(px(18.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child("OpenPresenter"),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(format!("Version {version}")),
        )
        .child(
            div()
                .w(px(380.))
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_SECONDARY))
                .child(
                    "Live presentation software for the control desk: edit slides, drive the show, send to the projector, the stage display, NDI and disk.",
                ),
        );
    let close = Button::new("about-close")
        .label("Close")
        .secondary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleAbout, window, cx);
        }));
    components::dialog_overlay(
        "About OpenPresenter",
        content.into_any_element(),
        vec![close.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ToggleAbout, window, cx);
        }))),
    )
}

fn section(label: &'static str) -> impl IntoElement {
    div()
        .text_size(px(10.))
        .text_color(theme::c(theme::TEXT_MUTED))
        .child(label)
}

/// Delete-confirmation modal shared by the presentation, song, bible
/// translation, and playlist flows (mirrors the iced-era
/// `MainWindow::delete_confirm_view` dispatch).
fn delete_confirm_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    if let Some(id) = w.service.to_delete.clone() {
        let target_name = w
            .service
            .plans
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| String::from("this plan"));

        return components::confirm_dialog(
            "Delete Service Plan",
            format!("Delete \"{target_name}\"?"),
            "Delete",
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Playlist(crate::ui::playlist::Message::ConfirmDelete),
                    window,
                    cx,
                );
            })),
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Playlist(crate::ui::playlist::Message::CancelDelete),
                    window,
                    cx,
                );
            })),
            true,
        );
    }

    if let Some(id) = w.song.to_delete.clone() {
        // The whole library, not the filtered list: a song the filter
        // hides is still the song the operator is about to delete, and a
        // confirmation that names nothing is a confirmation they cannot
        // answer.
        let target_name = w
            .song
            .library_songs
            .iter()
            .chain(w.song.songs.iter())
            .find(|s| s.id == id)
            .map(|s| s.title.clone())
            .unwrap_or_else(|| String::from("this song"));

        return components::confirm_dialog(
            "Delete Song",
            format!("Delete \"{target_name}\"?"),
            "Delete",
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Songs(crate::ui::songs::Message::ConfirmDelete),
                    window,
                    cx,
                );
            })),
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Songs(crate::ui::songs::Message::CancelDelete),
                    window,
                    cx,
                );
            })),
            true,
        );
    }

    if let Some(id) = w.library.to_delete.clone() {
        let target_name = w
            .library
            .assets
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| String::from("this asset"));

        return components::confirm_dialog(
            "Delete Asset",
            format!("Delete \"{target_name}\" from the library?"),
            "Delete",
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Library(crate::ui::library::Message::ConfirmDelete),
                    window,
                    cx,
                );
            })),
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Library(crate::ui::library::Message::CancelDelete),
                    window,
                    cx,
                );
            })),
            true,
        );
    }

    if let Some(id) = w.bible.translation_to_delete.clone() {
        let target_name = w
            .bible
            .translations
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| String::from("this translation"));

        return components::confirm_dialog(
            "Delete Bible Translation",
            format!("Delete \"{target_name}\"?"),
            "Delete",
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Bible(crate::ui::bible::Message::ConfirmDeleteTranslation),
                    window,
                    cx,
                );
            })),
            std::rc::Rc::new(cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Bible(crate::ui::bible::Message::CancelDeleteTranslation),
                    window,
                    cx,
                );
            })),
            true,
        );
    }

    let target_name = w
        .editor
        .delete_target_id
        .as_ref()
        .and_then(|id| {
            w.editor
                .presentations
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.clone())
        })
        .unwrap_or_else(|| String::from("this presentation"));

    let message = format!("Delete presentation \"{target_name}\"?");
    components::confirm_dialog(
        "Delete Presentation",
        message,
        "Delete",
        std::rc::Rc::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::ConfirmDeletePresentation, window, cx);
        })),
        std::rc::Rc::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::CancelDelete, window, cx);
        })),
        true,
    )
}

/// New-presentation modal with a live name input.
fn new_presentation_modal(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let content = div().w(px(300.)).child(
        gpui_kit::component::input::Input::new(&w.new_presentation_input.clone())
            .aria_label("Presentation name"),
    );

    let create = Button::new("new-pres-create")
        .label("Create")
        .primary()
        .px(px(24.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::CreatePresentation, window, cx);
        }));
    let cancel = Button::new("new-pres-cancel")
        .label("Cancel")
        .secondary()
        .px(px(20.))
        .py(px(8.))
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::CancelNewPresentation, window, cx);
        }));

    components::dialog_overlay(
        "New Presentation",
        content.into_any_element(),
        vec![create.into_any_element(), cancel.into_any_element()],
        Some(Box::new(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::CancelNewPresentation, window, cx);
        }))),
    )
}
