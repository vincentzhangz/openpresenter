//! Slide-management update logic (Add/Duplicate/Delete/Move/Select, group
//! labels, text + style property edits, conversions, file pickers, style
//! copy/paste, inline text editing).
//!
//! Ported from the iced-era `ui/slides/mod.rs`. The property messages are
//! emitted by the inspector's Text / Build tab controls (see
//! `ui/inspector.rs`).

use gpui_kit::{Context, Window};

use crate::domain::{ImageFit, SlideContent, TextAlignment, TextStyle, Transition};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::{InspectorTab, ViewMode};

/// Mirror programmatic slide-text state into the inspector inputs.
///
/// The inspector's own [`sync_inputs`](crate::ui::inspector::sync_inputs)
/// covers the text fields; this hook exists for the root render's per-feature
/// sync list and keeps the slide's mirror in step after undo / selection
/// changes.
pub(crate) fn sync_inputs(
    _w: &mut MainWindow,
    _window: &mut Window,
    _cx: &mut Context<MainWindow>,
) {
}

/// Messages owned by the Slides editor feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Slides(..)`.
#[derive(Debug, Clone)]
pub enum Message {
    AddSlide,
    AddSlideAfter(usize),
    DuplicateSlide(usize),
    SelectSlide(usize),
    DeleteSlide(String),
    MoveSlideUp(usize),
    MoveSlideDown(usize),
    SetSlideGroupLabel(usize, String),
    GroupLabelChanged(String),
    SlideTextChanged(String),
    SlideFontSizeChanged(String),
    SlideAlignmentChanged(crate::domain::TextAlignment),
    SlideColorChanged(crate::domain::Color),
    SlideShadowToggled(bool),
    SlideOutlineToggled(bool),
    SlideBoldToggled(bool),
    SlideItalicToggled(bool),
    SlidePositionPreset(f32, f32),
    SlideBackgroundChanged(crate::domain::Background),
    SlideTransitionChanged(crate::domain::Transition),
    TransitionDurationChanged(String),
    SaveSlide,
    ConvertSlideToText,
    ConvertSlideToImage,
    ConvertSlideToVideo,
    PickImageFile,
    SlideImageFitChanged(crate::domain::ImageFit),
    PickVideoFile,
    SlideNotesChanged(String),
    CopyTextStyle,
    PasteTextStyle,
    SlideFontFamilyCycle,
    SlideFontSizeStep(f32),
    SlideTextTransformCycle,
    SlideTextColorCycle,
    SlideGlowToggled(bool),
    StartInlineTextEditing(Option<usize>),
    EndInlineTextEditing,
}

