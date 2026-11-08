//! The Preferences window: the app's settings in their own window.
//!
//! A settings window is not a dialog pasted over the main window; it is an
//! OS window of its own, the way the reference editor opens its
//! Preferences. It reads the main window's settings state and drives it
//! through the same root messages, so the main window stays the single
//! place where the settings are stored and saved.

use gpui_kit::prelude::*;
use gpui_kit::{AnyWindowHandle, Entity, IntoElement, Window, div};

use crate::ui::main_window::MainWindow;
use crate::ui::theme;

/// The settings window's root entity: the preferences card of the main
/// window, in its own window.
pub struct SettingsWindow {
    /// The main window whose settings this window edits.
    main: Entity<MainWindow>,
}

impl SettingsWindow {
    /// Create the settings window's root view.
    pub(crate) fn new(main: Entity<MainWindow>) -> Self {
        Self { main }
    }
}

impl gpui_kit::Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The main window closing the settings flips the flag; the settings
        // window's next render sees it and closes itself.
        let open = cx.read_entity(&self.main, |w, _| w.output.settings_open);
        if !open {
            window.remove_window();
            return div().size_full().into_any_element();
        }
        let card = self.main.update(cx, |w, cx| {
            crate::ui::output::settings_card(w, cx).into_any_element()
        });
        div()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::c(theme::BG_DARKEST))
            .child(card)
            .into_any_element()
    }
}

/// Open the Preferences window.
pub(crate) fn open(
    main: Entity<MainWindow>,
    cx: &mut gpui_kit::App,
) -> anyhow::Result<AnyWindowHandle> {
    let options = gpui_kit::WindowOptions {
        window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
            origin: gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(0.)),
            size: gpui_kit::size(gpui_kit::px(1080.), gpui_kit::px(760.)),
        })),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("Preferences".into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        kind: gpui_kit::WindowKind::Normal,
        window_background: gpui_kit::WindowBackgroundAppearance::Opaque,
        ..Default::default()
    };

    let (handle, _entity) = gpui_kit::open_window(options, cx, move |_window, cx| {
        cx.new(|_cx| SettingsWindow::new(main.clone()))
    })?;
    Ok(handle)
}

/// Focus the settings window, if it is still open.
pub(crate) fn focus(handle: &AnyWindowHandle, cx: &mut gpui_kit::App) -> gpui_kit::Result<()> {
    handle.update(cx, |_view, window, _cx| {
        window.activate_window();
    })
}
