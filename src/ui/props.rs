//! Props panel (overlays, lower thirds, masks, looks) for the right dock's
//! Props tab.
//!
//! Ported from the iced-era `ui/props/mod.rs`. The four text fields (new prop
//! name, lower-third title/subtitle, look name) are GPUI `InputState`
//! entities owned by [`MainWindow`](crate::ui::main_window::MainWindow) and
//! mirrored into `PropsState` via subscriptions.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::{Context, Div, IntoElement, ParentElement, SharedString, Styled, Window, div, px};

use crate::domain::Transition;
use crate::domain::props::{Look, Mask, Prop, PropContent, PropManager};
use crate::render::{Overlay, StyleOverride};
use crate::ui::main_window::MainWindow;
use crate::ui::theme;

/// Messages owned by the Props feature module. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Props(..)`.
/// `SetMask` and `TogglePropsPanel` stay as root variants (they are global
/// rendering/navigation controls, not props-feature state).
#[derive(Debug, Clone)]
pub enum Message {
    Toggle(String),
    Remove(String),
    AddText,
    AddImage,
    NewNameChanged(String),
    /// Edit the text of the prop with this id.
    EditText(String),
    /// Stop editing a prop.
    CancelEdit,
    /// Choose the image that frames the output.
    PickCustomMask,
    LowerThirdTitleChanged(String),
    LowerThirdSubtitleChanged(String),
    CreateLowerThird,
    ApplyLook(String),
    RemoveLook(String),
    SaveLook,
    LookNameChanged(String),
}

/// Dispatch a props message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::Toggle(id) => prop_toggle(w, id, cx),
        Message::Remove(id) => prop_remove(w, id, cx),
        Message::AddText => prop_add_text(w, cx),
        Message::AddImage => prop_add_image(w, cx),
        Message::NewNameChanged(name) => new_name_changed(w, name, cx),
        Message::EditText(id) => prop_edit_text(w, id, cx),
        Message::CancelEdit => prop_cancel_edit(w, cx),
        Message::PickCustomMask => pick_custom_mask(w, cx),
        Message::LowerThirdTitleChanged(t) => lower_third_title_changed(w, t, cx),
        Message::LowerThirdSubtitleChanged(s) => lower_third_subtitle_changed(w, s, cx),
        Message::CreateLowerThird => create_lower_third(w, cx),
        Message::ApplyLook(id) => apply_look(w, id, cx),
        Message::RemoveLook(id) => remove_look(w, id, cx),
        Message::SaveLook => save_look(w, cx),
        Message::LookNameChanged(name) => look_name_changed(w, name, cx),
    }
}

pub(crate) fn prop_toggle(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.props.manager.toggle_prop(&id);
    push_overlay(w);
    // The visibility is part of the prop library, so it is saved.
    w.save_props();
    cx.notify();
}

/// Show or hide one prop.
///
/// The prop's visibility is part of the prop library, so it is saved: the
/// operator's logo is on for the next show as well.
pub(crate) fn set_prop_visible(w: &mut MainWindow, id: &str, visible: bool) {
    if let Some(prop) = w.props.manager.props.iter_mut().find(|p| p.id == id) {
        prop.visible = visible;
    }
    w.save_props();
}

/// The overlay a specific screen shows.
///
/// A look configures each screen separately: the projector can show the
/// lower thirds while the broadcast stream shows only the slide, or the
/// other way round. The screen's target decides which layers are drawn.
pub(crate) fn active_overlay_for_screen(w: &MainWindow, screen_id: &str) -> Overlay {
    let mut overlay = overlay_for_screen(&w.props.manager, screen_id);
    // The operator's layer switches are a master switch on top of the
    // screen's own configuration: clearing the slide layer blanks the slide
    // on every screen that would show it.
    apply_layer_switches(
        &mut overlay,
        w.presenting.slide_layer_active,
        w.presenting.media_layer_active,
    );
    overlay
}

/// Apply the operator's layer switches to a screen's overlay.
pub(crate) fn apply_layer_switches(
    overlay: &mut Overlay,
    slide_layer_active: bool,
    media_layer_active: bool,
) {
    overlay.show_slide &= slide_layer_active;
    overlay.show_media &= media_layer_active;
}

