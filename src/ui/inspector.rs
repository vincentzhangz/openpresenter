//! The slide inspector (Edit-mode right dock).
//!
//! Ported from the iced-era `ui/editor/inspector.rs`: the Shape / Text /
//! Build / Theme tab bar and the four tab bodies. The theme controls are
//! composed from `ui/themes.rs` logic, the layer-property editors dispatch
//! `ui/layers.rs` / `ui/typography.rs` messages, and the Text/Build controls
//! dispatch the live `ui/slides.rs` property logic.
//!
//! Because GPUI text inputs and sliders are entity-stateful, every field
//! lives on [`MainWindow`](crate::ui::main_window::MainWindow) and is kept in
//! step with the per-feature state strings by [`sync_inputs`], which runs on
//! each render and only rewrites an entity when its value diverged (so user
//! typing is never clobbered).

use gpui_kit::TestSupportExt;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::component::{Icon, IconName};
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, Context, Div, ElementId, Entity, Hsla, IntoElement, ParentElement,
    SharedString, Stateful, Styled, Window, div, hsla, px, rgb, white,
};

use crate::domain::{
    Background as SlideBackground, Color as SlideColor, ImageFit, Object, ObjectContent, ShapeType,
    Slide, SlideContent, TextAlignment, TextStyle, TextTransform, Transition,
};
use crate::ui::components::{
    color_channel_slider, color_swatch_btn, compact_toggle_btn, divider, option_btn, section_label,
    tab_bar, tab_btn,
};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, Message};
use crate::ui::theme;
use crate::ui::{layers, slides, themes as themes_feature, typography};

// ---------------------------------------------------------------------------
// Input sync
// ---------------------------------------------------------------------------

/// Mirror the per-feature state strings into the inspector / props field
/// entities, and the selected layer's numeric values into the colour-channel
/// sliders. Runs on every render; an entity is only rewritten when its value
/// diverged, which happens exactly when the state was changed from outside
/// the field (selection change, preset button, undo, ...).
pub(crate) fn sync_inputs(w: &mut MainWindow, window: &mut Window, cx: &mut Context<MainWindow>) {
    sync_text(
        &w.inspector_text_input,
        &w.editor.editing_slide_text,
        window,
        cx,
    );
    sync_text(
        &w.inspector_font_size_input,
        &w.editor.editing_slide_font_size,
        window,
        cx,
    );
    sync_text(
        &w.inspector_transition_dur_input,
        &w.editor.editing_transition_duration,
        window,
        cx,
    );
    sync_text(
        &w.inspector_group_label_input,
        &w.editor.editing_group_label,
        window,
        cx,
    );
    sync_text(
        &w.inspector_notes_input,
        &w.editor.editing_slide_notes,
        window,
        cx,
    );
    sync_text(
        &w.themes_name_input,
        &w.theme_state.new_theme_name,
        window,
        cx,
    );
    sync_text(&w.layer_text_input, &w.layer.text, window, cx);
    sync_text(&w.layer_font_size_input, &w.layer.font_size, window, cx);
    sync_text(&w.layer_pos_x_input, &w.layer.pos_x, window, cx);
    sync_text(&w.layer_pos_y_input, &w.layer.pos_y, window, cx);
    sync_text(&w.layer_width_input, &w.layer.width, window, cx);
    sync_text(&w.layer_height_input, &w.layer.height, window, cx);
    sync_text(
        &w.layer_stroke_width_input,
        &w.layer.stroke_width,
        window,
        cx,
    );
    sync_text(
        &w.layer_corner_radius_input,
        &w.layer.corner_radius,
        window,
        cx,
    );
    sync_text(&w.layer_font_family_input, &w.layer.font_family, window, cx);
    sync_text(&w.layer_line_height_input, &w.layer.line_height, window, cx);
    sync_text(
        &w.layer_letter_spacing_input,
        &w.layer.letter_spacing,
        window,
        cx,
    );
    sync_text(&w.layer_glow_radius_input, &w.layer.glow_radius, window, cx);
    sync_text(
        &w.layer_text_stroke_width_input,
        &w.layer.text_stroke_width,
        window,
        cx,
    );
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
    sync_text(&w.props_look_name_input, &w.props.new_look_name, window, cx);

    // Colour-channel sliders read from the selected layer, like the iced
    // sliders did (the layer is the source of truth while dragging).
    let snapshot = layer_snapshot(w);
    if let Some(snap) = &snapshot {
        sync_slider(&w.layer_opacity_slider, snap.opacity, window, cx);
        sync_slider(&w.layer_text_r_slider, f32::from(snap.r), window, cx);
        sync_slider(&w.layer_text_g_slider, f32::from(snap.g), window, cx);
        sync_slider(&w.layer_text_b_slider, f32::from(snap.b), window, cx);
        if let Some([fr, fg, fb, fa]) = snap.fill {
            sync_slider(&w.layer_fill_r_slider, f32::from(fr), window, cx);
            sync_slider(&w.layer_fill_g_slider, f32::from(fg), window, cx);
            sync_slider(&w.layer_fill_b_slider, f32::from(fb), window, cx);
            sync_slider(&w.layer_fill_a_slider, f32::from(fa), window, cx);
        }
        if let Some([sr, sg, sb, sa]) = snap.shape_stroke {
            sync_slider(&w.layer_shape_stroke_r_slider, f32::from(sr), window, cx);
            sync_slider(&w.layer_shape_stroke_g_slider, f32::from(sg), window, cx);
            sync_slider(&w.layer_shape_stroke_b_slider, f32::from(sb), window, cx);
            sync_slider(&w.layer_shape_stroke_a_slider, f32::from(sa), window, cx);
        }
        sync_slider(&w.layer_glow_r_slider, f32::from(snap.glow[0]), window, cx);
        sync_slider(&w.layer_glow_g_slider, f32::from(snap.glow[1]), window, cx);
        sync_slider(&w.layer_glow_b_slider, f32::from(snap.glow[2]), window, cx);
        sync_slider(
            &w.layer_stroke_r_slider,
            f32::from(snap.stroke[0]),
            window,
            cx,
        );
        sync_slider(
            &w.layer_stroke_g_slider,
            f32::from(snap.stroke[1]),
            window,
            cx,
        );
        sync_slider(
            &w.layer_stroke_b_slider,
            f32::from(snap.stroke[2]),
            window,
            cx,
        );
    }

    // The hex field is synced when the value changed; a focused field is
    // left alone by `sync_text_field`.
    if let Some(hex) = snapshot.as_ref().map(|s| s.hex.clone()) {
        if sync_text(&w.layer_text_hex_input, &hex, window, cx) {
            w.layer.text_hex_sync = true;
        }
    } else {
        w.layer.text_hex_sync = false;
    }

    sync_slider(&w.video_volume_slider, w.video.volume, window, cx);

    // The seek slider is normalised to 0–100; map the playback position into
    // that range so the thumb tracks the player. Skipped while the duration
    // is unknown (no player), which also hides the control.
    let video_duration = w
        .video
        .player
        .as_ref()
        .map(crate::media::MediaPlayer::duration_secs)
        .unwrap_or(0.0);
    if video_duration > 0.0 {
        let pos = w.video.position.clamp(0.0, video_duration);
        sync_slider(
            &w.video_seek_slider,
            ((pos / video_duration) * 100.0) as f32,
            window,
            cx,
        );
    }

    sync_slider(&w.audio_volume_slider, w.audio.volume, window, cx);
}

