//! Typography property logic for the selected layer (font family, line
//! height, letter spacing, text transform, glow, text stroke, hex colour).
//!
//! Ported from the iced-era `ui/typography/mod.rs`. The controls live inside
//! the inspector's Layers tab (see `ui/inspector.rs`); this module owns the
//! message vocabulary and the update logic.

use crate::domain::{Color, ObjectContent, TextRewrite, TextStyle, TextTransform};
use crate::ui::layers::selected_layer_mut;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::TextStyleCommand;
use gpui_kit::Context;

/// The font families the app offers.
///
/// The Text menu and the Layers tab pick from the same list, so a family
/// the operator finds in one is a family they find in the other.
pub(crate) const FONT_PRESETS: [&str; 4] = ["Arial", "Georgia", "Impact", "Courier New"];

/// Messages owned by the Typography editor feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Typography(..)`.
#[derive(Debug, Clone)]
pub enum Message {
    SelectedLayerFontFamilyChanged(String),
    SelectedLayerLineHeightChanged(String),
    SelectedLayerLetterSpacingChanged(String),
    SelectedLayerTextTransform(TextTransform),
    SelectedLayerGlowToggled,
    SelectedLayerGlowColorR(u8),
    SelectedLayerGlowColorG(u8),
    SelectedLayerGlowColorB(u8),
    SelectedLayerGlowRadiusChanged(String),
    SelectedLayerTextStrokeWidthChanged(String),
    SelectedLayerTextStrokeColorR(u8),
    SelectedLayerTextStrokeColorG(u8),
    SelectedLayerTextStrokeColorB(u8),
    SelectedLayerTextColorHex(String),
}

/// Dispatch a typography message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::SelectedLayerFontFamilyChanged(f) => font_family_changed(w, f, cx),
        Message::SelectedLayerLineHeightChanged(s) => line_height_changed(w, s, cx),
        Message::SelectedLayerLetterSpacingChanged(s) => letter_spacing_changed(w, s, cx),
        Message::SelectedLayerTextTransform(t) => text_transform_changed(w, t, cx),
        Message::SelectedLayerGlowToggled => glow_toggled(w, cx),
        Message::SelectedLayerGlowColorR(v) => glow_color_r(w, v, cx),
        Message::SelectedLayerGlowColorG(v) => glow_color_g(w, v, cx),
        Message::SelectedLayerGlowColorB(v) => glow_color_b(w, v, cx),
        Message::SelectedLayerGlowRadiusChanged(s) => glow_radius_changed(w, s, cx),
        Message::SelectedLayerTextStrokeWidthChanged(s) => text_stroke_width_changed(w, s, cx),
        Message::SelectedLayerTextStrokeColorR(v) => text_stroke_color_r(w, v, cx),
        Message::SelectedLayerTextStrokeColorG(v) => text_stroke_color_g(w, v, cx),
        Message::SelectedLayerTextStrokeColorB(v) => text_stroke_color_b(w, v, cx),
        Message::SelectedLayerTextColorHex(hex) => text_color_hex(w, hex, cx),
    }
}

pub(crate) fn font_family_changed(
    w: &mut MainWindow,
    family: String,
    cx: &mut Context<MainWindow>,
) {
    w.layer.font_family = family.clone();
    let trimmed = family.trim().to_string();
    w.push_undo_coalesced();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        // An empty family is the default one, and it is written like any
        // other value: the field is empty, so the slide has to be too.
        // Leaving the old family on the object would leave the panel and
        // the slide disagreeing about what the text looks like.
        style.font_family = trimmed;
        save_current_slide(w, cx);
    }
    cx.notify();
}

pub(crate) fn line_height_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    let mut applied = false;
    if let Ok(v) = s.parse::<f32>()
        && v > 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.line_height_multiplier = v;
        applied = true;
    }
    if applied {
        w.layer.line_height = s;
        save_current_slide(w, cx);
    } else {
        // The value cannot be used: the field keeps what the slide really
        // holds, so a number that does nothing is not left in front of the
        // operator.
        w.layer.line_height = line_height_of(w);
    }
    cx.notify();
}

