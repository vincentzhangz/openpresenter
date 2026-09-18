use crate::domain::{ImageFit, SlideContent, TextAlignment, TextStyle, Transition};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::{Point, Task};

/// Messages owned by the Slides editor feature.
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
    SlideAlignmentChanged(TextAlignment),
    SlideColorChanged(crate::domain::Color),
    SlideShadowToggled(bool),
    SlideOutlineToggled(bool),
    SlideBoldToggled(bool),
    SlideItalicToggled(bool),
    SlidePositionPreset(f32, f32),
    SlideBackgroundChanged(crate::domain::Background),
    SlideTransitionChanged(Transition),
    TransitionDurationChanged(String),
    SaveSlide,
    TextDragStarted,
    TextDragged(Point),
    TextDragEnded,
    ConvertSlideToText,
    ConvertSlideToImage,
    ConvertSlideToVideo,
    PickImageFile,
    SlideImageFitChanged(ImageFit),
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

/// Dispatch a Slides message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::AddSlide => add_slide(w),
        Message::AddSlideAfter(i) => add_slide_after(w, i),
        Message::DuplicateSlide(i) => duplicate_slide(w, i),
        Message::SelectSlide(i) => select_slide(w, i),
        Message::DeleteSlide(id) => delete_slide(w, id),
        Message::MoveSlideUp(i) => move_slide_up(w, i),
        Message::MoveSlideDown(i) => move_slide_down(w, i),
        Message::SetSlideGroupLabel(i, label) => set_slide_group_label(w, i, label),
        Message::GroupLabelChanged(label) => group_label_changed(w, label),
        Message::SlideTextChanged(t) => slide_text_changed(w, t),
        Message::SlideFontSizeChanged(s) => slide_font_size_changed(w, s),
        Message::SlideAlignmentChanged(a) => slide_alignment_changed(w, a),
        Message::SlideColorChanged(c) => slide_color_changed(w, c),
        Message::SlideShadowToggled(v) => slide_shadow_toggled(w, v),
        Message::SlideOutlineToggled(v) => slide_outline_toggled(w, v),
        Message::SlideBoldToggled(v) => slide_bold_toggled(w, v),
        Message::SlideItalicToggled(v) => slide_italic_toggled(w, v),
        Message::SlidePositionPreset(x, y) => slide_position_preset(w, x, y),
        Message::SlideBackgroundChanged(bg) => slide_background_changed(w, bg),
        Message::SlideTransitionChanged(t) => slide_transition_changed(w, t),
        Message::TransitionDurationChanged(s) => transition_duration_changed(w, s),
        Message::SaveSlide => save_slide(w),
        Message::TextDragStarted => Task::none(),
        Message::TextDragged(p) => text_dragged(w, p),
        Message::TextDragEnded => text_drag_ended(w),
        Message::ConvertSlideToText => convert_to_text(w),
        Message::ConvertSlideToImage => convert_to_image(w),
        Message::ConvertSlideToVideo => convert_to_video(w),
        Message::PickImageFile => pick_image_file(w),
        Message::SlideImageFitChanged(fit) => slide_image_fit_changed(w, fit),
        Message::PickVideoFile => pick_video_file(w),
        Message::SlideNotesChanged(notes) => slide_notes_changed(w, notes),
        Message::CopyTextStyle => copy_text_style(w),
        Message::PasteTextStyle => paste_text_style(w),
        Message::SlideFontFamilyCycle => slide_font_family_cycle(w),
        Message::SlideFontSizeStep(delta) => slide_font_size_step(w, delta),
        Message::SlideTextTransformCycle => slide_text_transform_cycle(w),
        Message::SlideTextColorCycle => slide_text_color_cycle(w),
        Message::SlideGlowToggled(v) => slide_glow_toggled(w, v),
        Message::StartInlineTextEditing(idx) => start_inline_text_editing(w, idx),
        Message::EndInlineTextEditing => end_inline_text_editing(w),
    }
}

pub(crate) fn start_inline_text_editing(
    w: &mut MainWindow,
    idx_opt: Option<usize>,
) -> Task<RootMessage> {
    if let Some(idx) = idx_opt {
        w.layer.selected_index = Some(idx);
        w.load_layer_for_editing();
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
            w.load_layer_for_editing();
        }
    }
    w.shell.inspector_tab = crate::ui::messages::InspectorTab::Text;
    w.editor.inline_editing = true;
    Task::none()
}