/// Numeric values of the selected layer for the slider sync.
struct LayerSnapshot {
    opacity: f32,
    r: u8,
    g: u8,
    b: u8,
    fill: Option<[u8; 4]>,
    /// The stroke colour of a selected shape.
    shape_stroke: Option<[u8; 4]>,
    glow: [u8; 3],
    stroke: [u8; 3],
    hex: String,
}

fn layer_snapshot(w: &MainWindow) -> Option<LayerSnapshot> {
    let idx = w.layer.selected_index?;
    let slide = w.get_current_slide()?;
    let layer = slide.layers.get(idx)?;
    let (r, g, b, fill, shape_stroke, glow, stroke, hex) = match &layer.content {
        ObjectContent::Text { style, .. } => (
            style.color.r,
            style.color.g,
            style.color.b,
            None,
            None,
            [style.glow_color.r, style.glow_color.g, style.glow_color.b],
            [
                style.text_stroke_color.r,
                style.text_stroke_color.g,
                style.text_stroke_color.b,
            ],
            format!(
                "#{:02X}{:02X}{:02X}",
                style.color.r, style.color.g, style.color.b
            ),
        ),
        ObjectContent::Shape {
            fill, stroke_color, ..
        } => {
            let hex = format!("#{:02X}{:02X}{:02X}", fill.r, fill.g, fill.b);
            (
                fill.r,
                fill.g,
                fill.b,
                Some([fill.r, fill.g, fill.b, fill.a]),
                Some([
                    stroke_color.r,
                    stroke_color.g,
                    stroke_color.b,
                    stroke_color.a,
                ]),
                [255, 255, 255],
                [0, 0, 0],
                hex,
            )
        }
        _ => (
            255,
            255,
            255,
            None,
            None,
            [255, 255, 255],
            [0, 0, 0],
            String::new(),
        ),
    };
    Some(LayerSnapshot {
        opacity: layer.opacity,
        r,
        g,
        b,
        fill,
        shape_stroke,
        glow,
        stroke,
        hex,
    })
}

/// Set the input's value when the state string diverged from it. Typing
/// keeps them equal (the subscription mirrors the value), so this never
/// resets the cursor mid-edit.
/// Write a value into a field when the field does not hold it.
///
/// Returns whether the field was written: a field the operator is typing in
/// is left alone, and the caller has to know that its own idea of the
/// field's value is still out of step.
fn sync_text(
    entity: &Entity<InputState>,
    value: &str,
    window: &mut Window,
    cx: &mut Context<MainWindow>,
) -> bool {
    crate::ui::components::sync_text_field(entity, value, window, cx)
}

/// Slider counterpart of [`sync_text`].
fn sync_slider(
    entity: &Entity<SliderState>,
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

// ---------------------------------------------------------------------------
// Panel
// ---------------------------------------------------------------------------

/// Render the inspector panel (fills the right dock in Edit mode).
pub(crate) fn inspector_panel(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let tab = w.shell.inspector_tab;
    let slide = w.get_current_slide().cloned();

    let tab_bar_el = tab_bar(vec![
        tab_btn(
            "inspector-tab-shape",
            "Shape",
            tab == InspectorTab::Layers,
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::SwitchInspectorTab(InspectorTab::Layers),
                    window,
                    cx,
                );
            }),
        )
        .into_any_element(),
        tab_btn(
            "inspector-tab-text",
            "Text",
            tab == InspectorTab::Text,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SwitchInspectorTab(InspectorTab::Text), window, cx);
            }),
        )
        .into_any_element(),
        tab_btn(
            "inspector-tab-build",
            "Build",
            tab == InspectorTab::Slide,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SwitchInspectorTab(InspectorTab::Slide), window, cx);
            }),
        )
        .into_any_element(),
        tab_btn(
            "inspector-tab-theme",
            "Theme",
            tab == InspectorTab::Theme,
            cx.listener(|w, _, window, cx| {
                w.handle_message(Message::SwitchInspectorTab(InspectorTab::Theme), window, cx);
            }),
        )
        .into_any_element(),
    ]);

    let body: Div = match (&tab, slide.as_ref()) {
        (InspectorTab::Theme, _) => themes_tab(w, cx),
        (InspectorTab::Text, Some(slide)) => text_tab(w, slide, cx),
        (InspectorTab::Slide, Some(slide)) => slide_tab(w, slide, cx),
        (InspectorTab::Layers, Some(slide)) => layers_tab(w, slide, cx),
        _ => empty_body("Select a slide to edit properties"),
    };

    let mut panel = div()
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme::c(theme::BG_PANEL))
        .child(tab_bar_el)
        .child(
            div()
                .id("inspector-body")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(body),
        );

    if slide.is_some() {
        panel = panel.child(
            div()
                .w_full()
                .py(px(10.))
                .px(px(14.))
                .border_t_1()
                .border_color(theme::c(theme::BORDER_PANEL))
                .child(
                    Button::new("inspector-save-slide")
                        .label("Save Slide")
                        .primary()
                        .w_full()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(slides::Message::SaveSlide),
                                window,
                                cx,
                            );
                        })),
                ),
        );
    }

    panel
}

fn empty_body(text: &'static str) -> Div {
    div().p(px(20.)).child(
        div()
            .text_size(px(13.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child(text),
    )
}

// ---------------------------------------------------------------------------
// Text tab
// ---------------------------------------------------------------------------

fn text_tab(w: &mut MainWindow, slide: &Slide, cx: &mut Context<MainWindow>) -> Div {
    let selected_style = w
        .layer
        .selected_index
        .and_then(|idx| slide.layers.get(idx))
        .and_then(|l| match &l.content {
            ObjectContent::Text { style, .. } => Some(style),
            _ => None,
        });
    let first_style = slide.layers.iter().find_map(|l| match &l.content {
        ObjectContent::Text { style, .. } => Some(style),
        _ => None,
    });

    // The style the panel shows has to be the style it writes. The slide's
    // legacy content keeps the style the slide had before its text moved
    // into an object, and nothing writes it back — so showing it would
    // leave the panel displaying a frozen style while its buttons changed
    // the text object. The legacy style is only the one to show when the
    // slide has no objects at all.
    let legacy_style = match &slide.content {
        SlideContent::Text { style, .. } => Some(style),
        _ => None,
    };
    let style = selected_style
        .or(first_style)
        .or_else(|| slide.layers.is_empty().then_some(legacy_style).flatten());

    if let Some(style) = style {
        text_content_tab(w, style, cx)
    } else {
        // A media slide is one media object; the panel edits that object.
        // The file may not be chosen yet, and the panel still has to offer
        // the picker — a media slide without one is a blank the operator
        // cannot fill.
        match slide.media() {
            Some(crate::domain::SlideMedia::Image { path, fit }) => {
                image_content_tab(w, path, fit, cx)
            }
            Some(crate::domain::SlideMedia::Video { path, thumbnail }) => {
                video_content_tab(w, path, thumbnail, cx)
            }
            None => {
                // The slide carries no text: a box that swallows what the
                // operator types is a box that lies. The panel says so and
                // offers the text the slide is missing.
                div()
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(section_label("TEXT CONTENT"))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme::c(theme::TEXT_MUTED))
                            .child("This slide has no text. Add a text object to write one."),
                    )
                    .child(
                        Button::new("inspector-add-text")
                            .label("+ Text")
                            .secondary()
                            .small()
                            .on_click(cx.listener(|w, _, window, cx| {
                                w.handle_message(
                                    Message::Layers(crate::ui::layers::Message::AddTextLayer),
                                    window,
                                    cx,
                                );
                            })),
                    )
            }
        }
    }
}