/// Dispatch a slide-editor message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::AddSlide => add_slide(w, cx),
        Message::AddSlideAfter(i) => add_slide_after(w, i, cx),
        Message::DuplicateSlide(i) => duplicate_slide(w, i, cx),
        Message::SelectSlide(i) => select_slide(w, i, cx),
        Message::DeleteSlide(id) => delete_slide(w, id, cx),
        Message::MoveSlideUp(i) => move_slide_up(w, i, cx),
        Message::MoveSlideDown(i) => move_slide_down(w, i, cx),
        Message::SetSlideGroupLabel(i, label) => set_slide_group_label(w, i, label, cx),
        Message::GroupLabelChanged(label) => group_label_changed(w, label, cx),
        Message::SlideTextChanged(t) => slide_text_changed(w, t, cx),
        Message::SlideFontSizeChanged(s) => slide_font_size_changed(w, s, cx),
        Message::SlideAlignmentChanged(a) => slide_alignment_changed(w, a, cx),
        Message::SlideColorChanged(c) => slide_color_changed(w, c, cx),
        Message::SlideShadowToggled(v) => slide_shadow_toggled(w, v, cx),
        Message::SlideOutlineToggled(v) => slide_outline_toggled(w, v, cx),
        Message::SlideBoldToggled(v) => slide_bold_toggled(w, v, cx),
        Message::SlideItalicToggled(v) => slide_italic_toggled(w, v, cx),
        Message::SlidePositionPreset(x, y) => slide_position_preset(w, x, y, cx),
        Message::SlideBackgroundChanged(bg) => slide_background_changed(w, bg, cx),
        Message::SlideTransitionChanged(t) => slide_transition_changed(w, t, cx),
        Message::TransitionDurationChanged(s) => transition_duration_changed(w, s, cx),
        Message::SaveSlide => save_slide(w, cx),
        Message::ConvertSlideToText => convert_to_text(w, cx),
        Message::ConvertSlideToImage => convert_to_image(w, cx),
        Message::ConvertSlideToVideo => convert_to_video(w, cx),
        Message::PickImageFile => pick_image_file(w, cx),
        Message::SlideImageFitChanged(fit) => slide_image_fit_changed(w, fit, cx),
        Message::PickVideoFile => pick_video_file(w, cx),
        Message::SlideNotesChanged(notes) => slide_notes_changed(w, notes, cx),
        Message::CopyTextStyle => copy_text_style(w, cx),
        Message::PasteTextStyle => paste_text_style(w, cx),
        Message::SlideFontFamilyCycle => slide_font_family_cycle(w, cx),
        Message::SlideFontSizeStep(delta) => slide_font_size_step(w, delta, cx),
        Message::SlideTextTransformCycle => slide_text_transform_cycle(w, cx),
        Message::SlideTextColorCycle => slide_text_color_cycle(w, cx),
        Message::SlideGlowToggled(v) => slide_glow_toggled(w, v, cx),
        Message::StartInlineTextEditing(idx) => start_inline_text_editing(w, idx, cx),
        Message::EndInlineTextEditing => end_inline_text_editing(w, cx),
    }
}

pub(crate) fn start_inline_text_editing(
    w: &mut MainWindow,
    idx_opt: Option<usize>,
    cx: &mut Context<MainWindow>,
) {
    // The snapshot is taken when the editing starts, so the whole inline
    // edit is one entry in the history rather than one per keystroke.
    w.push_undo();
    if let Some(idx) = idx_opt {
        w.layer.selected_index = Some(idx);
        crate::ui::layers::load_layer_for_editing(w);
    } else if w.layer.selected_index.is_none()
        && let Some(slide) = w.get_current_slide()
    {
        let layers = slide.effective_layers();
        if let Some((i, _)) = layers
            .iter()
            .enumerate()
            .find(|(_, l)| matches!(l.content, crate::domain::ObjectContent::Text { .. }))
        {
            w.layer.selected_index = Some(i);
            crate::ui::layers::load_layer_for_editing(w);
        }
    }
    w.shell.inspector_tab = InspectorTab::Text;
    w.editor.inline_editing = true;
    cx.notify();
}

pub(crate) fn end_inline_text_editing(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.editor.inline_editing = false;
    save_current_slide(w, cx);
    cx.notify();
}

/// Persist the slide being edited through the presentation service.
fn save_current_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        w.persist_slide(clone, cx);
    }
}

pub(crate) const FONT_FAMILIES: &[&str] = &[
    "Arial",
    "Inter",
    "Helvetica",
    "Georgia",
    "Impact",
    "Roboto",
    "Times New Roman",
];

pub(crate) const COLOR_PRESETS: &[crate::domain::Color] = &[
    crate::domain::Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    }, // White
    crate::domain::Color {
        r: 254,
        g: 240,
        b: 138,
        a: 255,
    }, // Yellow
    crate::domain::Color {
        r: 245,
        g: 158,
        b: 11,
        a: 255,
    }, // Gold
    crate::domain::Color {
        r: 248,
        g: 113,
        b: 113,
        a: 255,
    }, // Coral Red
    crate::domain::Color {
        r: 96,
        g: 165,
        b: 250,
        a: 255,
    }, // Sky Blue
    crate::domain::Color {
        r: 52,
        g: 211,
        b: 153,
        a: 255,
    }, // Mint
    crate::domain::Color {
        r: 203,
        g: 213,
        b: 225,
        a: 255,
    }, // Silver
    crate::domain::Color {
        r: 24,
        g: 24,
        b: 27,
        a: 255,
    }, // Charcoal
];