/// The overlay a screen shows, resolved from the prop manager.
///
/// Split out from [`active_overlay_for_screen`] so the rule can be tested
/// without a window.
pub(crate) fn overlay_for_screen(manager: &PropManager, screen_id: &str) -> Overlay {
    let look = manager
        .looks
        .iter()
        .find(|look| Some(&look.id) == manager.active_look_id.as_ref());

    let target = look.map(|look| look.target_or_default(screen_id));

    // The prop's own switch is the truth about what is on the output: the
    // live look's membership is kept in step with it, so a prop the panel
    // shows as "On" is drawn.
    let props = manager
        .props
        .iter()
        .filter(|prop| prop.visible)
        .cloned()
        .collect();

    // The live mask is the operator's control; applying a look sets it.
    let mask = manager.active_mask.clone();

    // The screen's target switches the layers off; an unconfigured screen
    // falls back to showing everything.
    let (show_slide, show_media, show_props, show_messages, show_announcements, show_mask) =
        match &target {
            Some(target) => (
                target.slide_enabled,
                target.media_enabled,
                target.props_enabled,
                target.messages_enabled,
                target.announcements_enabled,
                target.mask_enabled,
            ),
            None => (true, true, true, true, true, true),
        };

    Overlay {
        props: if show_props { props } else { Vec::new() },
        mask: if show_mask { mask } else { Mask::None },
        live_message: if show_messages {
            manager
                .live_message
                .clone()
                .filter(|message| message.visible)
        } else {
            None
        },
        announcement: if show_announcements {
            manager
                .announcement
                .clone()
                .filter(|message| message.visible)
        } else {
            None
        },
        show_slide,
        show_media,
    }
}

/// The style a screen draws the slide in.
///
/// A screen can be assigned a theme in the Looks & Screens matrix; the slide
/// itself is not changed, only how that screen draws it.
pub(crate) fn style_for_screen(w: &MainWindow, screen_id: &str) -> StyleOverride {
    let manager = &w.props.manager;
    let Some(look) = manager
        .looks
        .iter()
        .find(|look| Some(&look.id) == manager.active_look_id.as_ref())
    else {
        return StyleOverride::default();
    };
    let target = look.target_or_default(screen_id);
    let Some(theme_id) = target.theme_id else {
        return StyleOverride::default();
    };
    let Some(theme) = w.theme_state.list.iter().find(|theme| theme.id == theme_id) else {
        return StyleOverride::default();
    };

    StyleOverride {
        background: Some(theme.background.clone()),
        text_style: Some(theme.default_text_style.clone()),
        // A theme that asks for a cut has not chosen the motion: the slide's
        // own transition, or the operator's default, decides.
        transition: match theme.default_transition {
            Transition::Cut => None,
            other => Some(other),
        },
    }
}

/// Set the mask that frames the output.
///
/// The mask is a master frame over the slide: letterbox bars, an oval or a
/// border. It applies to every screen that shows the mask layer.
pub(crate) fn set_mask(w: &mut MainWindow, mask: Mask, cx: &mut Context<MainWindow>) {
    w.props.manager.active_mask = mask;

    // Picking a mask means the operator wants to see it, so the mask layer is
    // switched on for every screen of the active look.
    let screen_ids = crate::ui::output::screen_ids(w);
    if let Some(look_id) = w.props.manager.active_look_id.clone()
        && let Some(look) = w
            .props
            .manager
            .looks
            .iter_mut()
            .find(|look| look.id == look_id)
    {
        for screen_id in screen_ids {
            let mut target = look.target_or_default(&screen_id);
            target.mask_enabled = true;
            look.set_screen_target(target);
        }
    }
    // The broadcast composites the slide itself, so it has to be told.
    push_overlay(w);
    w.save_props();
    cx.notify();
}

/// The screen the NDI broadcast belongs to.
///
/// There is one broadcast sender, so it takes the first NDI output's
/// configuration.
pub(crate) fn ndi_screen_id(w: &MainWindow) -> String {
    w.output
        .manager
        .iter()
        .find(|output| matches!(output.output_type, crate::output::OutputType::Ndi { .. }))
        .map(|output| output.id.clone())
        .unwrap_or_else(|| String::from("stream"))
}

