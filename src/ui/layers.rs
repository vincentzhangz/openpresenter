//! Layer (object) inspector logic: add/select/reorder/delete layers,
//! visibility and lock toggles, per-layer property edits (opacity, geometry,
//! text, colour channels), and the layer field loader.
//!
//! Ported from the iced-era `ui/layers/mod.rs` plus the
//! `MainWindow::load_layer_for_editing` field loader. The `LayerDrag*` /
//! `LayerResized` messages are emitted by the editor canvas.

use crate::domain::{Object, ObjectContent, ShapeType};
use crate::ui::main_window::MainWindow;
use gpui_kit::{Context, Pixels, Point};

/// Messages owned by the Layers editor feature. The root
/// [`crate::ui::messages::Message`] wraps them in `Message::Layers(..)`.
#[derive(Debug, Clone)]
pub enum Message {
    AddTextLayer,
    AddShapeLayer(ShapeType),
    SelectLayer(Option<usize>),
    DeleteLayer(usize),
    DuplicateLayer(usize),
    MoveLayerUp(usize),
    MoveLayerDown(usize),
    ToggleSelectedLayerVisibility,
    ToggleLayerVisibility(usize),
    ToggleSelectedLayerLock,
    ToggleLayerLock(usize),
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
    SelectedLayerShapeCornerRadiusChanged(String),
    SelectedLayerShapeStrokeR(u8),
    SelectedLayerShapeStrokeG(u8),
    SelectedLayerShapeStrokeB(u8),
    SelectedLayerShapeStrokeA(u8),
    SelectedLayerShapeTypeChanged(ShapeType),
    SelectedLayerPositionXChanged(String),
    SelectedLayerPositionYChanged(String),
    SelectedLayerWidthChanged(String),
    SelectedLayerHeightChanged(String),
    LayerDragStarted(usize),
    LayerDragged(Point<Pixels>),
    LayerResized {
        position: Point<Pixels>,
        width: f32,
        height: f32,
    },
    LayerDragEnded,
}