/// Apply `f` to the text style the operator is editing: the selected layer's
/// style when a layer is selected, the slide's top-level text style
/// otherwise, else the first text object's style.
fn edit_text_style<T>(w: &mut MainWindow, f: impl FnOnce(&mut TextStyle) -> T) -> Option<T> {
    let sel_layer = w.layer.selected_index;
    let slide = w.get_current_slide_mut()?;
    slide
        .editable_text_mut(sel_layer)
        .map(|(_, style)| f(style))
}

pub(crate) fn copy_text_style(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(slide) = w.get_current_slide()
        && let Some((_, style)) = slide.editable_text(w.layer.selected_index)
    {
        w.editor.copied_style = Some(style.clone());
    }
    cx.notify();
}

pub(crate) fn paste_text_style(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(copied) = w.editor.copied_style.clone() else {
        return;
    };
    w.push_undo();
    let updated_font_size = edit_text_style(w, |style| {
        *style = copied.clone();
        (copied.font_size as u32).to_string()
    });
    if let Some(fs) = updated_font_size {
        w.editor.editing_slide_font_size = fs.clone();
        w.layer.font_size = fs;
    }
    // The paste replaces the whole style, so the Layers tab has to show the
    // pasted values: its family, line height, letter spacing, glow radius
    // and stroke width are all part of the style.
    crate::ui::layers::load_layer_for_editing(w);
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_font_family_cycle(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    let updated_font = edit_text_style(w, |style| {
        let cur = style.font_family.as_str();
        let next = match FONT_FAMILIES
            .iter()
            .position(|&f| f.eq_ignore_ascii_case(cur))
        {
            Some(idx) => FONT_FAMILIES[(idx + 1) % FONT_FAMILIES.len()],
            None => FONT_FAMILIES[0],
        };
        style.font_family = next.to_string();
        next.to_string()
    });
    if let Some(font) = updated_font {
        w.layer.font_family = font;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_font_size_step(w: &mut MainWindow, delta: f32, cx: &mut Context<MainWindow>) {
    w.push_undo();
    let updated_font_size = edit_text_style(w, |style| {
        style.font_size = crate::domain::TextStyle::clamp_font_size(style.font_size + delta);
        (style.font_size as u32).to_string()
    });
    if let Some(fs) = updated_font_size {
        w.editor.editing_slide_font_size = fs.clone();
        w.layer.font_size = fs;
    }
    // The paste replaces the whole style, so the Layers tab has to show the
    // pasted values: its family, line height, letter spacing, glow radius
    // and stroke width are all part of the style.
    crate::ui::layers::load_layer_for_editing(w);
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_text_transform_cycle(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.text_transform = match style.text_transform {
            crate::domain::TextTransform::None => crate::domain::TextTransform::Uppercase,
            crate::domain::TextTransform::Uppercase => crate::domain::TextTransform::SmallCaps,
            crate::domain::TextTransform::SmallCaps => crate::domain::TextTransform::TitleCase,
            crate::domain::TextTransform::TitleCase => crate::domain::TextTransform::StartCase,
            crate::domain::TextTransform::StartCase => crate::domain::TextTransform::Lowercase,
            crate::domain::TextTransform::Lowercase => crate::domain::TextTransform::None,
        };
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_text_color_cycle(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        let next_color = match COLOR_PRESETS.iter().position(|&c| c == style.color) {
            Some(idx) => COLOR_PRESETS[(idx + 1) % COLOR_PRESETS.len()],
            None => COLOR_PRESETS[1],
        };
        style.color = next_color;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_glow_toggled(w: &mut MainWindow, enabled: bool, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.glow_enabled = enabled;
    });
    save_current_slide(w, cx);
    cx.notify();
}

/// Close the slide context menu (used after an action triggered from it).
fn close_context_menu(w: &mut MainWindow) {
    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.cue_submenu = false;
    w.presenting.transition_submenu = false;
}

/// The id of the presentation the editor is working on: the editing copy if
/// one is open, else the live presentation.
fn editing_pres_id(w: &MainWindow) -> Option<String> {
    w.editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone())
}

/// Apply a service result that replaced the whole presentation: update both
/// the editing and presenting copies, select `new_index`, and reload the
/// editing fields.
fn apply_presentation_update(
    w: &mut MainWindow,
    updated: crate::domain::Presentation,
    pres_id: &str,
    new_index: Option<usize>,
) {
    w.editor.selected_slide_index = new_index.or(w.editor.selected_slide_index);
    w.editor.editing = Some(updated.clone());
    if w.presenting.presentation.as_ref().map(|p| p.id.as_str()) == Some(pres_id) {
        w.presenting.slide_index = new_index
            .unwrap_or(0)
            .min(updated.slides.len().saturating_sub(1));
        w.presenting.presentation = Some(updated);
    }
    w.load_presentations();
    if w.shell.current_mode == ViewMode::Edit {
        crate::ui::navigation::load_slide_for_editing(w);
    }
}

pub(crate) fn add_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(pres_id) = editing_pres_id(w) {
        match w.services.presentations.add_slide(&pres_id) {
            Ok(updated) => {
                let new_index = updated.slides.len().saturating_sub(1);
                apply_presentation_update(w, updated, &pres_id, Some(new_index));
            }
            Err(e) => w.set_error(format!("Failed to add slide: {e}")),
        }
    }
    cx.notify();
}

pub(crate) fn add_slide_after(
    w: &mut MainWindow,
    after_index: usize,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    if let Some(pres_id) = editing_pres_id(w) {
        match w
            .services
            .presentations
            .add_slide_after(&pres_id, after_index)
        {
            Ok((updated, new_index)) => {
                apply_presentation_update(w, updated, &pres_id, Some(new_index));
            }
            Err(e) => w.set_error(format!("Failed to add slide after: {e}")),
        }
    }
    close_context_menu(w);
    cx.notify();
}

pub(crate) fn duplicate_slide(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(pres_id) = editing_pres_id(w) {
        match w.services.presentations.duplicate_slide(&pres_id, index) {
            Ok((updated, new_index)) => {
                apply_presentation_update(w, updated, &pres_id, Some(new_index));
            }
            Err(e) => w.set_error(format!("Failed to duplicate slide: {e}")),
        }
    }
    close_context_menu(w);
    cx.notify();
}

pub(crate) fn select_slide(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    w.editor.selected_slide_index = Some(index);
    crate::ui::navigation::load_slide_for_editing(w);
    cx.notify();
}

pub(crate) fn delete_slide(w: &mut MainWindow, slide_id: String, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(pres_id) = editing_pres_id(w) {
        match w.services.presentations.delete_slide(&pres_id, &slide_id) {
            Ok(updated) => {
                if let Some(cur) = w.editor.selected_slide_index {
                    if updated.slides.is_empty() {
                        w.editor.selected_slide_index = None;
                    } else if cur >= updated.slides.len() {
                        w.editor.selected_slide_index =
                            Some(updated.slides.len().saturating_sub(1));
                    }
                }
                if w.presenting.presentation.as_ref().map(|p| p.id.as_str()) == Some(&pres_id) {
                    w.presenting.slide_index = w
                        .presenting
                        .slide_index
                        .min(updated.slides.len().saturating_sub(1));
                }
                apply_presentation_update(w, updated, &pres_id, w.editor.selected_slide_index);
            }
            Err(e) => w.set_error(format!("Failed to delete slide: {e}")),
        }
    }
    close_context_menu(w);
    cx.notify();
}

pub(crate) fn move_slide_up(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    // The first slide cannot move up, so nothing is changed and no undo
    // step is taken: an undo that does nothing reads as a broken undo.
    if index == 0 {
        cx.notify();
        return;
    }
    w.push_undo();
    let Some(pres_id) = editing_pres_id(w) else {
        cx.notify();
        return;
    };
    let mut slides = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.slides.clone())
        .unwrap_or_default();
    if index < slides.len() {
        slides.swap(index, index - 1);
        let ids: Vec<String> = slides.iter().map(|s| s.id.clone()).collect();
        match w.services.presentations.reorder_slides(&pres_id, &ids) {
            Ok(()) => {
                if let Some(ref mut pres) = w.editor.editing {
                    pres.slides = slides.clone();
                }
                if let Some(ref mut pres) = w.presenting.presentation
                    && pres.id == pres_id
                {
                    pres.slides = slides;
                    w.presenting.slide_index = index - 1;
                }
                w.editor.selected_slide_index = Some(index - 1);
            }
            Err(e) => w.set_error(format!("Failed to reorder: {e}")),
        }
    }
    cx.notify();
}

pub(crate) fn move_slide_down(w: &mut MainWindow, index: usize, cx: &mut Context<MainWindow>) {
    // The last slide cannot move down, so nothing is changed and no undo
    // step is taken: an undo that does nothing reads as a broken undo.
    let is_last = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .is_none_or(|pres| index + 1 >= pres.slides.len());
    if is_last {
        cx.notify();
        return;
    }
    w.push_undo();
    let Some(pres_id) = editing_pres_id(w) else {
        cx.notify();
        return;
    };
    let mut slides = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.slides.clone())
        .unwrap_or_default();
    if index + 1 < slides.len() {
        slides.swap(index, index + 1);
        let ids: Vec<String> = slides.iter().map(|s| s.id.clone()).collect();
        match w.services.presentations.reorder_slides(&pres_id, &ids) {
            Ok(()) => {
                if let Some(ref mut pres) = w.editor.editing {
                    pres.slides = slides.clone();
                }
                if let Some(ref mut pres) = w.presenting.presentation
                    && pres.id == pres_id
                {
                    pres.slides = slides;
                    w.presenting.slide_index = index + 1;
                }
                w.editor.selected_slide_index = Some(index + 1);
            }
            Err(e) => w.set_error(format!("Failed to reorder: {e}")),
        }
    }
    cx.notify();
}

