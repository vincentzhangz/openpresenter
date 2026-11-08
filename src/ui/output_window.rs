//! The dedicated output window (projector screen).

use std::sync::Arc;

use gpui_kit::prelude::*;
use gpui_kit::{
    AnyWindowHandle, App, Context, Entity, IntoElement, ObjectFit, Window, div, img, px,
};

use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use crate::ui::theme;

/// Shared routing the output windows read.
pub(crate) type RouteSlot = crate::ui::state::RouteSlot;

/// The audience window's root entity.
pub struct OutputWindow {
    slot: Arc<crate::render::frame_slot::FrameSlot>,
    route_slot: Arc<RouteSlot>,
    screen_id: String,
    main: Entity<MainWindow>,
    shown: Option<u64>,
}

impl OutputWindow {
    /// Create the window entity.
    pub(crate) fn new(
        slot: Arc<crate::render::frame_slot::FrameSlot>,
        route_slot: Arc<RouteSlot>,
        screen_id: String,
        main: Entity<MainWindow>,
        cx: &mut Context<Self>,
    ) -> Self {
        let window = Self {
            slot,
            route_slot,
            screen_id,
            main,
            shown: None,
        };
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let Some(entity) = this.upgrade() else {
                    break;
                };
                let stale = cx.read_entity(&entity, |w, _| {
                    let (_, generation, _) = (w.slot.latest(&w.screen_id), 0, 0);
                    let _ = generation;
                    true
                });
                if stale {
                    entity.update(cx, |_w, cx| cx.notify());
                }
            }
        })
        .detach();
        window
    }
}

impl Render for OutputWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Resolve mirrors: a mirror shows its source's picture.
        let routes = self.route_slot.read();
        let mut screen = self.screen_id.clone();
        for _ in 0..8 {
            if let Some(crate::output::OutputContentRoute::Mirror { source_id }) =
                routes.get(&screen)
            {
                screen = source_id.clone();
            } else {
                break;
            }
        }
        let latest = self.slot.latest(&screen);
        self.shown = latest.as_ref().map(|(generation, _)| *generation);

        if let Some((_, image)) = latest {
            div()
                .w_full()
                .h_full()
                .bg(theme::c(0x000000))
                .child(img(image).object_fit(ObjectFit::Contain).w_full().h_full())
        } else {
            div()
                .w_full()
                .h_full()
                .bg(theme::c(0x000000))
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme::c(theme::TEXT_MUTED))
                .text_size(px(13.))
                .child("No signal")
        }
    }
}

/// Open an output window.
pub(crate) fn open(
    slot: Arc<crate::render::frame_slot::FrameSlot>,
    route_slot: Arc<RouteSlot>,
    screen_id: String,
    main: Entity<MainWindow>,
    bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
    _fullscreen: bool,
    cx: &mut App,
) -> anyhow::Result<(AnyWindowHandle, Entity<OutputWindow>)> {
    let options = gpui_kit::WindowOptions {
        window_bounds: Some(gpui_kit::WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some(screen_id.clone().into()),
            ..Default::default()
        }),
        focus: false,
        show: true,
        kind: gpui_kit::WindowKind::Normal,
        window_background: gpui_kit::WindowBackgroundAppearance::Opaque,
        ..Default::default()
    };
    let (handle, entity) = gpui_kit::open_window(options, cx, move |_window, cx| {
        cx.new(|cx| {
            OutputWindow::new(
                Arc::clone(&slot),
                Arc::clone(&route_slot),
                screen_id.clone(),
                main.clone(),
                cx,
            )
        })
    })?;
    Ok((handle, entity))
}

/// Send a message to the main window; used by the window's controls.
fn drive(
    main: Entity<MainWindow>,
    message: Message,
) -> impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static {
    move |_event, window, cx| {
        main.update(cx, |w, cx| w.handle_message(message.clone(), window, cx));
    }
}