/// Dispatch a layer message.
pub(crate) fn update(w: &mut MainWindow, msg: Message, cx: &mut Context<MainWindow>) {
    match msg {
        Message::AddTextLayer => add_text_layer(w, cx),
        Message::AddShapeLayer(s) => add_shape_layer(w, s, cx),
        Message::SelectLayer(i) => select_layer(w, i, cx),
        Message::DeleteLayer(idx) => delete_layer(w, idx, cx),
        Message::DuplicateLayer(idx) => duplicate_layer(w, idx, cx),
        Message::MoveLayerUp(idx) => move_layer_up(w, idx, cx),
        Message::MoveLayerDown(idx) => move_layer_down(w, idx, cx),
        Message::ToggleSelectedLayerVisibility => toggle_selected_layer_visibility(w, cx),
        Message::ToggleLayerVisibility(idx) => toggle_layer_visibility(w, idx, cx),
        Message::ToggleSelectedLayerLock => toggle_selected_layer_lock(w, cx),
        Message::ToggleLayerLock(idx) => toggle_layer_lock(w, idx, cx),
        Message::SelectedLayerOpacityChanged(v) => selected_layer_opacity_changed(w, v, cx),
        Message::SelectedLayerTextChanged(t) => selected_layer_text_changed(w, t, cx),
        Message::SelectedLayerFontSizeChanged(s) => selected_layer_font_size_changed(w, s, cx),
        Message::SelectedLayerTextColorR(v) => selected_layer_text_color_r(w, v, cx),
        Message::SelectedLayerTextColorG(v) => selected_layer_text_color_g(w, v, cx),
        Message::SelectedLayerTextColorB(v) => selected_layer_text_color_b(w, v, cx),
        Message::SelectedLayerTextShadowToggled => selected_layer_text_shadow_toggled(w, cx),
        Message::SelectedLayerTextOutlineToggled => selected_layer_text_outline_toggled(w, cx),
        Message::SelectedLayerTextBoldToggled => selected_layer_text_bold_toggled(w, cx),
        Message::SelectedLayerTextItalicToggled => selected_layer_text_italic_toggled(w, cx),
        Message::SelectedLayerShapeFillR(v) => selected_layer_shape_fill_r(w, v, cx),
        Message::SelectedLayerShapeFillG(v) => selected_layer_shape_fill_g(w, v, cx),
        Message::SelectedLayerShapeFillB(v) => selected_layer_shape_fill_b(w, v, cx),
        Message::SelectedLayerShapeFillA(v) => selected_layer_shape_fill_a(w, v, cx),
        Message::SelectedLayerShapeStrokeWidthChanged(s) => {
            selected_layer_shape_stroke_width_changed(w, s, cx)
        }
        Message::SelectedLayerShapeCornerRadiusChanged(s) => {
            selected_layer_shape_corner_radius_changed(w, s, cx)
        }
        Message::SelectedLayerShapeStrokeR(v) => selected_layer_shape_stroke_r(w, v, cx),
        Message::SelectedLayerShapeStrokeG(v) => selected_layer_shape_stroke_g(w, v, cx),
        Message::SelectedLayerShapeStrokeB(v) => selected_layer_shape_stroke_b(w, v, cx),
        Message::SelectedLayerShapeStrokeA(v) => selected_layer_shape_stroke_a(w, v, cx),
        Message::SelectedLayerShapeTypeChanged(s) => selected_layer_shape_type_changed(w, s, cx),
        Message::SelectedLayerPositionXChanged(s) => selected_layer_position_x_changed(w, s, cx),
        Message::SelectedLayerPositionYChanged(s) => selected_layer_position_y_changed(w, s, cx),
        Message::SelectedLayerWidthChanged(s) => selected_layer_width_changed(w, s, cx),
        Message::SelectedLayerHeightChanged(s) => selected_layer_height_changed(w, s, cx),
        Message::LayerDragStarted(i) => layer_drag_started(w, i, cx),
        Message::LayerDragged(p) => layer_dragged(w, p, cx),
        Message::LayerResized {
            position,
            width,
            height,
        } => layer_resized(w, position, width, height, cx),
        Message::LayerDragEnded => layer_drag_ended(w, cx),
    }
}

/// Return a mutable reference to the currently selected layer of the slide
/// being edited, if any.
pub(crate) fn selected_layer_mut(w: &mut MainWindow) -> Option<&mut Object> {
    let idx = w.layer.selected_index?;
    w.get_current_slide_mut()?.layers.get_mut(idx)
}

/// Persist the slide being edited through the presentation service.
pub(crate) fn save_current_slide(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if let Some(slide) = w.get_current_slide() {
        let clone = slide.clone();
        w.persist_slide(clone, cx);
    }
}