/// Push the current overlay to the running NDI output.
///
/// The NDI sender composes the slide itself, so it has to be told about the
/// overlay separately from the slide.
pub(crate) fn push_overlay(w: &mut MainWindow) {
    // Every sender shows the same slide, but each with the look of its own
    // screen: the projector and the broadcast can carry different props.
    for (id, ndi) in &w.presenting.ndi_outputs {
        let screen_overlay = active_overlay_for_screen(w, id);
        let screen_style = style_for_screen(w, id);
        ndi.set_overlay(screen_overlay);
        ndi.set_style(screen_style);
        ndi.set_default_transition(w.presenting.global_transition);
        ndi.set_reduced_motion(w.ui.reduce_motion);
    }
}

/// A fingerprint of the overlay content.
///
/// The compositor compares this against the previous one so a prop, a look or
/// a live message change triggers a new composition.
pub(crate) fn overlay_fingerprint(w: &MainWindow, screen_id: &str) -> u64 {
    let manager = &w.props.manager;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };

    // The props this screen shows: the look decides which ones are active,
    // and a prop's own visibility still counts.
    let look = manager
        .looks
        .iter()
        .find(|look| Some(&look.id) == manager.active_look_id.as_ref());
    for prop in &manager.props {
        let in_look = look
            .map(|look| look.active_prop_ids.contains(&prop.id))
            .unwrap_or(true);
        if !in_look {
            continue;
        }
        mix(prop.id.as_bytes());
        mix(&[u8::from(prop.visible)]);
        mix(format!("{:?}", prop.content).as_bytes());
    }

    mix(manager.active_look_id.as_deref().unwrap_or("").as_bytes());
    mix(format!("{:?}", manager.active_mask).as_bytes());
    // The operator's layer switches change what the screen shows, so they
    // have to be part of the fingerprint.
    mix(&[
        u8::from(w.presenting.slide_layer_active),
        u8::from(w.presenting.media_layer_active),
    ]);
    if let Some(message) = &manager.live_message {
        mix(message.text.as_bytes());
        mix(&[u8::from(message.visible)]);
    }

    // The screen's own configuration: switching a layer on or off, or giving
    // the screen a theme, changes what that screen shows.
    if let Some(look) = look {
        let target = look.target_or_default(screen_id);
        mix(&[
            u8::from(target.media_enabled),
            u8::from(target.slide_enabled),
            u8::from(target.props_enabled),
            u8::from(target.messages_enabled),
            u8::from(target.mask_enabled),
        ]);
        mix(target.theme_id.as_deref().unwrap_or("").as_bytes());
        // The theme's content, not only its id: re-importing a theme with the
        // same id but a different look has to reach the screen.
        if let Some(theme) = w
            .theme_state
            .list
            .iter()
            .find(|theme| Some(&theme.id) == target.theme_id.as_ref())
        {
            mix(format!("{:?}", theme.background).as_bytes());
            mix(format!("{:?}", theme.default_text_style).as_bytes());
            mix(format!("{:?}", theme.default_transition).as_bytes());
        }
    }
    hash
}

pub(crate) fn prop_remove(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.props.manager.remove_prop(&id);
    w.save_props();
    push_overlay(w);
    cx.notify();
}

pub(crate) fn prop_add_text(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let text = w.props.new_prop_name.trim().to_string();
    // A prop that is being edited is updated, not duplicated: the operator
    // changes the text of the overlay that is already on the screen.
    if let Some(id) = w.props.editing_prop_id.clone() {
        if let Some(prop) = w.props.manager.props.iter_mut().find(|p| p.id == id) {
            if let PropContent::Text {
                text: prop_text, ..
            } = &mut prop.content
            {
                *prop_text = text.clone();
            }
            prop.name = if text.is_empty() {
                prop.name.clone()
            } else {
                text.clone()
            };
        }
        w.props.editing_prop_id = None;
        w.props.new_prop_name.clear();
        w.save_props();
        push_overlay(w);
        cx.notify();
        return;
    }
    let name = if text.is_empty() {
        String::from("New Text Prop")
    } else {
        text.clone()
    };
    let prop = Prop {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        content: PropContent::Text {
            // The text the operator typed is the text on the screen: a prop
            // that says "Text" is a prop nobody asked for.
            text: name,
            font_size: 32.0,
            color: [1.0, 1.0, 1.0, 1.0],
            bold: false,
            italic: false,
        },
        x: 0.1,
        y: 0.1,
        width: 0.8,
        height: 0.1,
        visible: true,
    };
    w.props.manager.add_prop(prop);
    w.props.new_prop_name.clear();
    w.save_props();
    push_overlay(w);
    cx.notify();
}