pub(crate) fn end_inline_text_editing(w: &mut MainWindow) -> Task<RootMessage> {
    w.editor.inline_editing = false;
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
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

pub(crate) fn copy_text_style(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide() {
        match &slide.content {
            SlideContent::Text { style, .. } => {
                w.editor.copied_style = Some(style.clone());
            }
            _ => {
                if let Some(idx) = w.layer.selected_index
                    && let Some(layer) = slide.layers.get(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &layer.content
                {
                    w.editor.copied_style = Some(style.clone());
                }
            }
        }
    }
    Task::none()
}

pub(crate) fn paste_text_style(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(copied) = w.editor.copied_style.clone() else {
        return Task::none();
    };
    let sel_layer = w.layer.selected_index;
    w.push_undo();
    let mut updated_font_size = None;
    if let Some(slide) = w.get_current_slide_mut() {
        match &mut slide.content {
            SlideContent::Text { style, .. } => {
                *style = copied.clone();
                updated_font_size = Some((copied.font_size as u32).to_string());
            }
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    *style = copied.clone();
                    updated_font_size = Some((copied.font_size as u32).to_string());
                }
            }
        }
    }
    if let Some(fs) = updated_font_size {
        w.editor.editing_slide_font_size = fs.clone();
        w.layer.font_size = fs;
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_font_family_cycle(w: &mut MainWindow) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    let mut updated_font = None;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    None
                }
            }
        };
        if let Some(style) = style {
            let cur = style.font_family.as_str();
            let next = match FONT_FAMILIES
                .iter()
                .position(|&f| f.eq_ignore_ascii_case(cur))
            {
                Some(idx) => FONT_FAMILIES[(idx + 1) % FONT_FAMILIES.len()],
                None => FONT_FAMILIES[0],
            };
            style.font_family = next.to_string();
            updated_font = Some(next.to_string());
        }
    }
    if let Some(font) = updated_font {
        w.layer.font_family = font;
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_font_size_step(w: &mut MainWindow, delta: f32) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    let mut updated_font_size = None;
    if let Some(slide) = w.get_current_slide_mut() {
        match &mut slide.content {
            SlideContent::Text { style, .. } => {
                style.font_size = (style.font_size + delta).clamp(12.0, 200.0);
                updated_font_size = Some((style.font_size as u32).to_string());
            }
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    style.font_size = (style.font_size + delta).clamp(12.0, 200.0);
                    updated_font_size = Some((style.font_size as u32).to_string());
                }
            }
        }
    }
    if let Some(fs) = updated_font_size {
        w.editor.editing_slide_font_size = fs.clone();
        w.layer.font_size = fs;
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_text_transform_cycle(w: &mut MainWindow) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    None
                }
            }
        };
        if let Some(style) = style {
            style.text_transform = match style.text_transform {
                crate::domain::TextTransform::None => crate::domain::TextTransform::Uppercase,
                crate::domain::TextTransform::Uppercase => crate::domain::TextTransform::Lowercase,
                crate::domain::TextTransform::Lowercase => crate::domain::TextTransform::Capitalize,
                crate::domain::TextTransform::Capitalize => crate::domain::TextTransform::None,
            };
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_text_color_cycle(w: &mut MainWindow) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    None
                }
            }
        };
        if let Some(style) = style {
            let next_color = match COLOR_PRESETS.iter().position(|&c| c == style.color) {
                Some(idx) => COLOR_PRESETS[(idx + 1) % COLOR_PRESETS.len()],
                None => COLOR_PRESETS[1],
            };
            style.color = next_color;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_glow_toggled(w: &mut MainWindow, enabled: bool) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        match &mut slide.content {
            SlideContent::Text { style, .. } => {
                style.glow_enabled = enabled;
            }
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    style.glow_enabled = enabled;
                }
            }
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

/// Close the slide context menu (used after an action triggered from it).
fn close_context_menu(w: &mut MainWindow) {
    w.presenting.slide_context_index = None;
    w.presenting.group_submenu = false;
    w.presenting.cue_submenu = false;
    w.presenting.transition_submenu = false;
}