/// Load the per-layer editing fields from the selected layer. Call whenever
/// the layer selection or the underlying slide changes; clears the selection
/// when it no longer points at a layer.
pub(crate) fn load_layer_for_editing(w: &mut MainWindow) {
    let Some(idx) = w.layer.selected_index else {
        // Nothing is selected: the Text tab still edits the slide's text, so
        // the mirror follows it. A mirror that still held another object's
        // text would put that text over the slide's own.
        let mirror = w
            .get_current_slide()
            .and_then(|slide| slide.editable_text(None))
            .map(|(text, style)| (text.clone(), style.font_size.to_string()));
        if let Some((text, font_size)) = mirror {
            w.editor.editing_slide_text = text;
            w.editor.editing_slide_font_size = font_size;
        }
        return;
    };
    #[allow(clippy::type_complexity)]
    let data: Option<(
        String,
        String,
        f32,
        f32,
        f32,
        f32,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    )> = w.get_current_slide().and_then(|slide| {
        let layer = slide.layers.get(idx)?;
        let (
            text,
            font_size,
            font_family,
            line_height,
            letter_spacing,
            glow_radius,
            text_stroke_width,
        ) = if let ObjectContent::Text {
            ref text,
            ref style,
            ..
        } = layer.content
        {
            (
                text.clone(),
                style.font_size.to_string(),
                style.font_family.clone(),
                format!("{:.2}", style.line_height_multiplier),
                format!("{:.1}", style.letter_spacing),
                format!("{:.1}", style.glow_radius),
                format!("{:.1}", style.text_stroke_width),
            )
        } else {
            // A shape or an image is selected. The Text tab still edits the
            // slide's text, so the mirror keeps the text the slide really
            // has. A blank mirror would be written back over the slide's
            // own text by "Save Slide" and wipe it.
            let (text, font_size) = match slide.editable_text(Some(idx)) {
                Some((text, style)) => (text.clone(), style.font_size.to_string()),
                None => (String::new(), String::from("72")),
            };
            (
                text,
                font_size,
                String::from("Arial"),
                String::from("1.2"),
                String::from("0"),
                String::from("8"),
                String::from("0"),
            )
        };
        let (stroke_w, radius) = match layer.content {
            ObjectContent::Shape {
                stroke_width,
                corner_radius,
                ..
            } => (format!("{stroke_width}"), format!("{corner_radius}")),
            _ => (String::from("0"), String::from("0")),
        };
        Some((
            text,
            font_size,
            layer.position_x,
            layer.position_y,
            layer.width,
            layer.height,
            stroke_w,
            radius,
            font_family,
            line_height,
            letter_spacing,
            glow_radius,
            text_stroke_width,
        ))
    });
    if let Some((text, font_size, px, py, width, height, sw, radius, ff, lh, ls, gr, tsw)) = data {
        w.layer.text = text.clone();
        w.layer.font_size = font_size.clone();
        w.editor.editing_slide_text = text;
        w.editor.editing_slide_font_size = font_size;
        w.layer.pos_x = format!("{px:.3}");
        w.layer.pos_y = format!("{py:.3}");
        w.layer.width = format!("{width:.3}");
        w.layer.height = format!("{height:.3}");
        w.layer.stroke_width = sw;
        w.layer.corner_radius = radius;
        w.layer.font_family = ff;
        w.layer.line_height = lh;
        w.layer.letter_spacing = ls;
        w.layer.glow_radius = gr;
        w.layer.text_stroke_width = tsw;
    } else {
        w.layer.selected_index = None;
    }
}

pub(crate) fn add_text_layer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_objects();
        let layer = Object::new_text(String::new());
        let new_idx = slide.add_object_on_top(layer);
        save_current_slide(w, cx);
        w.layer.selected_index = Some(new_idx);
        load_layer_for_editing(w);
    }
    cx.notify();
}

pub(crate) fn add_shape_layer(w: &mut MainWindow, shape: ShapeType, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.migrate_to_objects();
        let layer = Object::new_shape(shape);
        let new_idx = slide.add_object_on_top(layer);
        save_current_slide(w, cx);
        w.layer.selected_index = Some(new_idx);
        load_layer_for_editing(w);
    }
    cx.notify();
}

pub(crate) fn select_layer(w: &mut MainWindow, idx: Option<usize>, cx: &mut Context<MainWindow>) {
    // Selecting another object ends the inline edit: the editor belongs to
    // the object that was being edited, and leaving it open over the old
    // object would have the operator typing into the wrong place.
    // A single selection replaces the group, so the two never disagree.
    w.layer.selected_indices.clear();
    // A legacy slide is converted to objects the first time it is selected.
    // That conversion is an edit, so it can be undone.
    let legacy = idx.is_some()
        && w.get_current_slide()
            .is_some_and(|slide| slide.layers.is_empty());
    if legacy {
        w.push_undo();
    }

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
        save_current_slide(w, cx);
    }
    w.layer.selected_index = valid_idx;
    // Selecting another object ends the inline edit: the editor belongs to
    // the object that was being edited, and leaving it open over the old
    // object would have the operator typing into the wrong place.
    if w.canvas.editing != valid_idx {
        crate::ui::canvas::end_text_edit(w, cx);
    }
    load_layer_for_editing(w);
    if let Some(i) = valid_idx
        && let Some(slide) = w.get_current_slide()
        && let Some(layer) = slide.layers.get(i)
        && matches!(layer.content, ObjectContent::Text { .. })
    {
        w.shell.inspector_tab = crate::ui::messages::InspectorTab::Text;
    }
    cx.notify();
}