/// Edit the text of a prop.
///
/// The prop's text goes into the field, so the operator can change what the
/// overlay says.
pub(crate) fn prop_edit_text(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    let Some(prop) = w.props.manager.props.iter().find(|p| p.id == id) else {
        return;
    };
    let text = match &prop.content {
        PropContent::Text { text, .. } => text.clone(),
        // Only text carries text; the other kinds are edited elsewhere.
        _ => return,
    };
    w.props.editing_prop_id = Some(id);
    w.props.new_prop_name = text;
    cx.notify();
}

/// Stop editing a prop.
pub(crate) fn prop_cancel_edit(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.props.editing_prop_id = None;
    w.props.new_prop_name.clear();
    cx.notify();
}

pub(crate) fn prop_add_image(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let path = rfd::FileDialog::new()
        .set_title("Select Image for Prop")
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
        .pick_file();

    if let Some(p) = path {
        let filename = p
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Image")
            .to_string();
        let name = if w.props.new_prop_name.trim().is_empty() {
            filename
        } else {
            w.props.new_prop_name.trim().to_string()
        };
        let prop = Prop {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            content: PropContent::Image {
                path: p.to_string_lossy().into_owned(),
            },
            x: 0.80,
            y: 0.03,
            width: 0.15,
            height: 0.10,
            visible: true,
        };
        w.props.manager.add_prop(prop);
        w.props.new_prop_name.clear();
    }
    w.save_props();
    push_overlay(w);
    cx.notify();
}

pub(crate) fn new_name_changed(w: &mut MainWindow, name: String, cx: &mut Context<MainWindow>) {
    w.props.new_prop_name = name;
    // A prop that is being edited follows the field as the operator types:
    // the overlay on the projector is the text they can see.
    if let Some(id) = w.props.editing_prop_id.clone()
        && let Some(prop) = w.props.manager.props.iter_mut().find(|p| p.id == id)
        && let PropContent::Text { text, .. } = &mut prop.content
    {
        *text = w.props.new_prop_name.clone();
        push_overlay(w);
        w.save_props();
    }
    cx.notify();
}

pub(crate) fn lower_third_title_changed(
    w: &mut MainWindow,
    title: String,
    _cx: &mut Context<MainWindow>,
) {
    w.props.lower_third_title = title;
}

pub(crate) fn lower_third_subtitle_changed(
    w: &mut MainWindow,
    subtitle: String,
    _cx: &mut Context<MainWindow>,
) {
    w.props.lower_third_subtitle = subtitle;
}

pub(crate) fn create_lower_third(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // A lower third is one thing on the screen: creating a second one would
    // leave the first on the projector under the new one.
    let previous: Vec<String> = w
        .props
        .manager
        .props
        .iter()
        .filter(|prop| prop.name.starts_with("LT "))
        .map(|prop| prop.id.clone())
        .collect();
    for id in previous {
        w.props.manager.remove_prop(&id);
    }
    let props = Prop::new_lower_third(&w.props.lower_third_title, &w.props.lower_third_subtitle);
    for prop in props {
        w.props.manager.add_prop(prop);
    }
    // The fields are cleared, so the next lower third starts from an empty
    // title instead of repeating the last one.
    w.props.lower_third_title.clear();
    w.props.lower_third_subtitle.clear();
    w.save_props();
    push_overlay(w);
    cx.notify();
}

pub(crate) fn apply_look(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    // The same path the matrix takes, so the two panels always agree about
    // which look is live: the matrix highlights the live look, and a look
    // chosen here has to be the one it highlights.
    crate::ui::output::select_look(w, id, cx);
}