pub(crate) fn add_slide(w: &mut MainWindow) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(pres_id) = pres_id {
        match w.services.presentations.add_slide(&pres_id) {
            Ok(updated) => {
                let new_index = updated.slides.len().saturating_sub(1);
                w.editor.selected_slide_index = Some(new_index);
                w.editor.editing = Some(updated.clone());
                if w.presenting.presentation.as_ref().map(|p| &p.id) == Some(&pres_id) {
                    w.presenting.presentation = Some(updated.clone());
                    w.presenting.slide_index = new_index;
                }
                w.load_presentations();
                if w.shell.current_mode == crate::ui::messages::ViewMode::Edit {
                    w.load_slide_for_editing();
                }
            }
            Err(e) => w.set_error(format!("Failed to add slide: {e}")),
        }
    }
    Task::none()
}

pub(crate) fn add_slide_after(w: &mut MainWindow, after_index: usize) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(pres_id) = pres_id {
        match w
            .services
            .presentations
            .add_slide_after(&pres_id, after_index)
        {
            Ok((updated, new_index)) => {
                w.editor.selected_slide_index = Some(new_index);
                w.editor.editing = Some(updated.clone());
                if w.presenting.presentation.as_ref().map(|p| &p.id) == Some(&pres_id) {
                    w.presenting.presentation = Some(updated.clone());
                    w.presenting.slide_index = new_index;
                }
                w.load_presentations();
                if w.shell.current_mode == crate::ui::messages::ViewMode::Edit {
                    w.load_slide_for_editing();
                }
            }
            Err(e) => w.set_error(format!("Failed to add slide after: {e}")),
        }
    }
    close_context_menu(w);
    Task::none()
}

pub(crate) fn duplicate_slide(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(pres_id) = pres_id {
        match w.services.presentations.duplicate_slide(&pres_id, index) {
            Ok((updated, new_index)) => {
                w.editor.selected_slide_index = Some(new_index);
                w.editor.editing = Some(updated.clone());
                if w.presenting.presentation.as_ref().map(|p| &p.id) == Some(&pres_id) {
                    w.presenting.presentation = Some(updated.clone());
                    w.presenting.slide_index = new_index;
                }
                w.load_presentations();
                if w.shell.current_mode == crate::ui::messages::ViewMode::Edit {
                    w.load_slide_for_editing();
                }
            }
            Err(e) => w.set_error(format!("Failed to duplicate slide: {e}")),
        }
    }
    close_context_menu(w);
    Task::none()
}

pub(crate) fn select_slide(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    w.editor.selected_slide_index = Some(index);
    w.load_slide_for_editing();
    iced::advanced::widget::operate(iced::advanced::widget::operation::scrollable::scroll_to(
        crate::ui::editor::slide_list::scrollable_id(),
        iced::advanced::widget::operation::scrollable::AbsoluteOffset {
            x: Some(0.0_f32),
            y: Some(index as f32 * 96.0),
        },
    ))
}

pub(crate) fn delete_slide(w: &mut MainWindow, slide_id: String) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(pres_id) = pres_id {
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
                w.editor.editing = Some(updated.clone());
                if w.presenting.presentation.as_ref().map(|p| &p.id) == Some(&pres_id) {
                    if updated.slides.is_empty() {
                        w.presenting.slide_index = 0;
                    } else if w.presenting.slide_index >= updated.slides.len() {
                        w.presenting.slide_index = updated.slides.len() - 1;
                    }
                    w.presenting.presentation = Some(updated.clone());
                }
                w.load_presentations();
                if w.shell.current_mode == crate::ui::messages::ViewMode::Edit {
                    w.load_slide_for_editing();
                }
            }
            Err(e) => w.set_error(format!("Failed to delete slide: {e}")),
        }
    }
    close_context_menu(w);
    Task::none()
}

pub(crate) fn move_slide_up(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if index > 0
        && let Some(pres_id) = pres_id
    {
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
    }
    Task::none()
}

pub(crate) fn move_slide_down(w: &mut MainWindow, index: usize) -> Task<RootMessage> {
    w.push_undo();
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(pres_id) = pres_id {
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
    }
    Task::none()
}