/// Select every object on the slide.
///
/// The commands that arrange a slide act on a group of objects, so the
/// operator has to be able to pick a whole slide: "align left" on a title
/// and a picture lines them up with each other, not with the slide.
pub(crate) fn select_all_layers(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    // A legacy slide is converted to objects the first time it is selected.
    let legacy = w
        .get_current_slide()
        .is_some_and(|slide| slide.layers.is_empty());
    if legacy {
        let migrated = w
            .get_current_slide_mut()
            .is_some_and(|slide| slide.migrate_to_objects());
        if !migrated {
            return;
        }
        w.push_undo();
        save_current_slide(w, cx);
    }
    let count = w.get_current_slide().map(|s| s.layers.len()).unwrap_or(0);
    if count == 0 {
        return;
    }
    w.layer.selected_indices = (0..count).collect();
    // The first object is the one the panels show, so the inspector still
    // describes something the operator can see.
    w.layer.selected_index = Some(0);
    cx.notify();
}

/// Leave every object alone.
///
/// Deselecting puts the editor back to the slide itself: with nothing
/// selected the Text tab edits the slide's own text, which is what the
/// operator means by "nothing is selected".
pub(crate) fn deselect_all_layers(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.layer.selected_index = None;
    w.layer.selected_indices.clear();
    load_layer_for_editing(w);
    cx.notify();
}

pub(crate) fn delete_selected_layer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(idx) = w.layer.selected_index else {
        return;
    };
    delete_layer(w, idx, cx);
}

pub(crate) fn delete_layer(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && idx < slide.layers.len()
    {
        slide.layers.remove(idx);
        // The depths are renumbered so they keep following the list: the
        // Layers panel's arrows move a row by its place in the list.
        slide.renumber_z_order();
        if slide.layers.is_empty() {
            // The slide's legacy content would bring the deleted object
            // back: a slide with no objects falls back to the content it
            // carried before the objects were moved into it.
            slide.content = crate::domain::SlideContent::Text {
                text: String::new(),
                style: crate::domain::TextStyle::default(),
            };
        }
        let clamp = if slide.layers.is_empty() {
            None
        } else {
            Some(idx.saturating_sub(1))
        };
        save_current_slide(w, cx);
        w.layer.selected_index = clamp;
    }
    load_layer_for_editing(w);
    cx.notify();
}

/// Duplicate a layer, placing the copy directly above it.
pub(crate) fn duplicate_layer(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    w.push_undo();
    let mut selected = None;
    if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get(idx).cloned()
    {
        // The copy is nudged so it does not hide behind the original.
        let mut copy = layer;
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.position_x = (copy.position_x + 0.02).min(1.0);
        copy.position_y = (copy.position_y + 0.02).min(1.0);
        let at = idx + 1;
        slide.layers.insert(at, copy);
        // The depths are renumbered so the list order and the draw order
        // agree: the Layers panel's up and down arrows move a row by its
        // place in the list, and a depth that does not follow the list
        // would move the wrong row.
        slide.renumber_z_order();
        selected = Some(at);
        save_current_slide(w, cx);
    }
    if let Some(at) = selected {
        w.layer.selected_index = Some(at);
    }
    load_layer_for_editing(w);
    cx.notify();
}