pub(crate) fn remove_look(w: &mut MainWindow, id: String, cx: &mut Context<MainWindow>) {
    w.props.manager.remove_look(&id);
    // The matrix highlights the live look, so it has to follow the one that
    // is really live.
    w.output.selected_look_id = w.props.manager.active_look_id.clone();
    w.save_props();
    // Removing the live look changes what the screens show, so the
    // broadcast has to be told.
    push_overlay(w);
    cx.notify();
}

pub(crate) fn save_look(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let name = if w.props.new_look_name.trim().is_empty() {
        format!("Look {}", w.props.manager.looks.len() + 1)
    } else {
        w.props.new_look_name.trim().to_string()
    };
    w.props.manager.save_look(name);
    w.props.new_look_name.clear();
    w.save_props();
    push_overlay(w);
    cx.notify();
}

pub(crate) fn look_name_changed(w: &mut MainWindow, name: String, _cx: &mut Context<MainWindow>) {
    w.props.new_look_name = name;
}

/// Mirror the props panel's text field into the field itself.
///
/// Runs on every render; the field is only rewritten when its value diverged
/// from the prop being edited, which happens when the operator picks a prop
/// to edit.
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    sync_text(&w.props_new_name_input, &w.props.new_prop_name, window, cx);
    sync_text(
        &w.props_lower_third_title_input,
        &w.props.lower_third_title,
        window,
        cx,
    );
    sync_text(
        &w.props_lower_third_subtitle_input,
        &w.props.lower_third_subtitle,
        window,
        cx,
    );
}

/// Write a value into a field when the field does not hold it.
fn sync_text(
    entity: &gpui_kit::Entity<gpui_kit::component::input::InputState>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) {
    crate::ui::components::sync_text_field(entity, value, window, cx);
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

/// Render the props panel (right dock Props tab).
pub fn props_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let new_name_input = w.props_new_name_input.clone();
    let lt_title_input = w.props_lower_third_title_input.clone();
    let lt_subtitle_input = w.props_lower_third_subtitle_input.clone();
    let look_name_input = w.props_look_name_input.clone();
    let mask = w.props.manager.active_mask.clone();
    let props: Vec<Prop> = w.props.manager.props.clone();
    let looks: Vec<Look> = w.props.manager.looks.clone();
    // The prop whose text is being edited, if any.
    let editing_prop = w.props.editing_prop_id.clone();

    let mut props_list = div().flex().flex_col().gap(px(2.));
    for prop in &props {
        props_list = props_list.child(prop_row(prop, cx));
    }

    let mut looks_list = div().flex().flex_col().gap(px(2.));
    for look in &looks {
        looks_list = looks_list.child(look_row(look, cx));
    }

    let mut edit_row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .w(px(120.))
                .child(gpui_kit::component::input::Input::new(&new_name_input)),
        )
        // While a prop is being edited the same field and the same
        // button edit it: the operator does not learn a second way to
        // change a prop's text.
        .child(small_button(
            if editing_prop.is_some() {
                "props-update-text"
            } else {
                "props-add-text"
            },
            if editing_prop.is_some() {
                "Update"
            } else {
                "+ Text"
            },
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::Props(Message::AddText),
                    window,
                    cx,
                );
            }),
        ))
        .child(small_button(
            "props-add-image",
            "+ Image",
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::Props(Message::AddImage),
                    window,
                    cx,
                );
            }),
        ));
    if editing_prop.is_some() {
        edit_row = edit_row.child(small_button(
            "props-cancel-edit",
            "Cancel",
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::Props(Message::CancelEdit),
                    window,
                    cx,
                );
            }),
        ));
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(8.))
        .p(px(8.))
        .child(panel_title("Props"))
        .child(props_list)
        .child(edit_row)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme::c(theme::TEXT_PRIMARY))
                        .child("Lower Third"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .w(px(120.))
                                .child(gpui_kit::component::input::Input::new(&lt_title_input)),
                        )
                        .child(
                            div()
                                .w(px(120.))
                                .child(gpui_kit::component::input::Input::new(&lt_subtitle_input)),
                        )
                        .child(
                            Button::new("props-create-lt")
                                .label("Create")
                                .primary()
                                .small()
                                .on_click(cx.listener(|w, _, window, cx| {
                                    w.handle_message(
                                        crate::ui::messages::Message::Props(
                                            Message::CreateLowerThird,
                                        ),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                ),
        )
        .child(mask_row(mask, cx))
        .child(panel_title("Looks"))
        .child(looks_list)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .w(px(140.))
                        .child(gpui_kit::component::input::Input::new(&look_name_input)),
                )
                .child(
                    Button::new("props-save-look")
                        .label("Save Look")
                        .primary()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                crate::ui::messages::Message::Props(Message::SaveLook),
                                window,
                                cx,
                            );
                        })),
                ),
        )
}