pub(crate) fn group_label_changed(w: &mut MainWindow, label: String) -> Task<RootMessage> {
    let index = w.editor.selected_slide_index.unwrap_or(0);
    apply_group_label(w, index, label);
    Task::none()
}

/// Apply a new group label to the slide at `index` (in both the editing and
/// presenting copies) and persist it. Empty/whitespace clears the group.
fn apply_group_label(w: &mut MainWindow, index: usize, label: String) {
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
    let pres_id = w
        .editor
        .editing
        .as_ref()
        .or(w.presenting.presentation.as_ref())
        .map(|p| p.id.clone());
    if let Some(clone) = updated_slide
        && let Some(pres_id) = pres_id
        && let Err(e) = w.services.presentations.update_slide(&pres_id, &clone)
    {
        w.set_error(format!("Failed to save group label: {e}"));
    }
}

pub(crate) fn set_slide_group_label(
    w: &mut MainWindow,
    index: usize,
    label: String,
) -> Task<RootMessage> {
    apply_group_label(w, index, label);
    close_context_menu(w);
    Task::none()
}

pub(crate) fn slide_text_changed(w: &mut MainWindow, t: String) -> Task<RootMessage> {
    w.editor.editing_slide_text = t.clone();
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        if let Some(idx) = sel_layer
            && let Some(layer) = slide.layers.get_mut(idx)
            && let crate::domain::ObjectContent::Text {
                text: ref mut layer_text,
                ..
            } = layer.content
        {
            *layer_text = t.clone();
            w.layer.text = t;
        } else if let SlideContent::Text {
            text: ref mut slide_text,
            ..
        } = slide.content
        {
            *slide_text = t;
        } else if let Some(layer) = slide
            .layers
            .iter_mut()
            .find(|l| matches!(l.content, crate::domain::ObjectContent::Text { .. }))
            && let crate::domain::ObjectContent::Text {
                text: ref mut layer_text,
                ..
            } = layer.content
        {
            *layer_text = t.clone();
            w.layer.text = t;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_font_size_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.editor.editing_slide_font_size = s.clone();
    let sel_layer = w.layer.selected_index;
    if let Ok(size) = s.parse::<f32>() {
        if let Some(slide) = w.get_current_slide_mut() {
            if let Some(idx) = sel_layer
                && let Some(layer) = slide.layers.get_mut(idx)
                && let crate::domain::ObjectContent::Text {
                    style: ref mut layer_style,
                    ..
                } = layer.content
            {
                layer_style.font_size = size;
                w.layer.font_size = s;
            } else if let SlideContent::Text {
                style: ref mut slide_style,
                ..
            } = slide.content
            {
                slide_style.font_size = size;
            } else if let Some(layer) = slide
                .layers
                .iter_mut()
                .find(|l| matches!(l.content, crate::domain::ObjectContent::Text { .. }))
                && let crate::domain::ObjectContent::Text {
                    style: ref mut layer_style,
                    ..
                } = layer.content
            {
                layer_style.font_size = size;
                w.layer.font_size = s;
            }
        }
        crate::ui::editor::object_helpers::save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn slide_alignment_changed(w: &mut MainWindow, a: TextAlignment) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.alignment = a;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_color_changed(
    w: &mut MainWindow,
    c: crate::domain::Color,
) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.color = c;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_shadow_toggled(w: &mut MainWindow, v: bool) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.shadow = v;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_outline_toggled(w: &mut MainWindow, v: bool) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.outline = v;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_bold_toggled(w: &mut MainWindow, v: bool) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.bold = v;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_italic_toggled(w: &mut MainWindow, v: bool) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        let style = match &mut slide.content {
            SlideContent::Text { style, .. } if sel_layer.is_none() => Some(style),
            _ => {
                if let Some(idx) = sel_layer
                    && let Some(layer) = slide.layers.get_mut(idx)
                    && let crate::domain::ObjectContent::Text { style, .. } = &mut layer.content
                {
                    Some(style)
                } else {
                    slide.layers.iter_mut().find_map(|l| match &mut l.content {
                        crate::domain::ObjectContent::Text { style, .. } => Some(style),
                        _ => None,
                    })
                }
            }
        };
        if let Some(style) = style {
            style.italic = v;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_position_preset(w: &mut MainWindow, x: f32, y: f32) -> Task<RootMessage> {
    let sel_layer = w.layer.selected_index;
    if let Some(slide) = w.get_current_slide_mut() {
        if let Some(idx) = sel_layer
            && let Some(layer) = slide.layers.get_mut(idx)
        {
            layer.position_x = x;
            layer.position_y = y;
            w.layer.pos_x = format!("{x:.3}");
            w.layer.pos_y = format!("{y:.3}");
        } else if let SlideContent::Text { ref mut style, .. } = slide.content {
            style.position_x = x;
            style.position_y = y;
        }
    }
    crate::ui::editor::object_helpers::save_current_slide(w);
    Task::none()
}

pub(crate) fn slide_background_changed(
    w: &mut MainWindow,
    bg: crate::domain::Background,
) -> Task<RootMessage> {
    if let Some(s) = w.get_current_slide_mut() {
        s.background = bg;
    }
    Task::none()
}

pub(crate) fn slide_transition_changed(w: &mut MainWindow, t: Transition) -> Task<RootMessage> {
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
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        w.persist_slide(clone);
    }
    Task::none()
}

pub(crate) fn transition_duration_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
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
    Task::none()
}

pub(crate) fn save_slide(w: &mut MainWindow) -> Task<RootMessage> {
    w.push_undo();
    let text = w.editor.editing_slide_text.clone();
    let fs_str = w.editor.editing_slide_font_size.clone();
    if let Some(slide) = w.get_current_slide_mut() {
        if let SlideContent::Text {
            text: ref mut slide_text,
            ref mut style,
        } = slide.content
        {
            *slide_text = text;
            if let Ok(size) = fs_str.parse::<f32>() {
                style.font_size = size;
            }
        }
        let clone = slide.clone();
        w.persist_slide(clone);
    }
    Task::none()
}

pub(crate) fn text_dragged(w: &mut MainWindow, point: Point) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Text { ref mut style, .. } = slide.content
    {
        style.position_x = point.x;
        style.position_y = point.y;
    }
    Task::none()
}

pub(crate) fn text_drag_ended(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        w.persist_slide(clone);
    }
    Task::none()
}

