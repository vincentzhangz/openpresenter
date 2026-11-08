//! A centred modal card on a dimmed overlay.

use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{AnyElement, IntoElement, MouseButton, SharedString, Window, div, px, relative};

use crate::ui::theme;

/// Turn a dialog title into an id slug: lowercased, non-alphanumerics
/// become dashes, with repeats collapsed and ends trimmed.
fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                slug.push(lower);
            }
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("dialog");
    }
    slug
}

/// A centered dialog card with a backdrop.
///
/// The card is bounded by the screen and its body scrolls, so a dialog
/// taller than the window keeps its buttons on screen. Clicks inside the
/// card stop before they reach the backdrop that closes the dialog.
#[allow(clippy::type_complexity)]
pub fn dialog_overlay(
    title: impl Into<SharedString>,
    content: AnyElement,
    actions: Vec<AnyElement>,
    on_backdrop: Option<Box<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App)>>,
) -> impl IntoElement {
    let title: SharedString = title.into();
    let slug = slugify(&title);
    let card_id = SharedString::from(format!("dialog-card-{slug}"));
    let body_id = SharedString::from(format!("dialog-body-{slug}"));
    let body = div()
        .id(body_id)
        .test_support()
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .child(content);
    let mut card = div()
        .id(card_id)
        .test_support()
        .flex()
        .flex_col()
        .min_w(px(320.))
        .max_h(relative(0.9))
        .overflow_hidden()
        .p(px(20.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme::c(theme::BORDER_STRONG))
        .bg(theme::c(theme::BG_TOOLBAR))
        // A click inside the card belongs to the card. The backdrop closes
        // the dialog on click, and a click inside the card reaches it too
        // unless the card stops it — an operator pressing a control inside
        // the panel would have the panel close under their finger.
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .on_click(|_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(title),
        )
        .child(div().h(px(12.)))
        .child(body)
        .child(div().h(px(16.)));

    if !actions.is_empty() {
        let mut row = div().flex().flex_row().gap(px(8.)).justify_end();
        for action in actions {
            row = row.child(action);
        }
        card = card.child(row);
    }

    let overlay = div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui_kit::hsla(0.0, 0.0, 0.0, 0.6))
        .child(card);

    let overlay = overlay.id("dialog-backdrop");
    if let Some(on_backdrop) = on_backdrop {
        overlay.on_click(on_backdrop)
    } else {
        overlay
    }
}

/// A confirmation dialog.
#[allow(clippy::type_complexity)]
pub fn confirm_dialog(
    title: impl Into<SharedString>,
    message: impl Into<SharedString>,
    confirm_label: impl Into<SharedString>,
    on_confirm: std::rc::Rc<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App)>,
    on_cancel: std::rc::Rc<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App)>,
    danger: bool,
) -> impl IntoElement {
    use gpui_kit::component::button::{Button, ButtonVariants};

    let confirm = if danger {
        Button::new("dialog-confirm")
            .label(confirm_label)
            .danger()
            .on_click({
                let on_confirm = on_confirm.clone();
                move |event: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut gpui_kit::App| {
                    on_confirm(event, window, cx);
                }
            })
    } else {
        Button::new("dialog-confirm")
            .label(confirm_label)
            .primary()
            .on_click({
                let on_confirm = on_confirm.clone();
                move |event: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut gpui_kit::App| {
                    on_confirm(event, window, cx);
                }
            })
    };
    let cancel = Button::new("dialog-cancel")
        .label("Cancel")
        .secondary()
        .on_click({
            let on_cancel = on_cancel.clone();
            move |event: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut gpui_kit::App| {
                on_cancel(event, window, cx);
            }
        });

    let message: SharedString = message.into();
    let content = div()
        .text_size(px(13.))
        .text_color(theme::c(theme::TEXT_SECONDARY))
        .child(message)
        .into_any_element();

    dialog_overlay(
        title,
        content,
        vec![cancel.into_any_element(), confirm.into_any_element()],
        None,
    )
}