fn text_content_tab(w: &mut MainWindow, style: &TextStyle, cx: &mut Context<MainWindow>) -> Div {
    let bold = style.bold;
    let italic = style.italic;
    let outline = style.outline;
    let shadow = style.shadow;
    let glow_enabled = style.glow_enabled;
    let alignment = style.alignment;
    let transform = style.text_transform;
    let color = style.color;

    // The object's position is the text's position; the style only carries
    // the look. The position presets act on the object, so they have to
    // start from the object's coordinates.
    let (pos_x, pos_y) = w
        .layer
        .selected_index
        .and_then(|index| {
            w.get_current_slide()
                .and_then(|slide| slide.layers.get(index))
        })
        .map(|object| (object.position_x, object.position_y))
        .unwrap_or((style.position_x, style.position_y));

    let mut panel = div().p(px(10.)).flex().flex_col().gap(px(10.));

    panel = panel.child(section_label("TEXT CONTENT"));
    panel = panel.child(Input::new(&w.inspector_text_input.clone()));

    panel = panel
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.))
                .child(
                    Button::new("inspector-copy-style")
                        .label("Copy Style")
                        .secondary()
                        .small()
                        .flex_1()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(slides::Message::CopyTextStyle),
                                window,
                                cx,
                            );
                        })),
                )
                .child(paste_style_button(w, cx)),
        )
        .child(divider());

    panel = panel.child(faux_dropdown(
        "inspector-font-family",
        style.font_family.clone(),
        cx.listener(|w, _, window, cx| {
            w.handle_message(
                Message::Slides(slides::Message::SlideFontFamilyCycle),
                window,
                cx,
            );
        }),
    ));

    panel =
        panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    Button::new("inspector-weight")
                        .label(if bold { "Bold" } else { "Regular" })
                        .secondary()
                        .small()
                        .flex_1()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(slides::Message::SlideBoldToggled(!bold)),
                                window,
                                cx,
                            );
                        })),
                )
                .child(div().w(px(64.)).child(
                    Input::new(&w.inspector_font_size_input.clone()).aria_label("Font size"),
                ))
                .child(
                    Button::new("inspector-fs-minus")
                        .icon(Icon::new(IconName::Minus))
                        .secondary()
                        .xsmall()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(slides::Message::SlideFontSizeStep(-4.0)),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("inspector-fs-plus")
                        .icon(Icon::new(IconName::Plus))
                        .secondary()
                        .xsmall()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Slides(slides::Message::SlideFontSizeStep(4.0)),
                                window,
                                cx,
                            );
                        })),
                ),
        );

    // Style flag toggles + glow.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.))
            .child(compact_toggle_btn(
                "inspector-bold",
                "B",
                bold,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideBoldToggled(!bold)),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "inspector-italic",
                "I",
                italic,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideItalicToggled(!italic)),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "inspector-outline",
                "U",
                outline,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideOutlineToggled(!outline)),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "inspector-shadow",
                "S",
                shadow,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideShadowToggled(!shadow)),
                        window,
                        cx,
                    );
                }),
            ))
            .child(div().flex_1())
            .child(
                Button::new("inspector-glow-sun")
                    .icon(Icon::new(IconName::Sun).when(glow_enabled, |icon| {
                        icon.text_color(theme::c(theme::ACCENT_ORANGE))
                    }))
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            Message::Slides(slides::Message::SlideGlowToggled(!glow_enabled)),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    // Capitalization.
    let cap_label = transform.label();
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child("Capitalization"),
            )
            .child(div().flex_1())
            .child(faux_dropdown(
                "inspector-transform",
                cap_label,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideTextTransformCycle),
                        window,
                        cx,
                    );
                }),
            )),
    );

    panel = panel.child(divider());

    // Text colour swatch + palette cycle button.
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme::c(theme::TEXT_PRIMARY))
                    .child("Text Color"),
            )
            .child(div().flex_1())
            .child(color_swatch_btn(
                "inspector-color-current",
                slide_color_hsla(&color),
                false,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideTextColorCycle),
                        window,
                        cx,
                    );
                }),
            ))
            .child(
                Button::new("inspector-color-palette")
                    .icon(Icon::new(IconName::Palette))
                    .secondary()
                    .xsmall()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            Message::Slides(slides::Message::SlideTextColorCycle),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    let mut color_row = div().flex().flex_row().items_center().gap(px(4.));
    for (i, preset) in slides::COLOR_PRESETS.iter().enumerate() {
        color_row = color_row.child(color_swatch_btn(
            SharedString::from(format!("inspector-color-{i}")),
            slide_color_hsla(preset),
            *preset == color,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideColorChanged(*preset)),
                    window,
                    cx,
                );
            }),
        ));
    }
    panel = panel.child(color_row);

    panel = panel.child(divider());

    // Alignment.
    let mut align_row = div().flex().flex_row().gap(px(4.));
    for (i, (label, value)) in [
        ("Left", TextAlignment::Left),
        ("Center", TextAlignment::Center),
        ("Right", TextAlignment::Right),
    ]
    .into_iter()
    .enumerate()
    {
        let active = std::mem::discriminant(&value) == std::mem::discriminant(&alignment);
        align_row = align_row.child(compact_toggle_btn(
            SharedString::from(format!("inspector-align-{i}")),
            label,
            active,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideAlignmentChanged(value)),
                    window,
                    cx,
                );
            }),
        ));
    }
    panel = panel.child(align_row);

    // Vertical position presets (keep the current X).
    let top_active = pos_y <= 0.30;
    let middle_active = (pos_y - 0.5).abs() < 0.20;
    let bottom_active = pos_y >= 0.70;
    let mut pos_row = div().flex().flex_row().gap(px(4.));
    for (i, (icon, active, y)) in [
        (IconName::ArrowUp, top_active, 0.15_f32),
        (IconName::ChevronsUpDown, middle_active, 0.5),
        (IconName::ArrowDown, bottom_active, 0.85),
    ]
    .into_iter()
    .enumerate()
    {
        pos_row = pos_row.child(preset_icon_btn(
            SharedString::from(format!("inspector-pos-{i}")),
            icon,
            active,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlidePositionPreset(pos_x, y)),
                    window,
                    cx,
                );
            }),
        ));
    }
    panel = panel.child(pos_row);

    panel = panel
        .child(divider())
        .child(flag_row(
            "inspector-stroke-flag",
            "Stroke",
            outline,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideOutlineToggled(!outline)),
                    window,
                    cx,
                );
            }),
        ))
        .child(divider())
        .child(flag_row(
            "inspector-shadow-flag",
            "Shadow",
            shadow,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideShadowToggled(!shadow)),
                    window,
                    cx,
                );
            }),
        ))
        .child(divider())
        .child(flag_row(
            "inspector-glow-flag",
            "Glow Effect",
            glow_enabled,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideGlowToggled(!glow_enabled)),
                    window,
                    cx,
                );
            }),
        ))
        .child(div().h(px(40.)));

    panel
}