pub(crate) fn group_label_changed(w: &mut MainWindow, label: String, cx: &mut Context<MainWindow>) {
    let index = w.editor.selected_slide_index.unwrap_or(0);
    apply_group_label(w, index, label, cx);
    cx.notify();
}

/// Apply a new group label to the slide at `index` (in both the editing and
/// presenting copies) and persist it. Empty/whitespace clears the group.
fn apply_group_label(
    w: &mut MainWindow,
    index: usize,
    label: String,
    cx: &mut Context<MainWindow>,
) {
    // The group label is an edit like any other, so it can be undone. The
    // snapshot is coalesced, because the label is typed.
    w.push_undo_coalesced();
    let label_opt = if label.trim().is_empty() {
        None
    } else {
        Some(label)
    };
    let mut updated_slide: Option<crate::domain::Slide> = None;
    if let Some(ref mut pres) = w.editor.editing
        && let Some(slide) = pres.slides.get_mut(index)
    {
        slide.group = label_opt.clone();
        updated_slide = Some(slide.clone());
    }
    if let Some(ref mut pres) = w.presenting.presentation
        && let Some(slide) = pres.slides.get_mut(index)
    {
        slide.group = label_opt.clone();
        updated_slide = Some(slide.clone());
    }
    if w.editor.selected_slide_index == Some(index) {
        w.editor.editing_group_label = label_opt.clone().unwrap_or_default();
    }
    if let Some(pres_id) = editing_pres_id(w)
        && let Some(clone) = updated_slide
        && let Err(e) = w.services.presentations.update_slide(&pres_id, &clone)
    {
        w.set_error(format!("Failed to save group label: {e}"));
    }
    cx.notify();
}