fn panel_title(label: &'static str) -> Div {
    div()
        .text_size(px(14.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .child(label)
}

fn prop_row(prop: &Prop, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let vis_label = if prop.visible { "On" } else { "Off" };
    let type_label = match &prop.content {
        PropContent::Text { .. } => "T",
        PropContent::Image { .. } => "Img",
        PropContent::Rectangle { .. } => "R",
    };
    let prop_id = prop.id.clone();
    let prop_id_remove = prop.id.clone();
    let prop_id_edit = prop.id.clone();
    // Only a text prop carries text; the other kinds have nothing to edit
    // here, so they get no edit button.
    let editable = matches!(prop.content, PropContent::Text { .. });

    let mut row = div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            Button::new(SharedString::from(format!("prop-vis-{}", prop.id)))
                .label(vis_label)
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Props(Message::Toggle(prop_id.clone())),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            div()
                .w(px(24.))
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(type_label),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(prop.name.clone()),
        );

    if editable {
        row = row.child(
            Button::new(SharedString::from(format!("prop-edit-{}", prop.id)))
                .label("Edit")
                .ghost()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Props(Message::EditText(
                            prop_id_edit.clone(),
                        )),
                        window,
                        cx,
                    );
                })),
        );
    }

    row.child(
        Button::new(SharedString::from(format!("prop-del-{}", prop.id)))
            .label("X")
            .danger()
            .xsmall()
            .on_click(cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    crate::ui::messages::Message::Props(Message::Remove(prop_id_remove.clone())),
                    window,
                    cx,
                );
            })),
    )
}

fn look_row(look: &Look, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let look_id = look.id.clone();
    let look_id_remove = look.id.clone();
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(look.name.clone()),
        )
        .child(
            Button::new(SharedString::from(format!("look-apply-{}", look.id)))
                .label("Apply")
                .primary()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Props(Message::ApplyLook(look_id.clone())),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new(SharedString::from(format!("look-del-{}", look.id)))
                .label("X")
                .danger()
                .xsmall()
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        crate::ui::messages::Message::Props(Message::RemoveLook(
                            look_id_remove.clone(),
                        )),
                        window,
                        cx,
                    );
                })),
        )
}

fn mask_row(mask: Mask, cx: &mut Context<MainWindow>) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(format!("Mask: {mask}")),
        )
        .child(div().flex_1())
        .child(mask_btn("mask-none", "None", cx))
        .child(mask_btn("mask-letterbox", "Letterbox", cx))
        .child(mask_btn("mask-oval", "Oval", cx))
        .child(mask_btn("mask-border", "Border", cx))
        .child(custom_mask_btn(cx))
}

/// The button that picks a custom mask image.
///
/// A mask the renderer honours and the panel cannot reach is a mask nobody
/// can use: the operator frames the output with their own image.
fn custom_mask_btn(cx: &mut Context<MainWindow>) -> Button {
    Button::new("mask-custom")
        .label("Image…")
        .ghost()
        .xsmall()
        .on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(
                crate::ui::messages::Message::Props(Message::PickCustomMask),
                window,
                cx,
            );
        }))
}

/// Choose the image that frames the output.
pub(crate) fn pick_custom_mask(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Choose a mask image")
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
        .pick_file()
    else {
        return;
    };
    set_mask(
        w,
        Mask::Custom {
            path: path.to_string_lossy().into_owned(),
        },
        cx,
    );
}

fn mask_btn(id: &'static str, label: &'static str, cx: &mut Context<MainWindow>) -> Button {
    let msg = match label {
        "Letterbox" => crate::ui::messages::Message::SetMask(Mask::Letterbox { bar_fraction: 0.1 }),
        "Oval" => crate::ui::messages::Message::SetMask(Mask::Oval { feather: 0.05 }),
        "Border" => crate::ui::messages::Message::SetMask(Mask::FrameBorder {
            thickness: 0.02,
            color: [1.0, 1.0, 1.0, 1.0],
        }),
        _ => crate::ui::messages::Message::SetMask(Mask::None),
    };
    Button::new(id)
        .label(label)
        .ghost()
        .xsmall()
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(msg.clone(), window, cx);
        }))
}