/// Paste Style button: only clickable when a style has been copied.
fn paste_style_button(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Button {
    let can_paste = w.editor.copied_style.is_some();
    let button = Button::new("inspector-paste-style")
        .label("Paste Style")
        .small()
        .flex_1();
    if can_paste {
        button.secondary().on_click(cx.listener(|w, _, window, cx| {
            w.handle_message(Message::Slides(slides::Message::PasteTextStyle), window, cx);
        }))
    } else {
        button.ghost()
    }
}

/// A labelled checkbox-replacement row (Stroke / Shadow / Glow).
fn flag_row(
    id: &'static str,
    label: &'static str,
    checked: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(on_click)
        .child(
            div()
                .w(px(14.))
                .h(px(14.))
                .rounded(px(3.))
                .border_1()
                .border_color(if checked {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    theme::c(theme::BORDER_STRONG)
                })
                .bg(if checked {
                    theme::c(theme::ACCENT_ORANGE)
                } else {
                    gpui_kit::transparent_black()
                })
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |box_el| {
                    box_el.child(Icon::new(IconName::Check).xsmall().text_color(white()))
                }),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(label),
        )
}

/// Small square icon button used by the position presets.
fn preset_icon_btn(
    id: impl Into<ElementId>,
    icon: IconName,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut icon_el = Icon::new(icon).small();
    icon_el = if active {
        icon_el.text_color(white())
    } else {
        icon_el.text_color(theme::c(theme::TEXT_SECONDARY))
    };
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .py(px(4.))
        .px(px(10.))
        .rounded(px(4.))
        .bg(if active {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            theme::c(theme::BG_HOVER)
        })
        .hover(|s| s.bg(theme::c(theme::BG_ACTIVE)))
        .on_click(on_click)
        .child(icon_el)
}

/// Full-width faux dropdown: label + chevron that cycles on click.
fn faux_dropdown(
    id: &'static str,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .py(px(6.))
        .px(px(10.))
        .rounded(px(4.))
        .bg(theme::c(theme::BG_HOVER))
        .border_1()
        .border_color(theme::c(theme::BORDER_PANEL))
        .hover(|s| s.bg(theme::c(theme::BG_ACTIVE)))
        .on_click(on_click)
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_PRIMARY))
                .child(label.into()),
        )
        .child(div().flex_1())
        .child(
            Icon::new(IconName::ChevronDown)
                .small()
                .text_color(theme::c(theme::TEXT_SECONDARY)),
        )
}

// ---------------------------------------------------------------------------
// Image / Video content (Text tab fallbacks)
// ---------------------------------------------------------------------------

fn file_display_name(path: &str) -> String {
    if path.is_empty() {
        String::from("(no file selected)")
    } else {
        std::path::Path::new(path)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string())
    }
}

fn image_content_tab(
    _w: &mut MainWindow,
    path: &str,
    fit: ImageFit,
    cx: &mut Context<MainWindow>,
) -> Div {
    let mut panel = div().p(px(14.)).flex().flex_col().gap(px(12.));

    panel = panel.child(section_label("IMAGE FILE"));
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child(file_display_name(path)),
            )
            .child(
                Button::new("inspector-image-browse")
                    .label("Browse…")
                    .secondary()
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            Message::Slides(slides::Message::PickImageFile),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    panel = panel.child(section_label("IMAGE FIT"));
    let mut fit_row = div().flex().flex_row().gap(px(4.));
    for (i, (label, value)) in [
        ("Fit", ImageFit::Fit),
        ("Fill", ImageFit::Fill),
        ("Stretch", ImageFit::Stretch),
        ("Center", ImageFit::Center),
    ]
    .into_iter()
    .enumerate()
    {
        fit_row = fit_row.child(option_btn(
            SharedString::from(format!("inspector-fit-{i}")),
            label,
            value == fit,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideImageFitChanged(value)),
                    window,
                    cx,
                );
            }),
        ));
    }
    panel = panel.child(fit_row);

    panel
}

fn video_content_tab(
    w: &mut MainWindow,
    path: &str,
    thumbnail: Option<&str>,
    cx: &mut Context<MainWindow>,
) -> Div {
    let playing = w
        .video
        .player
        .as_ref()
        .is_some_and(crate::media::MediaPlayer::is_playing);
    let looping = w.video.looping;
    let muted = w.video.muted;
    let volume = w.video.volume;
    let speed = w.video.speed;
    let position = w.video.position;
    let duration = w
        .video
        .player
        .as_ref()
        .map(crate::media::MediaPlayer::duration_secs)
        .unwrap_or(0.0);

    let mut panel = div().p(px(14.)).flex().flex_col().gap(px(12.));

    panel = panel.child(section_label("VIDEO FILE"));
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_SECONDARY))
                    .child(file_display_name(path)),
            )
            .child(
                Button::new("inspector-video-browse")
                    .label("Browse…")
                    .secondary()
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            Message::Slides(slides::Message::PickVideoFile),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    if !path.is_empty() {
        // Live frame preview: mirror the playing video while the player is
        // active.
        if let Some(frame) = w.video.frame.clone() {
            panel = panel.child(section_label("PREVIEW"));
            panel = panel.child(
                div()
                    .w_full()
                    .h(px(120.))
                    .overflow_hidden()
                    .bg(gpui_kit::black())
                    .border_1()
                    .border_color(theme::c(theme::BORDER_PANEL))
                    .child(
                        gpui_kit::img(frame)
                            .size_full()
                            .object_fit(gpui_kit::ObjectFit::Contain),
                    ),
            );
        }

        panel = panel.child(section_label("PLAYBACK"));
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    Button::new("inspector-video-play")
                        .icon(Icon::new(if playing {
                            IconName::Pause
                        } else {
                            IconName::Play
                        }))
                        .primary()
                        .small()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Video(crate::ui::video::Message::PlayToggled),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("inspector-video-loop")
                        .icon(Icon::new(IconName::RefreshCw))
                        .when(looping, |b| b.primary())
                        .when(!looping, |b| b.secondary())
                        .small()
                        .on_click(cx.listener(move |w, _, window, cx| {
                            w.handle_message(
                                Message::Video(crate::ui::video::Message::LoopToggled(!looping)),
                                window,
                                cx,
                            );
                        })),
                ),
        );

        panel = panel.child(section_label("VOLUME"));
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    Button::new("inspector-video-mute")
                        .label(if muted { "Muted" } else { "Sound" })
                        .when(muted, |b| b.primary())
                        .when(!muted, |b| b.secondary())
                        .xsmall()
                        .on_click(cx.listener(|w, _, window, cx| {
                            w.handle_message(
                                Message::Video(crate::ui::video::Message::MuteToggled),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .child(Slider::new(&w.video_volume_slider.clone())),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child(format!("{:.0}%", volume * 100.0)),
                ),
        );

        if duration > 0.0 {
            let safe_pos = position.clamp(0.0, duration);
            panel = panel.child(section_label("POSITION"));
            panel = panel.child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex_1()
                            .child(Slider::new(&w.video_seek_slider.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::c(theme::TEXT_SECONDARY))
                            .child(format!(
                                "{:.0}:{:02.0} / {:.0}:{:02.0}",
                                (safe_pos / 60.0).floor(),
                                safe_pos % 60.0,
                                (duration / 60.0).floor(),
                                duration % 60.0,
                            )),
                    ),
            );
        }

        panel = panel.child(section_label("SPEED"));
        let mut speed_row = div().flex().flex_row().gap(px(4.));
        for (i, s) in [0.5_f64, 1.0, 1.5, 2.0].into_iter().enumerate() {
            let active = (speed - s).abs() < 0.05;
            speed_row = speed_row.child(option_btn(
                SharedString::from(format!("inspector-speed-{i}")),
                format!("{s}x"),
                active,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Video(crate::ui::video::Message::SpeedChanged(s)),
                        window,
                        cx,
                    );
                }),
            ));
        }
        panel = panel.child(speed_row);
    }

    if let Some(thumb) = thumbnail.filter(|t| !t.is_empty()) {
        panel = panel.child(section_label("THUMBNAIL"));
        panel = panel.child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child(file_display_name(thumb)),
        );
    }

    panel
}