pub(crate) fn set_slide_group_label(
    w: &mut MainWindow,
    index: usize,
    label: String,
    cx: &mut Context<MainWindow>,
) {
    apply_group_label(w, index, label, cx);
    close_context_menu(w);
    cx.notify();
}

pub(crate) fn slide_text_changed(w: &mut MainWindow, t: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    w.editor.editing_slide_text = t.clone();
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut()
        && let Some((text, _)) = slide.editable_text_mut(sel_layer)
    {
        *text = t.clone();
        w.layer.text = t;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_font_size_changed(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    w.editor.editing_slide_font_size = s.clone();
    let sel_layer = w.layer.selected_index;
    if let Ok(size) = s.parse::<f32>() {
        // The size is clamped: a size of zero would draw nothing on the
        // projector while the editor still showed a hairline. The rule is
        // the domain's, so every panel clamps the same way.
        let size = crate::domain::TextStyle::clamp_font_size(size);
        if let Some(slide) = w.get_current_slide_mut()
            && let Some((_, style)) = slide.editable_text_mut(sel_layer)
        {
            style.font_size = size;
            w.layer.font_size = s;
        }
        save_current_slide(w, cx);
    }
    cx.notify();
}

/// Show the size the slide really holds, once the operator is done typing.
///
/// The size is clamped, so a value typed outside the range would otherwise
/// stay in the field while the slide drew another one.
pub(crate) fn commit_font_size(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide()
        && let Some((_, style)) = slide.editable_text(sel_layer)
    {
        let size = style.font_size.to_string();
        w.editor.editing_slide_font_size = size.clone();
        w.layer.font_size = size;
    }
    cx.notify();
}

pub(crate) fn slide_alignment_changed(
    w: &mut MainWindow,
    a: TextAlignment,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.alignment = a;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_color_changed(
    w: &mut MainWindow,
    c: crate::domain::Color,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    edit_text_style(w, |style| {
        style.color = c;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_shadow_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.shadow = v;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_outline_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.outline = v;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_bold_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.bold = v;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_italic_toggled(w: &mut MainWindow, v: bool, cx: &mut Context<MainWindow>) {
    w.push_undo();
    edit_text_style(w, |style| {
        style.italic = v;
    });
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_position_preset(
    w: &mut MainWindow,
    x: f32,
    y: f32,
    cx: &mut Context<MainWindow>,
) {
    let sel_layer = w.layer.selected_index;
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        // The preset moves the object the panel shows: the text object the
        // editor is editing, or the slide's text when it has one. The
        // renderer takes the text's place from the object, not from the
        // style — and a preset that moved a shape while the panel showed the
        // text would move the wrong thing.
        let selected_is_text = sel_layer
            .and_then(|idx| slide.layers.get(idx))
            .map(|layer| matches!(layer.content, crate::domain::ObjectContent::Text { .. }))
            .unwrap_or(false);
        if selected_is_text
            && let Some(idx) = sel_layer
            && let Some(layer) = slide.layers.get_mut(idx)
        {
            layer.position_x = x;
            layer.position_y = y;
            // The geometry fields show the selected object, so they follow
            // it.
            w.layer.pos_x = format!("{x:.3}");
            w.layer.pos_y = format!("{y:.3}");
        } else if let Some(layer) = slide
            .layers
            .iter_mut()
            .find(|layer| matches!(layer.content, crate::domain::ObjectContent::Text { .. }))
        {
            layer.position_x = x;
            layer.position_y = y;
        } else if let SlideContent::Text { ref mut style, .. } = slide.content {
            // A slide with no objects at all: the content is the object.
            style.position_x = x;
            style.position_y = y;
        }
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_background_changed(
    w: &mut MainWindow,
    bg: crate::domain::Background,
    cx: &mut Context<MainWindow>,
) {
    // The background is an edit like any other, so it can be undone.
    w.push_undo();
    if let Some(s) = w.get_current_slide_mut() {
        s.background = bg;
    }
    // The background is part of the slide, so it is saved with it: leaving it
    // in memory would lose it on the next launch.
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_transition_changed(
    w: &mut MainWindow,
    t: Transition,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    let dur_str = match &t {
        Transition::Cut => String::from("500"),
        Transition::Fade { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => duration_ms.to_string(),
    };
    w.editor.editing_transition_duration = dur_str;
    if let Some(s) = w.get_current_slide_mut() {
        s.transition = t;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn transition_duration_changed(
    w: &mut MainWindow,
    s: String,
    cx: &mut Context<MainWindow>,
) {
    // The duration is an edit like any other, so it can be undone. The
    // snapshot is coalesced, because the value is typed.
    w.push_undo_coalesced();
    w.editor.editing_transition_duration = s.clone();
    if let Ok(ms) = s.parse::<u64>()
        && let Some(slide) = w.get_current_slide_mut()
    {
        slide.transition = match slide.transition {
            Transition::Cut => Transition::Cut,
            Transition::Fade { .. } => Transition::Fade { duration_ms: ms },
            Transition::Slide { .. } => Transition::Slide { duration_ms: ms },
            Transition::Dissolve { .. } => Transition::Dissolve { duration_ms: ms },
            Transition::Push { direction, .. } => Transition::Push {
                duration_ms: ms,
                direction,
            },
            Transition::Zoom { .. } => Transition::Zoom { duration_ms: ms },
            Transition::Flip { .. } => Transition::Flip { duration_ms: ms },
            Transition::Clock { .. } => Transition::Clock { duration_ms: ms },
            Transition::Wipe { angle_deg, .. } => Transition::Wipe {
                duration_ms: ms,
                angle_deg,
            },
        };
    }
    // The duration belongs to the slide's transition, so it is saved with
    // the slide: leaving it in memory would lose it on the next launch.
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn save_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // The Text tab writes the slide's text and font size into the model as
    // the operator types, and the canvas writes an inline edit straight into
    // the model too, so the model is already the truth. Copying the editor's
    // mirror back into the model here would overwrite the slide's real text
    // whenever the mirror did not match it — which is what happened while a
    // shape was selected, and it emptied the slide's text.
    //
    // "Save" is not an edit either, so it does not add a history entry: an
    // undo step that restores the state that is already on screen would
    // swallow the first Ctrl+Z.
    if let Some(slide) = w.get_current_slide_mut() {
        let clone = slide.clone();
        w.persist_slide(clone, cx);
    }
    cx.notify();
}

pub(crate) fn slide_notes_changed(w: &mut MainWindow, notes: String, cx: &mut Context<MainWindow>) {
    w.push_undo_coalesced();
    w.editor.editing_slide_notes = notes.clone();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.notes = if notes.trim().is_empty() {
            None
        } else {
            Some(notes)
        };
    }
    save_current_slide(w, cx);
    cx.notify();
}

/// Turn the slide into a text slide.
///
/// The slide's objects are the picture, so a conversion has to make them
/// match the panel: a text slide that keeps its image object shows the
/// picture while the panel says the slide is a title. The text the slide
/// already had is kept — the operator is changing the kind, not the words.
pub(crate) fn convert_to_text(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(slide) = w.get_current_slide() else {
        return;
    };
    if slide.content_kind() == crate::domain::SlideKind::Text {
        // The slide already is a text slide: nothing changes, so the undo
        // stack does not get an entry that restores what is on the screen.
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        let (text, style) = slide
            .layers
            .iter()
            .find_map(|layer| match &layer.content {
                crate::domain::ObjectContent::Text { text, style, .. } => {
                    Some((text.clone(), style.clone()))
                }
                _ => None,
            })
            .or_else(|| match &slide.content {
                SlideContent::Text { text, style } => Some((text.clone(), style.clone())),
                _ => None,
            })
            .unwrap_or_else(|| (String::new(), TextStyle::default()));
        let mut object = crate::domain::Object::new_text(text.clone());
        if let crate::domain::ObjectContent::Text {
            style: object_style,
            ..
        } = &mut object.content
        {
            *object_style = style.clone();
        }
        slide.layers = vec![object];
        slide.content = SlideContent::Text { text, style };
    }
    w.layer.selected_index = None;
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn convert_to_image(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(slide) = w.get_current_slide() else {
        return;
    };
    if slide.content_kind() == crate::domain::SlideKind::Image {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        // A media slide is one media object covering the slide: the renderer
        // draws the slide's objects, so the media has to be one of them.
        let fit = slide_image_fit(slide);
        slide.layers = vec![crate::domain::Object::new_image(String::new(), fit)];
        slide.content = SlideContent::Image {
            path: String::new(),
            fit,
        };
    }
    w.layer.selected_index = None;
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn convert_to_video(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(slide) = w.get_current_slide() else {
        return;
    };
    if slide.content_kind() == crate::domain::SlideKind::Video {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.layers = vec![crate::domain::Object::new_video(String::new(), None)];
        slide.content = SlideContent::Video {
            path: String::new(),
            thumbnail: None,
        };
    }
    w.layer.selected_index = None;
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn pick_image_file(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
        .set_title("Select Background Image")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let fit = slide_image_fit(slide);
        let path = path.to_string_lossy().into_owned();
        set_slide_media(slide, &path, fit, None, false);
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn slide_image_fit_changed(
    w: &mut MainWindow,
    fit: ImageFit,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        if let Some(layer) = slide
            .layers
            .iter_mut()
            .find(|layer| matches!(layer.content, crate::domain::ObjectContent::Image { .. }))
            && let crate::domain::ObjectContent::Image { fit: ref mut f, .. } = layer.content
        {
            *f = fit;
        }
        if let SlideContent::Image { fit: ref mut f, .. } = slide.content {
            *f = fit;
        }
    }
    save_current_slide(w, cx);
    cx.notify();
}

/// The image fit a slide uses, taken from its media object.
pub(crate) fn slide_image_fit(slide: &crate::domain::Slide) -> ImageFit {
    slide.image().map(|(_, fit)| fit).unwrap_or_default()
}

/// Point a slide's media object at a file.
///
/// The media is a normal object on the slide, so the renderer draws it like
/// any other object.
fn set_slide_media(
    slide: &mut crate::domain::Slide,
    path: &str,
    fit: ImageFit,
    thumbnail: Option<String>,
    is_video: bool,
) {
    if let Some(layer) = slide.layers.iter_mut().find(|layer| {
        matches!(
            layer.content,
            crate::domain::ObjectContent::Image { .. } | crate::domain::ObjectContent::Video { .. }
        )
    }) {
        if is_video {
            layer.content = crate::domain::ObjectContent::Video {
                path: path.to_string(),
                thumbnail: thumbnail.clone(),
            };
        } else {
            layer.content = crate::domain::ObjectContent::Image {
                path: path.to_string(),
                fit,
            };
        }
    } else {
        let object = if is_video {
            crate::domain::Object::new_video(path.to_string(), thumbnail.clone())
        } else {
            crate::domain::Object::new_image(path.to_string(), fit)
        };
        // The media goes on top: the slide is drawn back to front.
        slide.add_object_on_top(object);
    }

    // The legacy content is kept in step, so a slide that is opened by an
    // older version still shows the media.
    slide.content = if is_video {
        SlideContent::Video {
            path: path.to_string(),
            thumbnail,
        }
    } else {
        SlideContent::Image {
            path: path.to_string(),
            fit,
        }
    };
}

pub(crate) fn pick_video_file(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Videos", &["mp4", "mov", "avi", "mkv", "webm"])
        .set_title("Select Video")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let path_str = path.to_string_lossy().into_owned();
        let thumbnail = crate::media::extract_thumbnail(&path_str, 1.0);
        // A video is a video because the operator picked a video: a
        // thumbnail that could not be extracted must not turn it into an
        // image, or the slide would show nothing and never play.
        set_slide_media(slide, &path_str, ImageFit::default(), thumbnail, true);
        crate::ui::video::open_video(w, &path_str);
    }
    save_current_slide(w, cx);
    cx.notify();
}

/// Apply an image file chosen via the async pick flow ([`Message::ImageFilePicked`]).
pub(crate) fn image_file_picked(
    w: &mut MainWindow,
    path: Option<String>,
    cx: &mut Context<MainWindow>,
) {
    let Some(path) = path else {
        return;
    };
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        let fit = slide_image_fit(slide);
        set_slide_media(slide, &path, fit, None, false);
    }
    save_current_slide(w, cx);
    cx.notify();
}

/// Apply a video file chosen via the async pick flow ([`Message::VideoFilePicked`]).
pub(crate) fn video_file_picked(
    w: &mut MainWindow,
    path: Option<String>,
    cx: &mut Context<MainWindow>,
) {
    let Some(path) = path else {
        return;
    };
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        let thumbnail = crate::media::extract_thumbnail(&path, 1.0);
        set_slide_media(slide, &path, ImageFit::default(), thumbnail, true);
        crate::ui::video::open_video(w, &path);
    }
    save_current_slide(w, cx);
    cx.notify();
}