/// Duplicate the selected layer.
pub(crate) fn duplicate_selected_layer(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(idx) = w.layer.selected_index else {
        return;
    };
    duplicate_layer(w, idx, cx);
}

/// Move the selected object by a number of slide pixels.
///
/// The step is a step on the *slide*, not on the screen: an object that moves
/// one pixel on a 4K projector has to move one pixel on a 720p one too, so
/// the step is measured against the resolution the slide was authored at.
pub(crate) fn nudge_selected_layer(
    w: &mut MainWindow,
    dx: f32,
    dy: f32,
    cx: &mut Context<MainWindow>,
) {
    let Some(idx) = w.layer.selected_index else {
        return;
    };
    // The two axes are measured against their own side: the vertical
    // position is a fraction of the slide's height.
    let reference_width = w.app_config.output.width as f32;
    let reference_height = w.app_config.output.height as f32;
    if reference_width <= 0.0 || reference_height <= 0.0 {
        return;
    }
    // A run of nudges is one edit, not one edit per press.
    w.push_undo_coalesced();
    let updated = if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && layer.nudge(dx, dy, reference_width, reference_height)
    {
        Some((layer.position_x, layer.position_y))
    } else {
        None
    };
    if let Some((x, y)) = updated {
        // The geometry panel shows the position, so it follows the nudge.
        w.layer.pos_x = format!("{x:.3}");
        w.layer.pos_y = format!("{y:.3}");
        save_current_slide(w, cx);
        cx.notify();
    }
}