// ---------------------------------------------------------------------------
// Build (slide) tab
// ---------------------------------------------------------------------------

fn slide_tab(w: &mut MainWindow, slide: &Slide, cx: &mut Context<MainWindow>) -> Div {
    let mut panel = div().p(px(14.)).flex().flex_col().gap(px(12.));

    // Content type. The slide's objects decide, because they are the
    // picture: a slide whose objects hold an image is an image slide, and
    // the panel has to say what the slide is.
    let kind = slide.content_kind();
    let is_text = kind == crate::domain::SlideKind::Text;
    let is_image = kind == crate::domain::SlideKind::Image;
    let is_video = kind == crate::domain::SlideKind::Video;
    panel = panel.child(section_label("CONTENT TYPE"));
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .gap(px(4.))
            .child(option_btn(
                "inspector-ct-text",
                "Text",
                is_text,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::ConvertSlideToText),
                        window,
                        cx,
                    );
                }),
            ))
            .child(option_btn(
                "inspector-ct-image",
                "Image",
                is_image,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::ConvertSlideToImage),
                        window,
                        cx,
                    );
                }),
            ))
            .child(option_btn(
                "inspector-ct-video",
                "Video",
                is_video,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::ConvertSlideToVideo),
                        window,
                        cx,
                    );
                }),
            )),
    );

    // Background swatches.
    panel = panel.child(section_label("BACKGROUND"));
    panel = panel.child(background_swatch_row(slide, cx));

    // Transition buttons.
    panel = panel.child(section_label("TRANSITION"));
    let is_cut = matches!(slide.transition, Transition::Cut);
    let dur_ms: u64 = match slide.transition {
        Transition::Cut => 500,
        Transition::Fade { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => duration_ms,
    };

    let row1 = [
        ("Cut", Transition::Cut, is_cut),
        (
            "Fade",
            Transition::Fade {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Fade { .. }),
        ),
        (
            "Dissolve",
            Transition::Dissolve {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Dissolve { .. }),
        ),
        (
            "Slide",
            Transition::Slide {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Slide { .. }),
        ),
    ];
    let row2 = [
        (
            "Push",
            Transition::Push {
                duration_ms: dur_ms,
                direction: 1,
            },
            matches!(slide.transition, Transition::Push { .. }),
        ),
        (
            "Zoom",
            Transition::Zoom {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Zoom { .. }),
        ),
        (
            "Flip",
            Transition::Flip {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Flip { .. }),
        ),
        (
            "Clock",
            Transition::Clock {
                duration_ms: dur_ms,
            },
            matches!(slide.transition, Transition::Clock { .. }),
        ),
        (
            "Wipe",
            Transition::Wipe {
                duration_ms: dur_ms,
                angle_deg: 0,
            },
            matches!(slide.transition, Transition::Wipe { .. }),
        ),
    ];
    let transition_rows = vec![row1.to_vec(), row2.to_vec()];
    for (row_idx, row) in transition_rows.into_iter().enumerate() {
        let mut row_el = div().flex().flex_row().gap(px(4.));
        for (i, (label, transition, active)) in row.into_iter().enumerate() {
            row_el = row_el.child(option_btn(
                SharedString::from(format!("inspector-trans-{row_idx}-{i}")),
                label,
                active,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Slides(slides::Message::SlideTransitionChanged(transition)),
                        window,
                        cx,
                    );
                }),
            ));
        }
        panel = panel.child(row_el);
    }

    if !is_cut {
        panel = panel.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(90.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_SECONDARY))
                        .child("Duration (ms)"),
                )
                .child(
                    div()
                        .w(px(70.))
                        .child(Input::new(&w.inspector_transition_dur_input.clone())),
                ),
        );
    }

    panel = panel.child(section_label("GROUP LABEL"));
    panel =
        panel.child(Input::new(&w.inspector_group_label_input.clone()).aria_label("Group label"));
    // The groups the operator can pick. The field above takes any name the
    // operator types, and the picker takes the groups the app knows — so a
    // group spelled right is one click away and a group of their own is
    // still possible.
    panel = panel.child(crate::ui::groups::group_picker(w, cx));

    panel = panel.child(section_label("NOTES"));
    panel = panel.child(Input::new(&w.inspector_notes_input.clone()).aria_label("Speaker notes"));

    panel
}

/// The six background colour swatches of the Build tab.
fn background_swatch_row(slide: &Slide, cx: &mut Context<MainWindow>) -> impl IntoElement {
    let current: Option<SlideColor> = match &slide.background {
        SlideBackground::Solid(c) => Some(*c),
        _ => None,
    };
    let swatches: [(Hsla, SlideColor); 6] = [
        (gpui_kit::black(), SlideColor::black()),
        (white(), SlideColor::white()),
        (
            hsla(0.66, 0.6, 0.11, 1.0),
            SlideColor {
                r: 25,
                g: 25,
                b: 100,
                a: 255,
            },
        ),
        (
            hsla(0.33, 0.5, 0.1, 1.0),
            SlideColor {
                r: 25,
                g: 75,
                b: 25,
                a: 255,
            },
        ),
        (
            hsla(0., 0.65, 0.12, 1.0),
            SlideColor {
                r: 100,
                g: 25,
                b: 25,
                a: 255,
            },
        ),
        (
            hsla(0.62, 0.28, 0.16, 1.0),
            SlideColor {
                r: 50,
                g: 50,
                b: 90,
                a: 255,
            },
        ),
    ];
    let mut row = div().flex().flex_row().gap(px(6.));
    for (i, (color, slide_color)) in swatches.into_iter().enumerate() {
        let selected = current
            .map(|c| c.r == slide_color.r && c.g == slide_color.g && c.b == slide_color.b)
            .unwrap_or(false);
        row = row.child(color_swatch_btn(
            SharedString::from(format!("inspector-bg-{i}")),
            color,
            selected,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Slides(slides::Message::SlideBackgroundChanged(
                        SlideBackground::Solid(slide_color),
                    )),
                    window,
                    cx,
                );
            }),
        ));
    }
    row
}

// ---------------------------------------------------------------------------
// Theme tab
// ---------------------------------------------------------------------------

