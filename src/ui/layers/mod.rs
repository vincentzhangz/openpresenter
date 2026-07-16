use crate::domain::{ObjectContent, ShapeType};
use crate::ui::editor::object_helpers::{save_current_slide, selected_layer_mut};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message as RootMessage;
use iced::{Point, Task};

/// Messages owned by the Layers editor feature (see `AGENTS.md`).
#[derive(Debug, Clone)]
pub enum Message {
    AddTextLayer,
    AddShapeLayer(ShapeType),
    SelectLayer(Option<usize>),
    DeleteSelectedLayer,
    MoveSelectedLayerUp,
    MoveSelectedLayerDown,
    ToggleSelectedLayerVisibility,
    ToggleSelectedLayerLock,
    SelectedLayerOpacityChanged(f32),
    SelectedLayerTextChanged(String),
    SelectedLayerFontSizeChanged(String),
    SelectedLayerTextColorR(u8),
    SelectedLayerTextColorG(u8),
    SelectedLayerTextColorB(u8),
    SelectedLayerTextShadowToggled,
    SelectedLayerTextOutlineToggled,
    SelectedLayerTextBoldToggled,
    SelectedLayerTextItalicToggled,
    SelectedLayerShapeFillR(u8),
    SelectedLayerShapeFillG(u8),
    SelectedLayerShapeFillB(u8),
    SelectedLayerShapeFillA(u8),
    SelectedLayerShapeStrokeWidthChanged(String),
    SelectedLayerPositionXChanged(String),
    SelectedLayerPositionYChanged(String),
    SelectedLayerWidthChanged(String),
    SelectedLayerHeightChanged(String),
    LayerDragStarted(usize),
    LayerDragged(Point),
    LayerResized {
        position: Point,
        width: f32,
        height: f32,
    },
    LayerDragEnded,
}

/// Dispatch a Layers message.
pub fn update(w: &mut MainWindow, msg: Message) -> Task<RootMessage> {
    match msg {
        Message::AddTextLayer => add_text_layer(w),
        Message::AddShapeLayer(s) => add_shape_layer(w, s),
        Message::SelectLayer(i) => select_layer(w, i),
        Message::DeleteSelectedLayer => delete_selected_layer(w),
        Message::MoveSelectedLayerUp => move_selected_layer_up(w),
        Message::MoveSelectedLayerDown => move_selected_layer_down(w),
        Message::ToggleSelectedLayerVisibility => toggle_selected_layer_visibility(w),
        Message::ToggleSelectedLayerLock => toggle_selected_layer_lock(w),
        Message::SelectedLayerOpacityChanged(v) => selected_layer_opacity_changed(w, v),
        Message::SelectedLayerTextChanged(t) => selected_layer_text_changed(w, t),
        Message::SelectedLayerFontSizeChanged(s) => selected_layer_font_size_changed(w, s),
        Message::SelectedLayerTextColorR(v) => selected_layer_text_color_r(w, v),
        Message::SelectedLayerTextColorG(v) => selected_layer_text_color_g(w, v),
        Message::SelectedLayerTextColorB(v) => selected_layer_text_color_b(w, v),
        Message::SelectedLayerTextShadowToggled => selected_layer_text_shadow_toggled(w),
        Message::SelectedLayerTextOutlineToggled => selected_layer_text_outline_toggled(w),
        Message::SelectedLayerTextBoldToggled => selected_layer_text_bold_toggled(w),
        Message::SelectedLayerTextItalicToggled => selected_layer_text_italic_toggled(w),
        Message::SelectedLayerShapeFillR(v) => selected_layer_shape_fill_r(w, v),
        Message::SelectedLayerShapeFillG(v) => selected_layer_shape_fill_g(w, v),
        Message::SelectedLayerShapeFillB(v) => selected_layer_shape_fill_b(w, v),
        Message::SelectedLayerShapeFillA(v) => selected_layer_shape_fill_a(w, v),
        Message::SelectedLayerShapeStrokeWidthChanged(s) => {
            selected_layer_shape_stroke_width_changed(w, s)
        }
        Message::SelectedLayerPositionXChanged(s) => selected_layer_position_x_changed(w, s),
        Message::SelectedLayerPositionYChanged(s) => selected_layer_position_y_changed(w, s),
        Message::SelectedLayerWidthChanged(s) => selected_layer_width_changed(w, s),
        Message::SelectedLayerHeightChanged(s) => selected_layer_height_changed(w, s),
        Message::LayerDragStarted(i) => layer_drag_started(w, i),
        Message::LayerDragged(p) => layer_dragged(w, p),
        Message::LayerResized {
            position,
            width,
            height,
        } => layer_resized(w, position, width, height),
        Message::LayerDragEnded => layer_drag_ended(w),
    }
}