/// Small ghost text button used inside the props panel rows.
fn small_button(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
) -> Button {
    Button::new(id)
        .label(label)
        .ghost()
        .xsmall()
        .on_click(on_click)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::props::{PropManager, ScreenLookTarget};

    /// A manager with one visible prop and an active look that shows it.
    fn manager_with_look() -> PropManager {
        let mut manager = PropManager::default();
        let mut look = Look::new("Broadcast");
        look.active_prop_ids = vec![String::from("badge")];
        manager.props.push(Prop {
            id: String::from("badge"),
            name: String::from("Badge"),
            content: PropContent::Rectangle {
                color: [1.0, 0.0, 0.0, 1.0],
                corner_radius: 0.0,
            },
            x: 0.0,
            y: 0.0,
            width: 0.2,
            height: 0.2,
            visible: true,
        });
        manager.add_look(look.clone());
        manager.active_look_id = Some(look.id);
        manager
    }

    #[test]
    fn a_screen_shows_the_looks_props_by_default() {
        let manager = manager_with_look();
        let overlay = overlay_for_screen(&manager, "main");
        assert_eq!(overlay.props.len(), 1);
        assert!(overlay.show_slide);
        assert!(overlay.show_media);
    }

    #[test]
    fn a_screen_can_switch_the_props_off() {
        let mut manager = manager_with_look();
        let look_id = manager.active_look_id.clone().expect("a look is active");
        let look = manager
            .looks
            .iter_mut()
            .find(|look| Some(&look.id) == manager.active_look_id.as_ref())
            .expect("the look exists");
        let mut target = look.target_or_default("stream");
        target.props_enabled = false;
        look.set_screen_target(target);
        assert!(manager.looks.iter().any(|look| look.id == look_id));

        // The projector still shows the props...
        let projector = overlay_for_screen(&manager, "main");
        assert_eq!(projector.props.len(), 1);

        // ...while the broadcast does not.
        let broadcast = overlay_for_screen(&manager, "stream");
        assert!(broadcast.props.is_empty());
    }

    #[test]
    fn the_operator_layer_switches_beat_the_screen_configuration() {
        // Switching the slide layer off blanks the slide.
        let mut overlay = Overlay {
            show_slide: true,
            show_media: true,
            ..Overlay::default()
        };
        apply_layer_switches(&mut overlay, false, true);
        assert!(!overlay.show_slide);
        assert!(overlay.show_media);

        // Switching the media layer off leaves the slide alone.
        let mut overlay = Overlay {
            show_slide: true,
            show_media: true,
            ..Overlay::default()
        };
        apply_layer_switches(&mut overlay, true, false);
        assert!(overlay.show_slide);
        assert!(!overlay.show_media);
    }

    #[test]
    fn a_screen_can_switch_the_slide_and_the_mask_off() {
        let mut manager = manager_with_look();
        let look = manager
            .looks
            .iter_mut()
            .find(|look| Some(&look.id) == manager.active_look_id.as_ref())
            .expect("the look exists");
        let mut target = ScreenLookTarget::default_for_screen("stream");
        target.slide_enabled = false;
        target.mask_enabled = false;
        look.set_screen_target(target);

        let broadcast = overlay_for_screen(&manager, "stream");
        assert!(!broadcast.show_slide);
        assert!(matches!(broadcast.mask, Mask::None));
        // Media is a layer of its own: switching the slide off leaves it on.
        assert!(broadcast.show_media);

        // Switching the media layer off is a separate decision.
        let look = manager
            .looks
            .iter_mut()
            .find(|look| Some(&look.id) == manager.active_look_id.as_ref())
            .expect("the look exists");
        let mut target = look.target_or_default("stream");
        target.media_enabled = false;
        look.set_screen_target(target);
        let broadcast = overlay_for_screen(&manager, "stream");
        assert!(!broadcast.show_media);
    }
}