fn themes_tab(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> Div {
    let themes = w.theme_state.list.clone();
    let selected_theme_id = w.theme_state.selected_theme_id.clone();
    let new_name = w.theme_state.new_theme_name.clone();
    // A slide can be saved as a theme when it has a text object to take the
    // style from — not only when its legacy content is still text, which it
    // stops being as soon as the text moves into an object.
    let has_text = w
        .get_current_slide()
        .and_then(|slide| slide.editable_text(w.layer.selected_index))
        .is_some();
    let can_save = !new_name.trim().is_empty() && has_text;

    let mut panel = div().p(px(14.)).flex().flex_col().gap(px(12.));

    panel = panel.child(section_label("SAVE CURRENT SLIDE AS THEME"));
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&w.themes_name_input.clone())),
            )
            .child(pasteless_button(
                "inspector-theme-save",
                "Save",
                can_save,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Themes(themes_feature::Message::SaveSlideAsTheme),
                        window,
                        cx,
                    );
                }),
            )),
    );
    if !has_text {
        panel = panel.child(
            div()
                .text_size(px(10.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Select a text slide to save as theme."),
        );
    }

    panel = panel.child(section_label("IMPORT / EXPORT"));
    panel = panel.child(
        div()
            .flex()
            .flex_row()
            .gap(px(6.))
            .child(
                Button::new("inspector-theme-export")
                    .label("Export JSON")
                    .secondary()
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            Message::Themes(themes_feature::Message::Export),
                            window,
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("inspector-theme-import")
                    .label("Import JSON")
                    .secondary()
                    .small()
                    .on_click(cx.listener(|w, _, window, cx| {
                        w.handle_message(
                            Message::Themes(themes_feature::Message::Import),
                            window,
                            cx,
                        );
                    })),
            ),
    );

    panel = panel.child(section_label("SAVED THEMES"));

    if themes.is_empty() {
        panel = panel.child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No themes saved yet."),
        );
    } else {
        let mut list = div().flex().flex_col().gap(px(2.));
        for theme_item in &themes {
            let is_selected = selected_theme_id.as_deref() == Some(theme_item.id.as_str());
            let bg = match &theme_item.background {
                SlideBackground::Solid(c) => slide_color_hsla(c),
                SlideBackground::Image(_) => hsla(0.68, 0.3, 0.35, 1.0),
                SlideBackground::Video(_) => hsla(0.72, 0.4, 0.22, 1.0),
            };
            let theme_id = theme_item.id.clone();
            let theme_id_del = theme_item.id.clone();
            let theme_id_apply = theme_item.id.clone();
            let name = theme_item.name.clone();

            list = list.child(
                div()
                    .id(SharedString::from(format!(
                        "inspector-theme-{}",
                        theme_item.id
                    )))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.))
                    .py(px(6.))
                    .px(px(4.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if is_selected {
                        theme::c(theme::ACCENT_ORANGE)
                    } else {
                        gpui_kit::transparent_black()
                    })
                    .bg(if is_selected {
                        theme::c(theme::ACCENT_ORANGE).opacity(0.16)
                    } else {
                        gpui_kit::transparent_black()
                    })
                    .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
                    .on_click(cx.listener(move |w, _, window, cx| {
                        w.handle_message(
                            Message::Themes(themes_feature::Message::Select(theme_id.clone())),
                            window,
                            cx,
                        );
                    }))
                    .child(
                        div()
                            .w(px(22.))
                            .h(px(22.))
                            .flex_shrink_0()
                            .rounded(px(3.))
                            .border_1()
                            .border_color(theme::c(theme::BORDER_STRONG))
                            .bg(bg),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(if is_selected {
                                theme::c(theme::TEXT_PRIMARY)
                            } else {
                                theme::c(theme::TEXT_SECONDARY)
                            })
                            .child(name),
                    )
                    .child(theme_row_button(
                        SharedString::from(format!("inspector-theme-apply-{}", theme_item.id)),
                        "Apply",
                        cx.listener(move |w, _, window, cx| {
                            cx.stop_propagation();
                            w.handle_message(
                                Message::Themes(themes_feature::Message::Apply(
                                    theme_id_apply.clone(),
                                )),
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(theme_row_button(
                        SharedString::from(format!("inspector-theme-del-{}", theme_item.id)),
                        "×",
                        cx.listener(move |w, _, window, cx| {
                            cx.stop_propagation();
                            w.handle_message(
                                Message::Themes(themes_feature::Message::Delete(
                                    theme_id_del.clone(),
                                )),
                                window,
                                cx,
                            );
                        }),
                    )),
            );
        }
        panel = panel.child(list);
    }

    panel
}

/// Small text button used inside a clickable theme row; stops propagation so
/// the row's select handler does not also fire.
fn theme_row_button(
    id: impl Into<ElementId>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .py(px(3.))
        .px(px(8.))
        .rounded(px(3.))
        .text_size(px(10.))
        .text_color(theme::c(theme::TEXT_PRIMARY))
        .bg(theme::c(theme::BG_HOVER))
        .hover(|s| s.bg(theme::c(theme::BG_ACTIVE)))
        .on_click(on_click)
        .child(label)
}

/// A state-styled button that stays clickable in both states (primary when
/// `active`, secondary otherwise). Used for Visible / Locked.
fn state_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let button = Button::new(id).label(label).small();
    if active {
        button.primary().on_click(on_click)
    } else {
        button.secondary().on_click(on_click)
    }
}

/// A secondary-style button that renders without a click handler when
/// `enabled` is false.
fn pasteless_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let button = Button::new(id).label(label).small();
    if enabled {
        button.secondary().on_click(on_click)
    } else {
        button.ghost()
    }
}

// ---------------------------------------------------------------------------
// Shape (layers) tab
// ---------------------------------------------------------------------------

fn layers_tab(w: &mut MainWindow, slide: &Slide, cx: &mut Context<MainWindow>) -> Div {
    let mut col = div().flex().flex_col();

    // Add row.
    let add_row = div()
        .w_full()
        .flex()
        .flex_row()
        .gap(px(4.))
        .py(px(6.))
        .px(px(10.))
        .bg(theme::c(theme::BG_DARK))
        .child(
            Button::new("layers-add-text")
                .label("+ Text")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(Message::Layers(layers::Message::AddTextLayer), window, cx);
                })),
        )
        .child(
            Button::new("layers-add-rect")
                .label("+ Rect")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::AddShapeLayer(ShapeType::Rectangle)),
                        window,
                        cx,
                    );
                })),
        )
        .child(
            Button::new("layers-add-ellipse")
                .label("+ Ellipse")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::AddShapeLayer(ShapeType::Ellipse)),
                        window,
                        cx,
                    );
                })),
        );
    col = col.child(add_row);

    let layers_list = &slide.layers;
    if layers_list.is_empty() {
        col = col.child(
            div()
                .p(px(16.))
                .px(px(14.))
                .text_size(px(12.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("No layers — add one above, or\nclick \"+ Text\" to get started."),
        );
    } else {
        let mut sorted: Vec<(usize, &Object)> = layers_list.iter().enumerate().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1.z_order));

        let mut rows = div().flex().flex_col().py(px(2.)).px(px(4.));
        for (orig_idx, layer) in sorted {
            rows = rows.child(layer_row(
                w.layer.selected_index == Some(orig_idx),
                layer,
                orig_idx,
                cx,
            ));
        }
        col = col.child(rows);
    }

    if let Some(idx) = w.layer.selected_index
        && let Some(layer) = layers_list.get(idx)
    {
        col = col.child(layer_props_panel(w, layer, cx));
    }

    col
}