/// The line height the slide really holds, as the field shows it.
fn line_height_of(w: &mut MainWindow) -> String {
    selected_layer_mut(w)
        .and_then(|layer| match &layer.content {
            ObjectContent::Text { style, .. } => {
                Some(format!("{:.2}", style.line_height_multiplier))
            }
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn letter_spacing_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    let mut applied = false;
    if let Ok(v) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.letter_spacing = v;
        applied = true;
    }
    if applied {
        w.layer.letter_spacing = s;
        save_current_slide(w, cx);
    } else {
        // A value that is not a number is not used, and the field goes back
        // to what the slide holds.
        w.layer.letter_spacing = letter_spacing_of(w);
    }
    cx.notify();
}

/// The letter spacing the slide really holds, as the field shows it.
fn letter_spacing_of(w: &mut MainWindow) -> String {
    selected_layer_mut(w)
        .and_then(|layer| match &layer.content {
            ObjectContent::Text { style, .. } => Some(format!("{}", style.letter_spacing)),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn text_transform_changed(
    w: &mut MainWindow,
    t: TextTransform,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.text_transform = t;
        save_current_slide(w, cx);
    }
    cx.notify();
}

pub(crate) fn glow_toggled(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.glow_enabled = !style.glow_enabled;
        save_current_slide(w, cx);
    }
    cx.notify();
}

macro_rules! glow_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8, cx: &mut Context<MainWindow>) {
            w.push_undo_coalesced();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.glow_color.$channel = v;
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

glow_channel!(glow_color_r, r);
glow_channel!(glow_color_g, g);
glow_channel!(glow_color_b, b);

pub(crate) fn glow_radius_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    let mut applied = false;
    if let Ok(v) = s.parse::<f32>()
        && v >= 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.glow_radius = v;
        applied = true;
    }
    if applied {
        w.layer.glow_radius = s;
        save_current_slide(w, cx);
    } else {
        // A radius that cannot be used is not shown as if it were: the
        // field goes back to what the slide holds.
        w.layer.glow_radius = glow_radius_of(w);
    }
    cx.notify();
}

/// The glow radius the slide really holds, as the field shows it.
fn glow_radius_of(w: &mut MainWindow) -> String {
    selected_layer_mut(w)
        .and_then(|layer| match &layer.content {
            ObjectContent::Text { style, .. } => Some(format!("{}", style.glow_radius)),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn text_stroke_width_changed(
    w: &mut MainWindow,
    s: String,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    let mut applied = false;
    if let Ok(v) = s.parse::<f32>()
        && v >= 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.text_stroke_width = v;
        applied = true;
    }
    if applied {
        w.layer.text_stroke_width = s;
        save_current_slide(w, cx);
    } else {
        // A width that cannot be used is not shown as if it were: the field
        // goes back to what the slide holds.
        w.layer.text_stroke_width = text_stroke_width_of(w);
    }
    cx.notify();
}

/// The stroke width the slide really holds, as the field shows it.
fn text_stroke_width_of(w: &mut MainWindow) -> String {
    selected_layer_mut(w)
        .and_then(|layer| match &layer.content {
            ObjectContent::Text { style, .. } => Some(format!("{}", style.text_stroke_width)),
            _ => None,
        })
        .unwrap_or_default()
}

macro_rules! stroke_color_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8, cx: &mut Context<MainWindow>) {
            w.push_undo_coalesced();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.text_stroke_color.$channel = v;
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

stroke_color_channel!(text_stroke_color_r, r);
stroke_color_channel!(text_stroke_color_g, g);
stroke_color_channel!(text_stroke_color_b, b);

pub(crate) fn text_color_hex(w: &mut MainWindow, hex: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    let stripped = hex.trim().trim_start_matches('#');
    if stripped.len() != 6 {
        return;
    }
    let (Ok(r), Ok(g), Ok(b)) = (
        u8::from_str_radix(&stripped[0..2], 16),
        u8::from_str_radix(&stripped[2..4], 16),
        u8::from_str_radix(&stripped[4..6], 16),
    ) else {
        return;
    };
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.color = Color {
            r,
            g,
            b,
            a: style.color.a,
        };
        save_current_slide(w, cx);
    }
    cx.notify();
}

/// Persist the slide being edited through the presentation service.
fn save_current_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        w.persist_slide(clone, cx);
    }
}