pub(crate) fn move_layer_up(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    // The front-most layer has nothing in front of it, so the press changes
    // nothing — and an undo entry that restores what is already on the
    // screen swallows the next undo.
    let Some(slide) = w.get_current_slide() else {
        return;
    };
    if idx + 1 >= slide.layers.len() {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut() {
        slide.layers.swap(idx, idx + 1);
        slide.renumber_z_order();
        save_current_slide(w, cx);
        w.layer.selected_index = Some(idx + 1);
    }
    cx.notify();
}

pub(crate) fn move_layer_down(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    // The back-most layer is already at the back: nothing moves.
    if idx == 0 {
        return;
    }
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && idx < slide.layers.len()
    {
        slide.layers.swap(idx, idx - 1);
        slide.renumber_z_order();
        save_current_slide(w, cx);
        w.layer.selected_index = Some(idx - 1);
    }
    cx.notify();
}

pub(crate) fn toggle_selected_layer_visibility(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w) {
        layer.visible = !layer.visible;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn toggle_layer_visibility(
    w: &mut MainWindow,
    idx: usize,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
    {
        layer.visible = !layer.visible;
        save_current_slide(w, cx);
    }
    cx.notify();
}

pub(crate) fn toggle_selected_layer_lock(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w) {
        layer.locked = !layer.locked;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn toggle_layer_lock(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    w.push_undo();
    if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
    {
        layer.locked = !layer.locked;
        save_current_slide(w, cx);
    }
    cx.notify();
}

pub(crate) fn selected_layer_opacity_changed(
    w: &mut MainWindow,
    v: f32,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    if let Some(layer) = selected_layer_mut(w) {
        layer.opacity = v.clamp(0.0, 1.0);
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn selected_layer_text_changed(
    w: &mut MainWindow,
    text: String,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    w.layer.text = text.clone();
    w.editor.editing_slide_text = text.clone();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text {
            text: ref mut t, ..
        } = layer.content
    {
        *t = text;
    }
    save_current_slide(w, cx);
    cx.notify();
}

pub(crate) fn selected_layer_font_size_changed(
    w: &mut MainWindow,
    s: String,
    cx: &mut Context<MainWindow>,
) {
    // The font size is an edit like any other, so it belongs in the history.
    w.push_undo_coalesced();
    w.layer.font_size = s.clone();
    // The Text tab shows the editor's mirror, so it has to follow the size.
    w.editor.editing_slide_font_size = s.clone();
    if let Ok(size) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Text { ref mut style, .. } = layer.content
    {
        // The size is clamped: a size of zero would draw nothing on the
        // projector while the editor still showed a hairline. The rule is
        // the domain's, so every panel clamps the same way.
        let size = crate::domain::TextStyle::clamp_font_size(size);
        style.font_size = size;
        save_current_slide(w, cx);
    }
    cx.notify();
}

macro_rules! text_color_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8, cx: &mut Context<MainWindow>) {
            w.push_undo_coalesced();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.color.$channel = v;
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

text_color_channel!(selected_layer_text_color_r, r);
text_color_channel!(selected_layer_text_color_g, g);
text_color_channel!(selected_layer_text_color_b, b);

macro_rules! text_toggle {
    ($fn_name:ident, $field:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
            w.push_undo();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Text { ref mut style, .. } = layer.content
            {
                style.$field = !style.$field;
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

text_toggle!(selected_layer_text_shadow_toggled, shadow);
text_toggle!(selected_layer_text_outline_toggled, outline);
text_toggle!(selected_layer_text_bold_toggled, bold);
text_toggle!(selected_layer_text_italic_toggled, italic);

macro_rules! shape_fill_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8, cx: &mut Context<MainWindow>) {
            w.push_undo_coalesced();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Shape { ref mut fill, .. } = layer.content
            {
                fill.$channel = v;
                save_current_slide(w, cx);
            }
            cx.notify();
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
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    w.layer.stroke_width = s.clone();
    if let Ok(width) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Shape {
            ref mut stroke_width,
            ..
        } = layer.content
    {
        *stroke_width = width;
        save_current_slide(w, cx);
    }
    cx.notify();
}

/// The corner radius of the selected shape.
///
/// The renderer draws the rounded corner, so the operator has to be able to
/// set it: a property the output honours and the editor cannot reach is a
/// property nobody can use.
pub(crate) fn selected_layer_shape_corner_radius_changed(
    w: &mut MainWindow,
    s: String,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo_coalesced();
    w.layer.corner_radius = s.clone();
    if let Ok(radius) = s.parse::<f32>()
        && let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Shape {
            ref mut corner_radius,
            ..
        } = layer.content
    {
        *corner_radius = radius.max(0.0);
        save_current_slide(w, cx);
    }
    cx.notify();
}

/// Change the kind of the selected shape.
///
/// A shape that cannot become another shape has to be deleted and drawn
/// again, and the new shape does not keep the old one's colour.
pub(crate) fn selected_layer_shape_type_changed(
    w: &mut MainWindow,
    shape: ShapeType,
    cx: &mut Context<MainWindow>,
) {
    w.push_undo();
    if let Some(layer) = selected_layer_mut(w)
        && let ObjectContent::Shape {
            ref mut shape_type, ..
        } = layer.content
    {
        *shape_type = shape;
        save_current_slide(w, cx);
    }
    cx.notify();
}

/// The stroke colour of the selected shape.
macro_rules! shape_stroke_channel {
    ($fn_name:ident, $channel:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, v: u8, cx: &mut Context<MainWindow>) {
            w.push_undo_coalesced();
            if let Some(layer) = selected_layer_mut(w)
                && let ObjectContent::Shape {
                    ref mut stroke_color,
                    ..
                } = layer.content
            {
                stroke_color.$channel = v;
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

shape_stroke_channel!(selected_layer_shape_stroke_r, r);
shape_stroke_channel!(selected_layer_shape_stroke_g, g);
shape_stroke_channel!(selected_layer_shape_stroke_b, b);
shape_stroke_channel!(selected_layer_shape_stroke_a, a);

macro_rules! layer_geom_field {
    ($fn_name:ident, $edit_field:ident, $layer_field:ident) => {
        pub(crate) fn $fn_name(w: &mut MainWindow, s: String, cx: &mut Context<MainWindow>) {
            // The geometry is an edit like any other, so it belongs in the
            // history: without this, Ctrl+Z could not undo a move.
            w.push_undo_coalesced();
            if let Ok(v) = s.parse::<f32>()
                && let Some(layer) = selected_layer_mut(w)
            {
                let value = v.clamp(0.0, 1.0);
                layer.$layer_field = value;
                // The field shows the value the model holds, not what was
                // typed: a value that was clamped would otherwise be
                // written again the next time the field changed.
                w.layer.$edit_field = value.to_string();
                save_current_slide(w, cx);
            }
            cx.notify();
        }
    };
}

layer_geom_field!(selected_layer_position_x_changed, pos_x, position_x);
layer_geom_field!(selected_layer_position_y_changed, pos_y, position_y);
layer_geom_field!(selected_layer_width_changed, width, width);
layer_geom_field!(selected_layer_height_changed, height, height);

pub(crate) fn layer_drag_started(w: &mut MainWindow, idx: usize, cx: &mut Context<MainWindow>) {
    w.push_undo();
    select_layer(w, Some(idx), cx);
}

pub(crate) fn layer_dragged(
    w: &mut MainWindow,
    new_pos: Point<Pixels>,
    cx: &mut Context<MainWindow>,
) {
    let Some(idx) = w.layer.selected_index else {
        return;
    };
    // Canvas-driven drag: the new position arrives in normalised 0..1
    // coordinates.
    let new_x = f32::from(new_pos.x).clamp(0.0, 1.0);
    let new_y = f32::from(new_pos.y).clamp(0.0, 1.0);
    let updated = if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && !layer.locked
    {
        layer.position_x = new_x;
        layer.position_y = new_y;
        Some((layer.position_x, layer.position_y))
    } else {
        None
    };
    if let Some((x, y)) = updated {
        w.layer.pos_x = format!("{x:.3}");
        w.layer.pos_y = format!("{y:.3}");
        cx.notify();
    }
}

pub(crate) fn layer_resized(
    w: &mut MainWindow,
    position: Point<Pixels>,
    width: f32,
    height: f32,
    cx: &mut Context<MainWindow>,
) {
    let Some(idx) = w.layer.selected_index else {
        return;
    };
    let new_x = f32::from(position.x).clamp(0.0, 1.0);
    let new_y = f32::from(position.y).clamp(0.0, 1.0);
    let mut font_size = None;
    let updated = if let Some(slide) = w.get_current_slide_mut()
        && let Some(layer) = slide.layers.get_mut(idx)
        && !layer.locked
    {
        let old_width = layer.width;
        layer.position_x = new_x;
        layer.position_y = new_y;
        layer.width = width.clamp(0.02, 1.0);
        layer.height = height.clamp(0.02, 1.0);

        // A text object is drawn at its font size, not at the size of its
        // box, so resizing the box scales the text — otherwise the handles
        // would change nothing the audience can see.
        if let ObjectContent::Text { style, .. } = &mut layer.content
            && old_width > 0.0
        {
            let scale = layer.width / old_width;
            style.font_size = crate::domain::TextStyle::clamp_font_size(style.font_size * scale);
            // The inspector shows the font size, so it follows the resize.
            font_size = Some(style.font_size);
        }

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
        w.layer.pos_x = format!("{x:.3}");
        w.layer.pos_y = format!("{y:.3}");
        w.layer.width = format!("{width:.3}");
        w.layer.height = format!("{height:.3}");
        if let Some(font_size) = font_size {
            let font_size = format!("{font_size:.1}");
            w.layer.font_size = font_size.clone();
            // The Text tab shows the editor's mirror, so it has to follow the
            // resize. A mirror that still held the old size would put that
            // size back the next time the slide was saved.
            w.editor.editing_slide_font_size = font_size;
        }
        cx.notify();
    }
}

pub(crate) fn layer_drag_ended(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    save_current_slide(w, cx);
    cx.notify();
}