/// One row of the layer list: visibility, lock, name, reorder, delete.
fn layer_row(
    is_selected: bool,
    layer: &Object,
    orig_idx: usize,
    cx: &mut Context<MainWindow>,
) -> impl IntoElement {
    let name = layer.display_name();

    div()
        .id(SharedString::from(format!("layer-row-{orig_idx}")))
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(2.))
        .py(px(2.))
        .px(px(4.))
        .rounded(px(4.))
        .border_1()
        .border_color(if is_selected {
            theme::c(theme::ACCENT_ORANGE)
        } else {
            gpui_kit::transparent_black()
        })
        .bg(if is_selected {
            theme::c(theme::ACCENT_ORANGE).opacity(0.16)
        } else {
            gpui_kit::transparent_black()
        })
        .child(layer_row_icon_btn(
            SharedString::from(format!("layer-vis-{orig_idx}")),
            if layer.visible {
                IconName::Eye
            } else {
                IconName::EyeOff
            },
            layer.visible,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::ToggleLayerVisibility(orig_idx)),
                    window,
                    cx,
                );
            }),
        ))
        .child(layer_row_icon_btn(
            SharedString::from(format!("layer-lock-{orig_idx}")),
            IconName::Ban,
            layer.locked,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::ToggleLayerLock(orig_idx)),
                    window,
                    cx,
                );
            }),
        ))
        .child(
            div()
                .id(SharedString::from(format!("layer-name-{orig_idx}")))
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .text_color(if is_selected {
                    theme::c(theme::TEXT_PRIMARY)
                } else if layer.visible {
                    theme::c(theme::TEXT_SECONDARY)
                } else {
                    theme::c(theme::TEXT_MUTED)
                })
                .hover(|s| s.text_color(theme::c(theme::TEXT_PRIMARY)))
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectLayer(Some(orig_idx))),
                        window,
                        cx,
                    );
                }))
                // The name is observed, so a test can click it.
                .test_support()
                .child(name),
        )
        .child(layer_row_icon_btn(
            SharedString::from(format!("layer-up-{orig_idx}")),
            IconName::ArrowUp,
            false,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::MoveLayerUp(orig_idx)),
                    window,
                    cx,
                );
            }),
        ))
        .child(layer_row_icon_btn(
            SharedString::from(format!("layer-down-{orig_idx}")),
            IconName::ArrowDown,
            false,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::MoveLayerDown(orig_idx)),
                    window,
                    cx,
                );
            }),
        ))
        .child(layer_row_icon_btn(
            SharedString::from(format!("layer-del-{orig_idx}")),
            IconName::Close,
            false,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::DeleteLayer(orig_idx)),
                    window,
                    cx,
                );
            }),
        ))
}

/// Tiny icon button of a layer row; `active` tints it orange.
fn layer_row_icon_btn(
    id: impl Into<ElementId>,
    icon: IconName,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let mut icon_el = Icon::new(icon).xsmall();
    icon_el = if active {
        icon_el.text_color(theme::c(theme::ACCENT_ORANGE))
    } else {
        icon_el.text_color(theme::c(theme::TEXT_MUTED))
    };
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .py(px(2.))
        .px(px(4.))
        .rounded(px(3.))
        .hover(|s| s.bg(theme::c(theme::BG_HOVER)))
        .on_click(on_click)
        // The button is observed, so a test can press it: the layer's
        // controls are the editor's own, and nothing else observes them.
        .test_support()
        .child(icon_el)
}

/// The per-layer property editors below the layer list.
fn layer_props_panel(w: &mut MainWindow, layer: &Object, cx: &mut Context<MainWindow>) -> Div {
    let mut col = div().p(px(14.)).py(px(10.)).flex().flex_col().gap(px(8.));

    col = col.child(
        div()
            .py(px(6.))
            .text_size(px(11.))
            .text_color(theme::c(theme::TEXT_MUTED))
            .child(format!("▸ {} Properties", layer.display_name())),
    );

    // Visibility / lock.
    let visible = layer.visible;
    let locked = layer.locked;
    col = col.child(
        div()
            .flex()
            .flex_row()
            .gap(px(8.))
            .child(state_button(
                "layer-props-visible",
                if visible { "Visible" } else { "Hidden" },
                visible,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::ToggleSelectedLayerVisibility),
                        window,
                        cx,
                    );
                }),
            ))
            .child(state_button(
                "layer-props-locked",
                if locked { "Locked" } else { "Unlocked" },
                locked,
                cx.listener(|w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::ToggleSelectedLayerLock),
                        window,
                        cx,
                    );
                }),
            )),
    );

    // Opacity.
    col = col.child(section_label("Opacity"));
    col = col.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .child(Slider::new(&w.layer_opacity_slider.clone())),
            )
            .child(
                div()
                    .w(px(32.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(format!("{:.0}%", layer.opacity * 100.0)),
            ),
    );

    // Position / size.
    col = col.child(section_label("Position  (X / Y)"));
    col = col.child(
        div()
            .flex()
            .flex_row()
            .gap(px(6.))
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&w.layer_pos_x_input.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&w.layer_pos_y_input.clone())),
            ),
    );
    col = col.child(section_label("Size  (W / H)"));
    col = col.child(
        div()
            .flex()
            .flex_row()
            .gap(px(6.))
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&w.layer_width_input.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&w.layer_height_input.clone())),
            ),
    );

    match &layer.content {
        ObjectContent::Text { style, .. } => {
            col = col.child(text_layer_props(w, style, cx));
        }
        ObjectContent::Shape {
            fill,
            stroke_color,
            shape_type,
            ..
        } => {
            // The kind of the shape: a shape that cannot become another
            // shape has to be drawn again from scratch.
            col = col.child(section_label("Shape"));
            col = col.child(shape_type_row(*shape_type, cx));
            col = col.child(section_label("Fill Colour  (R / G / B / A)"));
            col = col.child(color_channel_slider(
                "R",
                fill.r,
                &w.layer_fill_r_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "G",
                fill.g,
                &w.layer_fill_g_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "B",
                fill.b,
                &w.layer_fill_b_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "A",
                fill.a,
                &w.layer_fill_a_slider.clone(),
            ));
            col = col.child(section_label("Stroke Colour  (R / G / B / A)"));
            col = col.child(color_channel_slider(
                "R",
                stroke_color.r,
                &w.layer_shape_stroke_r_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "G",
                stroke_color.g,
                &w.layer_shape_stroke_g_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "B",
                stroke_color.b,
                &w.layer_shape_stroke_b_slider.clone(),
            ));
            col = col.child(color_channel_slider(
                "A",
                stroke_color.a,
                &w.layer_shape_stroke_a_slider.clone(),
            ));
            col = col.child(section_label("Stroke Width"));
            col = col.child(
                div()
                    .w(px(80.))
                    .child(Input::new(&w.layer_stroke_width_input.clone())),
            );
            // The renderer draws the rounded corner, so the operator has to
            // be able to set it.
            col = col.child(section_label("Corner Radius"));
            col = col.child(
                div()
                    .w(px(80.))
                    .child(Input::new(&w.layer_corner_radius_input.clone())),
            );
        }
        ObjectContent::Image { path, .. } => {
            col = col.child(
                div()
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(if path.is_empty() {
                        String::from("No image set")
                    } else {
                        path.clone()
                    }),
            );
        }
        ObjectContent::Video { path, .. } => {
            col = col.child(
                div()
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child(if path.is_empty() {
                        String::from("No video set")
                    } else {
                        path.clone()
                    }),
            );
        }
    }

    col
}