// ---------------------------------------------------------------------------
// The Text menu
// ---------------------------------------------------------------------------

/// How much one "Bigger" or "Smaller" changes the size.
///
/// The step is a share of the size, not a fixed number of points: a step
/// that reads on a 20 pixel caption would be invisible on a 96 pixel title,
/// and a step that reads on a title would swallow a caption whole.
const SIZE_STEP: f32 = 0.05;

/// How much one "Loosen" or "Tighten" changes the letter spacing.
///
/// The spacing is measured in ems, so the step is a share of the letter
/// width whatever the size of the text is.
const SPACING_STEP: f32 = 0.05;

/// The letter spacing the text may not go past.
const SPACING_LIMIT: f32 = 1.0;

/// Apply a command from the Text menu.
///
/// The command acts on the text the operator is editing: the selected
/// object's text when a text object is selected, the slide's own text
/// otherwise — the same rule every other text control uses.
pub(crate) fn apply_text_style_command(
    w: &mut MainWindow,
    command: TextStyleCommand,
    cx: &mut Context<MainWindow>,
) -> bool {
    match command {
        TextStyleCommand::CopyStyle => {
            if let Some(slide) = w.get_current_slide()
                && let Some((_, style)) = slide.editable_text(w.layer.selected_index)
            {
                w.editor.copied_style = Some(style.clone());
            }
            cx.notify();
            return true;
        }
        TextStyleCommand::PasteStyle => {
            let Some(copied) = w.editor.copied_style.clone() else {
                return false;
            };
            w.push_undo();
            let font_size = edit_style(w, |style| {
                *style = copied.clone();
                copied.font_size
            });
            if let Some(size) = font_size {
                let text = format!("{}", size as u32);
                w.editor.editing_slide_font_size = text.clone();
                w.layer.font_size = text;
            }
            // The paste replaces the whole style, so the Layers tab has to
            // show the pasted values: the family, the line height, the
            // letter spacing, the casing and the glow are all part of it.
            crate::ui::layers::load_layer_for_editing(w);
            save_current_slide(w, cx);
            cx.notify();
            return true;
        }
        TextStyleCommand::ApplyToAllSlides => {
            return apply_style_to_all_slides(w, cx);
        }
        TextStyleCommand::Bold => {
            w.push_undo();
            edit_style(w, |style| {
                style.bold = !style.bold;
            });
        }
        TextStyleCommand::Italic => {
            w.push_undo();
            edit_style(w, |style| {
                style.italic = !style.italic;
            });
        }
        TextStyleCommand::Underline => {
            w.push_undo();
            edit_style(w, |style| {
                style.underline = !style.underline;
            });
        }
        TextStyleCommand::Strikethrough => {
            w.push_undo();
            edit_style(w, |style| {
                style.strikethrough = !style.strikethrough;
            });
        }
        TextStyleCommand::Bigger => {
            w.push_undo();
            edit_style(w, |style| {
                style.font_size = TextStyle::clamp_font_size(style.font_size * (1.0 + SIZE_STEP));
            });
        }
        TextStyleCommand::Smaller => {
            w.push_undo();
            edit_style(w, |style| {
                style.font_size = TextStyle::clamp_font_size(style.font_size * (1.0 - SIZE_STEP));
            });
        }
        TextStyleCommand::Loosen => {
            w.push_undo();
            edit_style(w, |style| {
                style.letter_spacing = (style.letter_spacing + SPACING_STEP).min(SPACING_LIMIT);
            });
        }
        TextStyleCommand::Tighten => {
            w.push_undo();
            edit_style(w, |style| {
                style.letter_spacing = (style.letter_spacing - SPACING_STEP).max(-SPACING_LIMIT);
            });
        }
        TextStyleCommand::SpacingDefault => {
            w.push_undo();
            edit_style(w, |style| {
                style.letter_spacing = 0.0;
            });
        }
        TextStyleCommand::Capitalization(value) => {
            w.push_undo();
            edit_style(w, |style| {
                style.text_transform = value;
            });
        }
        TextStyleCommand::Baseline(value) => {
            w.push_undo();
            edit_style(w, |style| {
                style.baseline = value;
            });
        }
        TextStyleCommand::Ligatures(value) => {
            w.push_undo();
            edit_style(w, |style| {
                style.ligatures = value;
            });
        }
    }
    // The font size the operator reads on the Text tab is a mirror of the
    // model's, so a change to the size has to reach the mirror too.
    if let Some(size) = edit_style(w, |style| style.font_size) {
        let text = format!("{}", size as u32);
        w.editor.editing_slide_font_size = text.clone();
        w.layer.font_size = text;
    }
    crate::ui::layers::load_layer_for_editing(w);
    save_current_slide(w, cx);
    cx.notify();
    true
}