pub(crate) fn add_text_layer(w: &mut MainWindow) -> Task<RootMessage> {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_objects();
        let max_z = slide.layers.iter().map(|l| l.z_order).max().unwrap_or(-1);
        let mut layer = crate::domain::Object::new_text(String::new());
        layer.z_order = max_z + 1;
        slide.layers.push(layer);
        let new_idx = slide.layers.len() - 1;
        save_current_slide(w);
        w.layer.selected_index = Some(new_idx);
        w.load_layer_for_editing();
    }
    Task::none()
}

pub(crate) fn add_shape_layer(w: &mut MainWindow, shape: ShapeType) -> Task<RootMessage> {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_objects();
        let max_z = slide.layers.iter().map(|l| l.z_order).max().unwrap_or(-1);
        let mut layer = crate::domain::Object::new_shape(shape);
        layer.z_order = max_z + 1;
        slide.layers.push(layer);
        let new_idx = slide.layers.len() - 1;
        save_current_slide(w);
        w.layer.selected_index = Some(new_idx);
        w.load_layer_for_editing();
    }
    Task::none()
}

pub(crate) fn select_layer(w: &mut MainWindow, idx: Option<usize>) -> Task<RootMessage> {
    let mut migrated = false;
    let valid_idx = if let Some(i) = idx {
        if let Some(slide) = w.get_current_slide_mut() {
            if slide.migrate_to_objects() {
                migrated = true;
            }
            if i < slide.layers.len() {
                Some(i)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    if migrated {
        save_current_slide(w);
    }
    w.layer.selected_index = valid_idx;
    w.load_layer_for_editing();
    Task::none()
}

pub(crate) fn delete_selected_layer(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && idx < slide.layers.len()
    {
        slide.layers.remove(idx);
        let clamp = if slide.layers.is_empty() {
            None
        } else {
            Some(idx.saturating_sub(1))
        };
        save_current_slide(w);
        w.layer.selected_index = clamp;
    }
    w.load_layer_for_editing();
    Task::none()
}

pub(crate) fn move_selected_layer_up(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && idx + 1 < slide.layers.len()
    {
        slide.layers[idx].z_order += 1;
        slide.layers[idx + 1].z_order -= 1;
        slide.layers.swap(idx, idx + 1);
        save_current_slide(w);
        w.layer.selected_index = Some(idx + 1);
    }
    Task::none()
}

pub(crate) fn move_selected_layer_down(w: &mut MainWindow) -> Task<RootMessage> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    if idx == 0 {
        return Task::none();
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.layers[idx].z_order -= 1;
        slide.layers[idx - 1].z_order += 1;
        slide.layers.swap(idx, idx - 1);
        save_current_slide(w);
        w.layer.selected_index = Some(idx - 1);
    }
    Task::none()
}

pub(crate) fn toggle_selected_layer_visibility(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.visible = !layer.visible;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn toggle_selected_layer_lock(w: &mut MainWindow) -> Task<RootMessage> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.locked = !layer.locked;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_opacity_changed(w: &mut MainWindow, v: f32) -> Task<RootMessage> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.opacity = v.clamp(0.0, 1.0);
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_text_changed(w: &mut MainWindow, text: String) -> Task<RootMessage> {
    w.layer.text = text.clone();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text {
            text: ref mut t, ..
        } = layer.content
    {
        *t = text;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_font_size_changed(w: &mut MainWindow, s: String) -> Task<RootMessage> {
    w.layer.font_size = s.clone();
    if let Ok(size) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        style.font_size = size;
        save_current_slide(w);
    }
    Task::none()
}

macro_rules! text_color_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<RootMessage> {
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.color.$channel = v;
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

text_color_channel!(selected_layer_text_color_r, r);
text_color_channel!(selected_layer_text_color_g, g);
text_color_channel!(selected_layer_text_color_b, b);

macro_rules! text_toggle {
    ($fn_name:ident, $field:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow) -> Task<RootMessage> {
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.$field = !style.$field;
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

text_toggle!(selected_layer_text_shadow_toggled, shadow);
text_toggle!(selected_layer_text_outline_toggled, outline);
text_toggle!(selected_layer_text_bold_toggled, bold);
text_toggle!(selected_layer_text_italic_toggled, italic);

macro_rules! shape_fill_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<RootMessage> {
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Shape { ref mut fill, .. } = layer.content
            {
                fill.$channel = v;
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

shape_fill_channel!(selected_layer_shape_fill_r, r);
shape_fill_channel!(selected_layer_shape_fill_g, g);
shape_fill_channel!(selected_layer_shape_fill_b, b);
shape_fill_channel!(selected_layer_shape_fill_a, a);

pub(crate) fn selected_layer_shape_stroke_width_changed(
    w: &mut MainWindow,
    s: String,
) -> Task<RootMessage> {
    w.layer.stroke_width = s.clone();
    if let Ok(width) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Shape {
            ref mut stroke_width,
            ..
        } = layer.content
    {
        *stroke_width = width;
        save_current_slide(w);
    }
    Task::none()
}

macro_rules! layer_geom_field {
    ($fn_name:ident, $edit_field:ident, $layer_field:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, s: String) -> Task<RootMessage> {
            w.layer.$edit_field = s.clone();
            if let Ok(v) = s.parse::<f32>()
                && let Some(layer) = selected_layer_mut(w)
            {
                layer.$layer_field = v.clamp(0.0, 1.0);
                save_current_slide(w);
            }
            Task::none()
        }
    };
}

layer_geom_field!(selected_layer_position_x_changed, pos_x, position_x);
layer_geom_field!(selected_layer_position_y_changed, pos_y, position_y);
layer_geom_field!(selected_layer_width_changed, width, width);
layer_geom_field!(selected_layer_height_changed, height, height);

pub(crate) fn layer_drag_started(w: &mut MainWindow, idx: usize) -> Task<RootMessage> {
    select_layer(w, Some(idx))
}

pub(crate) fn layer_dragged(w: &mut MainWindow, new_pos: Point) -> Task<RootMessage> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    let updated = if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && !layer.locked
    {
        layer.position_x = new_pos.x.clamp(0.0, 1.0);
        layer.position_y = new_pos.y.clamp(0.0, 1.0);
        Some((layer.position_x, layer.position_y))
    } else {
        None
    };
    if let Some((x, y)) = updated {
        w.layer.pos_x = format!("{:.3}", x);
        w.layer.pos_y = format!("{:.3}", y);
    }
    Task::none()
}

pub(crate) fn layer_resized(
    w: &mut MainWindow,
    position: Point,
    width: f32,
    height: f32,
) -> Task<RootMessage> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    let updated = if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && !layer.locked
    {
        layer.position_x = position.x.clamp(0.0, 1.0);
        layer.position_y = position.y.clamp(0.0, 1.0);
        layer.width = width.clamp(0.02, 1.0);
        layer.height = height.clamp(0.02, 1.0);
        Some((
            layer.position_x,
            layer.position_y,
            layer.width,
            layer.height,
        ))
    } else {
        None
    };
    if let Some((x, y, width, height)) = updated {
        w.layer.pos_x = format!("{:.3}", x);
        w.layer.pos_y = format!("{:.3}", y);
        w.layer.width = format!("{:.3}", width);
        w.layer.height = format!("{:.3}", height);
    }
    Task::none()
}

pub(crate) fn layer_drag_ended(w: &mut MainWindow) -> Task<RootMessage> {
    save_current_slide(w);
    Task::none()
}
