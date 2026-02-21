use crate::slides::{LayerContent, ShapeType, SlideLayer};
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;
use iced::{Point, Task};

fn save_current_slide(w: &mut MainWindow) {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("layers: save slide: {e}");
        }
    }
}

fn selected_layer_mut(w: &mut MainWindow) -> Option<&mut SlideLayer> {
    let idx = w.layer.selected_index?;
    w.get_current_slide_mut()?.layers.get_mut(idx)
}

pub(crate) fn add_text_layer(w: &mut MainWindow) -> Task<Message> {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_layers();
        let max_z = slide.layers.iter().map(|l| l.z_order).max().unwrap_or(-1);
        let mut layer = SlideLayer::new_text(String::new());
        layer.z_order = max_z + 1;
        slide.layers.push(layer);
        let new_idx = slide.layers.len() - 1;
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("add_text_layer: {e}");
        }
        w.layer.selected_index = Some(new_idx);
        w.load_layer_for_editing();
    }
    Task::none()
}

pub(crate) fn add_shape_layer(w: &mut MainWindow, shape: ShapeType) -> Task<Message> {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_layers();
        let max_z = slide.layers.iter().map(|l| l.z_order).max().unwrap_or(-1);
        let mut layer = SlideLayer::new_shape(shape);
        layer.z_order = max_z + 1;
        slide.layers.push(layer);
        let new_idx = slide.layers.len() - 1;
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("add_shape_layer: {e}");
        }
        w.layer.selected_index = Some(new_idx);
        w.load_layer_for_editing();
    }
    Task::none()
}

pub(crate) fn select_layer(w: &mut MainWindow, idx: Option<usize>) -> Task<Message> {
    let valid_idx = idx.and_then(|i| {
        let layers_len = w.get_current_slide().map(|s| s.layers.len()).unwrap_or(0);
        if i < layers_len { Some(i) } else { None }
    });
    w.layer.selected_index = valid_idx;
    w.load_layer_for_editing();
    Task::none()
}

pub(crate) fn delete_selected_layer(w: &mut MainWindow) -> Task<Message> {
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
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("delete_layer: {e}");
        }
        w.layer.selected_index = clamp;
    }
    w.load_layer_for_editing();
    Task::none()
}

pub(crate) fn move_selected_layer_up(w: &mut MainWindow) -> Task<Message> {
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
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("move_layer_up: {e}");
        }
        w.layer.selected_index = Some(idx + 1);
    }
    Task::none()
}

pub(crate) fn move_selected_layer_down(w: &mut MainWindow) -> Task<Message> {
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
        let clone = slide.clone();
        if let Err(e) = w.repo.update_slide(&clone) {
            eprintln!("move_layer_down: {e}");
        }
        w.layer.selected_index = Some(idx - 1);
    }
    Task::none()
}

pub(crate) fn toggle_selected_layer_visibility(w: &mut MainWindow) -> Task<Message> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.visible = !layer.visible;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn toggle_selected_layer_lock(w: &mut MainWindow) -> Task<Message> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.locked = !layer.locked;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_opacity_changed(w: &mut MainWindow, v: f32) -> Task<Message> {
    if let Some(layer) = selected_layer_mut(w) {
        layer.opacity = v.clamp(0.0, 1.0);
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_text_changed(w: &mut MainWindow, text: String) -> Task<Message> {
    w.layer.text = text.clone();
    if let Some(layer) = selected_layer_mut(w)
        && let LayerContent::Text {
            text: ref mut t, ..
        } = layer.content
    {
        *t = text;
    }
    save_current_slide(w);
    Task::none()
}

pub(crate) fn selected_layer_font_size_changed(w: &mut MainWindow, s: String) -> Task<Message> {
    w.layer.font_size = s.clone();
    if let Ok(size) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let LayerContent::Text { ref mut style, .. } = layer.content
    {
        style.font_size = size;
        save_current_slide(w);
    }
    Task::none()
}

macro_rules! text_color_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<Message> {
            if let Some(layer) = selected_layer_mut(w)
                && let LayerContent::Text { ref mut style, .. } = layer.content
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
        pub(crate) fn $fn_name(w: &mut MainWindow) -> Task<Message> {
            if let Some(layer) = selected_layer_mut(w)
                && let LayerContent::Text { ref mut style, .. } = layer.content
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
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8) -> Task<Message> {
            if let Some(layer) = selected_layer_mut(w)
                && let LayerContent::Shape { ref mut fill, .. } = layer.content
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
) -> Task<Message> {
    w.layer.stroke_width = s.clone();
    if let Ok(width) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let LayerContent::Shape {
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
        pub(crate) fn $fn_name(w: &mut MainWindow, s: String) -> Task<Message> {
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

pub(crate) fn layer_drag_started(w: &mut MainWindow, idx: usize) -> Task<Message> {
    w.layer.selected_index = Some(idx);
    w.load_layer_for_editing();
    Task::none()
}

pub(crate) fn layer_dragged(w: &mut MainWindow, new_pos: Point) -> Task<Message> {
    let Some(idx) = w.layer.selected_index else {
        return Task::none();
    };
    if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && !layer.locked
    {
        layer.position_x = new_pos.x.clamp(0.0, 1.0);
        layer.position_y = new_pos.y.clamp(0.0, 1.0);
        w.layer.pos_x = format!("{:.3}", new_pos.x);
        w.layer.pos_y = format!("{:.3}", new_pos.y);
    }
    Task::none()
}

pub(crate) fn layer_drag_ended(w: &mut MainWindow) -> Task<Message> {
    save_current_slide(w);
    Task::none()
}