/// Rewrite the letters of the text being edited.
///
/// The Transformations menu changes the words themselves, so the change is
/// in the model and not in the style. The Text tab shows a mirror of the
/// text, so the mirror has to be brought into step: a mirror left behind is
/// a text that is written back over the slide.
pub(crate) fn apply_text_rewrite(
    w: &mut MainWindow,
    rewrite: TextRewrite,
    cx: &mut Context<MainWindow>,
) -> bool {
    let selected = w.layer.selected_index;
    // The undo snapshot is taken before the words are changed, so they can
    // be put back.
    w.push_undo();
    let text = {
        let Some(slide) = w.get_current_slide_mut() else {
            return false;
        };
        if !slide.rewrite_editable_text(selected, rewrite) {
            return false;
        }
        slide.editable_text(selected).map(|(text, _)| text.clone())
    };
    let Some(text) = text else {
        return false;
    };
    w.editor.editing_slide_text = text;
    save_current_slide(w, cx);
    cx.notify();
    true
}

/// Apply `f` to the text style the operator is editing.
///
/// The selected object's style when a text object is selected, the slide's
/// own text style otherwise — the one rule every text control uses, so the
/// menu and the panel cannot disagree about which text is being edited.
fn edit_style<T>(w: &mut MainWindow, f: impl FnOnce(&mut TextStyle) -> T) -> Option<T> {
    let selected = w.layer.selected_index;
    let slide = w.get_current_slide_mut()?;
    slide.editable_text_mut(selected).map(|(_, style)| f(style))
}

/// Put the edited text's style on every slide of the presentation.
///
/// The style is the one the operator is looking at, so it is taken from the
/// slide being edited — a style copied from somewhere else would be a style
/// the operator did not choose.
fn apply_style_to_all_slides(w: &mut MainWindow, cx: &mut Context<MainWindow>) -> bool {
    let Some(style) = w
        .get_current_slide()
        .and_then(|slide| slide.editable_text(w.layer.selected_index))
        .map(|(_, style)| style.clone())
    else {
        return false;
    };
    let Some(presentation) = w.editor.editing.as_ref() else {
        return false;
    };
    let mut slides: Vec<crate::domain::Slide> = presentation.slides.clone();
    let presentation_id = presentation.id.clone();

    w.push_undo();
    for slide in slides.iter_mut() {
        // Every text object on the slide takes the style: a slide whose
        // title is styled and whose caption is not is a slide the operator
        // did not ask for.
        for object in slide.layers.iter_mut() {
            if let ObjectContent::Text {
                style: text_style, ..
            } = &mut object.content
            {
                *text_style = style.clone();
            }
        }
        if let crate::domain::SlideContent::Text {
            style: text_style, ..
        } = &mut slide.content
        {
            *text_style = style.clone();
        }
    }
    for slide in &slides {
        let clone = slide.clone();
        if let Err(e) = w
            .services
            .presentations
            .update_slide(&presentation_id, &clone)
        {
            w.set_error(format!("Failed to save slide: {e}"));
            return false;
        }
    }
    // The edited document has to hold the same slides the database does, or
    // the next edit would write the old styles back over them.
    if let Some(edited) = w.editor.editing.as_mut() {
        for (index, slide) in edited.slides.iter_mut().enumerate() {
            *slide = slides[index].clone();
        }
    }
    cx.notify();
    true
}