/// The kinds of shape a shape can be changed into.
///
/// A shape that cannot become another shape has to be deleted and drawn
/// again, and the new shape does not keep the old one's colour.
fn shape_type_row(current: ShapeType, cx: &mut Context<MainWindow>) -> Div {
    let mut row = div().flex().flex_row().gap(px(4.));
    for (shape, label) in [
        (ShapeType::Rectangle, "Rect"),
        (ShapeType::Ellipse, "Oval"),
        (ShapeType::Triangle, "Tri"),
        (ShapeType::Line, "Line"),
    ] {
        let selected = current == shape;
        row = row.child(
            Button::new(SharedString::from(format!("layer-shape-{label}")))
                .label(label)
                .ghost()
                .small()
                .toggled(selected)
                .on_click(cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectedLayerShapeTypeChanged(shape)),
                        window,
                        cx,
                    );
                })),
        );
    }
    row
}

/// Typography property editors of a selected text layer (Layers tab).
fn text_layer_props(w: &mut MainWindow, style: &TextStyle, cx: &mut Context<MainWindow>) -> Div {
    let mut col = div().flex().flex_col().gap(px(8.));

    // Text + font size.
    col = col
        .child(section_label("Text"))
        .child(Input::new(&w.layer_text_input.clone()))
        .child(section_label("Font Size"))
        .child(
            div()
                .w(px(80.))
                .child(Input::new(&w.layer_font_size_input.clone())),
        );

    // Colour channels + hex.
    col = col
        .child(section_label("Colour  (R / G / B)"))
        .child(color_channel_slider(
            "R",
            style.color.r,
            &w.layer_text_r_slider.clone(),
        ))
        .child(color_channel_slider(
            "G",
            style.color.g,
            &w.layer_text_g_slider.clone(),
        ))
        .child(color_channel_slider(
            "B",
            style.color.b,
            &w.layer_text_b_slider.clone(),
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(28.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("Hex"),
                )
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&w.layer_text_hex_input.clone())),
                ),
        );

    // Style toggles.
    let bold = style.bold;
    let italic = style.italic;
    let shadow = style.shadow;
    let outline = style.outline;
    col = col.child(section_label("Style")).child(
        div()
            .flex()
            .flex_row()
            .gap(px(4.))
            .child(compact_toggle_btn(
                "layer-bold",
                "Bold",
                bold,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectedLayerTextBoldToggled),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "layer-italic",
                "Italic",
                italic,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectedLayerTextItalicToggled),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "layer-shadow",
                "Shadow",
                shadow,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectedLayerTextShadowToggled),
                        window,
                        cx,
                    );
                }),
            ))
            .child(compact_toggle_btn(
                "layer-outline",
                "Outline",
                outline,
                cx.listener(move |w, _, window, cx| {
                    w.handle_message(
                        Message::Layers(layers::Message::SelectedLayerTextOutlineToggled),
                        window,
                        cx,
                    );
                }),
            )),
    );

    // Font family + presets. The presets are the app's own list of families,
    // the same list the Text menu offers.
    let mut family_presets = div().flex().flex_row().gap(px(4.));
    for family in crate::ui::typography::FONT_PRESETS {
        family_presets = family_presets.child(family_preset_btn(family, cx));
    }
    col = col
        .child(section_label("Font Family"))
        .child(Input::new(&w.layer_font_family_input.clone()))
        .child(family_presets);

    // Line height / letter spacing.
    col = col
        .child(section_label("Line Height / Letter Spacing"))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(20.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("LH"),
                )
                .child(
                    div()
                        .w(px(64.))
                        .child(Input::new(&w.layer_line_height_input.clone())),
                )
                .child(
                    div()
                        .w(px(20.))
                        .text_size(px(11.))
                        .text_color(theme::c(theme::TEXT_MUTED))
                        .child("LS"),
                )
                .child(
                    div()
                        .w(px(64.))
                        .child(Input::new(&w.layer_letter_spacing_input.clone())),
                ),
        );

    // Text transform.
    let transform = style.text_transform;
    let mut transform_row = div().flex().flex_row().gap(px(4.));
    for (i, (label, value)) in [
        ("None", TextTransform::None),
        ("UPPER", TextTransform::Uppercase),
        ("SMALL", TextTransform::SmallCaps),
        ("Title", TextTransform::TitleCase),
        ("Start", TextTransform::StartCase),
        ("lower", TextTransform::Lowercase),
    ]
    .into_iter()
    .enumerate()
    {
        transform_row = transform_row.child(compact_toggle_btn(
            SharedString::from(format!("layer-transform-{i}")),
            label,
            transform == value,
            cx.listener(move |w, _, window, cx| {
                w.handle_message(
                    Message::Typography(typography::Message::SelectedLayerTextTransform(value)),
                    window,
                    cx,
                );
            }),
        ));
    }
    col = col
        .child(section_label("Text Transform"))
        .child(transform_row);

    // Glow.
    let glow_enabled = style.glow_enabled;
    let mut glow_row = div().flex().flex_row().items_center().gap(px(6.));
    glow_row = glow_row
        .child(compact_toggle_btn(
            "layer-glow-toggle",
            if glow_enabled { "On" } else { "Off" },
            glow_enabled,
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Typography(typography::Message::SelectedLayerGlowToggled),
                    window,
                    cx,
                );
            }),
        ))
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme::c(theme::TEXT_MUTED))
                .child("Radius"),
        )
        .child(
            div()
                .w(px(52.))
                .child(Input::new(&w.layer_glow_radius_input.clone())),
        );
    col = col.child(section_label("Glow")).child(glow_row);
    if glow_enabled {
        col = col
            .child(section_label("Glow Colour  (R / G / B)"))
            .child(color_channel_slider(
                "R",
                style.glow_color.r,
                &w.layer_glow_r_slider.clone(),
            ))
            .child(color_channel_slider(
                "G",
                style.glow_color.g,
                &w.layer_glow_g_slider.clone(),
            ))
            .child(color_channel_slider(
                "B",
                style.glow_color.b,
                &w.layer_glow_b_slider.clone(),
            ));
    }

    // Text stroke.
    col = col.child(section_label("Text Stroke")).child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .w(px(14.))
                    .text_size(px(11.))
                    .text_color(theme::c(theme::TEXT_MUTED))
                    .child("W"),
            )
            .child(
                div()
                    .w(px(64.))
                    .child(Input::new(&w.layer_text_stroke_width_input.clone())),
            ),
    );
    if style.text_stroke_width > 0.0 {
        col = col
            .child(section_label("Stroke Colour  (R / G / B)"))
            .child(color_channel_slider(
                "R",
                style.text_stroke_color.r,
                &w.layer_stroke_r_slider.clone(),
            ))
            .child(color_channel_slider(
                "G",
                style.text_stroke_color.g,
                &w.layer_stroke_g_slider.clone(),
            ))
            .child(color_channel_slider(
                "B",
                style.text_stroke_color.b,
                &w.layer_stroke_b_slider.clone(),
            ));
    }

    col
}

/// One of the four font-family preset buttons.
fn family_preset_btn(family: &'static str, cx: &mut Context<MainWindow>) -> Button {
    Button::new(SharedString::from(format!("layer-family-{family}")))
        .label(family)
        .ghost()
        .xsmall()
        .on_click(cx.listener(move |w, _, window, cx| {
            w.handle_message(
                Message::Typography(typography::Message::SelectedLayerFontFamilyChanged(
                    String::from(family),
                )),
                window,
                cx,
            );
        }))
}

/// Convert a domain [`SlideColor`] to a GPUI [`Hsla`] (alpha is preserved
/// approximately via opacity; swatches are preview-only).
fn slide_color_hsla(c: &SlideColor) -> Hsla {
    let packed = (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b);
    rgb(packed).into()
}
