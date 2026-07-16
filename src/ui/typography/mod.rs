use crate::domain::{Color, ObjectContent, TextTransform};
use crate::ui::editor::object_helpers::{save_current_slide, selected_layer_mut};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::Task;

/// Messages owned by the Typography editor feature (see `AGENTS.md`).
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

/// Dispatch a Typography message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::SelectedLayerFontFamilyChanged(f) => font_family_changed(w, f),
        Message::SelectedLayerLineHeightChanged(s) => line_height_changed(w, s),
        Message::SelectedLayerLetterSpacingChanged(s) => letter_spacing_changed(w, s),
        Message::SelectedLayerTextTransform(t) => text_transform_changed(w, t),
        Message::SelectedLayerGlowToggled => glow_toggled(w),
        Message::SelectedLayerGlowColorR(v) => glow_color_r(w, v),
        Message::SelectedLayerGlowColorG(v) => glow_color_g(w, v),
        Message::SelectedLayerGlowColorB(v) => glow_color_b(w, v),
        Message::SelectedLayerGlowRadiusChanged(s) => glow_radius_changed(w, s),
        Message::SelectedLayerTextStrokeWidthChanged(s) => text_stroke_width_changed(w, s),
        Message::SelectedLayerTextStrokeColorR(v) => text_stroke_color_r(w, v),
        Message::SelectedLayerTextStrokeColorG(v) => text_stroke_color_g(w, v),
        Message::SelectedLayerTextStrokeColorB(v) => text_stroke_color_b(w, v),
        Message::SelectedLayerTextColorHex(hex) => text_color_hex(w, hex),
    }
}

pub(crate) fn font_family_changed(w: &mut MainWindow, family: String) -> Task<RootMessage> {
    w.layer.font_family = family.clone();
    let trimmed = family.trim().to_string();
    if trimmed.is_empty() {
        return Task::none();
    }
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.font_family = trimmed;
        save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn line_height_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.layer.line_height = s.clone();
    if let Ok(v) = s.parse::<f32>()
        && v > 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.line_height_multiplier = v;
        save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn letter_spacing_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.layer.letter_spacing = s.clone();
    if let Ok(v) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.letter_spacing = v;
        save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn text_transform_changed(w: &mut MainWindow, t: TextTransform) -> Task<RootMessage> {
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.text_transform = t;
        save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn glow_toggled(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.glow_enabled = !style.glow_enabled;
        save_current_slide(w);
    }
    Task::none()
}

macro_rules! glow_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<RootMessage> {
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.glow_color.$channel = v;
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

glow_channel!(glow_color_r, r);
glow_channel!(glow_color_g, g);
glow_channel!(glow_color_b, b);

pub(crate) fn glow_radius_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.layer.glow_radius = s.clone();
    if let Ok(v) = s.parse::<f32>()
        && v >= 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.glow_radius = v;
        save_current_slide(w);
    }
    Task::none()
}

pub(crate) fn text_stroke_width_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.layer.text_stroke_width = s.clone();
    if let Ok(v) = s.parse::<f32>()
        && v >= 0.0
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.text_stroke_width = v;
        save_current_slide(w);
    }
    Task::none()
}

macro_rules! stroke_color_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<RootMessage> {
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.text_stroke_color.$channel = v;
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

stroke_color_channel!(text_stroke_color_r, r);
stroke_color_channel!(text_stroke_color_g, g);
stroke_color_channel!(text_stroke_color_b, b);

pub(crate) fn text_color_hex(w: &mut MainWindow, hex: String) -> Task<RootMessage> {
    let stripped = hex.trim().trim_start_matches('#');
    if stripped.len() != 6 {
        return Task::none();
    }
    let Ok(r) = u8::from_str_radix(&stripped[0..2], 16) else {
        return Task::none();
    };
    let Ok(g) = u8::from_str_radix(&stripped[2..4], 16) else {
        return Task::none();
    };
    let Ok(b) = u8::from_str_radix(&stripped[4..6], 16) else {
        return Task::none();
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
        save_current_slide(w);
    }
    Task::none()
}