pub(crate) fn slide_notes_changed(w: &mut MainWindow, notes: String) -> Task<RootMessage> {
    w.editor.editing_slide_notes = notes.clone();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.notes = if notes.trim().is_empty() {
            None
        } else {
            Some(notes)
        };
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn convert_to_text(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Text { .. })
    {
        slide.content = SlideContent::Text {
            text: String::new(),
            style: TextStyle::default(),
        };
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn convert_to_image(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Image { .. })
    {
        slide.content = SlideContent::Image {
            path: String::new(),
            fit: ImageFit::default(),
        };
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn convert_to_video(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide_mut()
        && !matches!(slide.content, SlideContent::Video { .. })
    {
        slide.content = SlideContent::Video {
            path: String::new(),
            thumbnail: None,
        };
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn pick_image_file(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
        .set_title("Select Background Image")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let fit = match &slide.content {
            SlideContent::Image { fit, .. } => *fit,
            _ => ImageFit::default(),
        };
        slide.content = SlideContent::Image {
            path: path.to_string_lossy().into_owned(),
            fit,
        };
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn slide_image_fit_changed(w: &mut MainWindow, fit: ImageFit) -> Task<RootMessage> {
    if let Some(slide) = w.get_current_slide_mut()
        && let SlideContent::Image { fit: ref mut f, .. } = slide.content
    {
        *f = fit;
        let c = slide.clone();
        w.persist_slide(c);
    }
    Task::none()
}

pub(crate) fn pick_video_file(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Videos", &["mp4", "mov", "avi", "mkv", "webm"])
        .set_title("Select Video")
        .pick_file()
        && let Some(slide) = w.get_current_slide_mut()
    {
        let path_str = path.to_string_lossy().into_owned();
        let thumbnail = crate::media::extract_thumbnail(&path_str, 1.0);
        slide.content = SlideContent::Video {
            path: path_str.clone(),
            thumbnail,
        };
        let c = slide.clone();
        w.persist_slide(c);
        crate::ui::video::open_video(w, &path_str);
    }
    Task::none()
}
