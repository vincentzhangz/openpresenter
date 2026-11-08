//! Native GPUI slide canvas.
//!
//! Renders a domain [`Slide`] as a live GPUI element tree instead of
//! rasterising it. Every object becomes a positioned element, so text stays
//! crisp at any size, images reuse GPUI's asset cache, and the editor can
//! attach mouse handlers straight to the object boxes.
//!
//! The audience-facing surfaces (dock preview, stage display, output window)
//! deliberately do **not** use this module: they show the composited frame
//! produced by the shared pipeline in [`crate::render`], so what the operator
//! sees is exactly what the NDI sender and the recorder produce. This module
//! is for authoring and for thumbnails, where crispness and interactivity
//! matter more than pixel parity with the output pipeline.
//!
//! Known difference from the output pipeline: per-run text styling (the
//! domain's `TextRun` list) is not applied here — the whole text object uses
//! its base [`TextStyle`]. The output pipeline does honour runs.
//!
//! Known difference from the output pipeline: ligatures cannot be switched
//! off here. GPUI does not expose the shaping mode, so a text set to "Use
//! None" still shows the font's own joined letters in the editor, while the
//! projector draws the letters on their own.
//!
//! Known difference from the output pipeline: small caps need a per-letter
//! size, and GPUI measures a text element as a whole — so a text styled in
//! parts is shown in one size rather than in small capitals.

use std::sync::Arc;

use gpui_kit::Focusable;
use gpui_kit::TestSupportExt;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, BoxShadow, Context, Div, Entity, FontWeight, Hsla, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ObjectFit, Pixels, Point, RenderImage, Rgba, SharedString,
    Size, StrikethroughStyle, TextAlign, UnderlineStyle, div, hsla, img, point, px, svg,
};

use crate::domain::{
    Background, Color, ImageFit, Object, ObjectContent, ShapeType, Slide, TextAlignment, TextRun,
    TextStyle,
};
use crate::ui::layers;
use crate::ui::main_window::MainWindow;
use crate::ui::messages::Message;

/// How a canvas surface behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasMode {
    /// Read-only rendering, used for thumbnails.
    Preview,
    /// Interactive editing: click to select, drag to move, corner handles to
    /// resize.
    Edit,
}

/// Which corner handle of the selected object is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// One in-progress canvas interaction.
///
/// The drag is tracked in window pixels and converted to the slide's
/// normalised coordinates when the object is moved or resized, so the
/// behaviour does not depend on where the canvas sits in the window.
pub(crate) struct DragState {
    /// The object being dragged.
    pub(crate) index: usize,
    /// The corner handle being dragged; `None` means the object is moved.
    pub(crate) handle: Option<ResizeHandle>,
    /// Pointer position in window pixels when the drag started.
    pub(crate) start_pointer: Point<Pixels>,
    /// The object's centre in normalised slide coordinates at drag start.
    pub(crate) start_center: Point<f32>,
    /// The object's size in normalised slide coordinates at drag start.
    pub(crate) start_size: (f32, f32),
    /// Whether the object has actually moved.
    ///
    /// A click that only selects the object is not an edit, so it must not
    /// put an entry on the undo stack: the operator would press Ctrl+Z and
    /// nothing would happen, again and again.
    pub(crate) moved: bool,
}

/// Start editing a text object on the canvas.
///
/// The text is edited in place, the way a presentation editor does it: a
/// double click on the object opens an input over it.
pub(crate) fn begin_text_edit(
    w: &mut MainWindow,
    index: usize,
    window: &mut gpui_kit::Window,
    cx: &mut Context<MainWindow>,
) {
    let Some(slide) = w.get_current_slide() else {
        return;
    };
    let Some(object) = slide.layers.get(index) else {
        return;
    };
    let crate::domain::ObjectContent::Text { text, .. } = &object.content else {
        return;
    };
    let text = text.clone();

    w.canvas.editing = Some(index);
    let input = w.canvas_text_input.clone();
    let focus = input.read(cx).focus_handle(cx);
    input.update(cx, |state, input_cx| {
        state.set_value(&text, window, input_cx);
    });
    window.focus(&focus, cx);
    cx.notify();
}

/// The text of the object being edited changed.
pub(crate) fn text_edited(w: &mut MainWindow, value: String, cx: &mut Context<MainWindow>) {
    let Some(index) = w.canvas.editing else {
        return;
    };
    let edited = {
        let Some(slide) = w.get_current_slide_mut() else {
            return;
        };
        let Some(layer) = slide.layers.get_mut(index) else {
            return;
        };
        let crate::domain::ObjectContent::Text {
            text: ref mut layer_text,
            ..
        } = layer.content
        else {
            return;
        };
        *layer_text = value.clone();
        true
    };
    if edited {
        // The Text tab shows the editor's mirror, so it has to follow the
        // inline edit. A mirror that still held the old text would put that
        // text back the next time the slide was saved.
        w.editor.editing_slide_text = value.clone();
        w.layer.text = value;
        // The edit is saved as it is typed: the inline editor is only closed
        // by Escape or by another click, and a show that is closed in between
        // must not lose the words.
        crate::ui::layers::save_current_slide(w, cx);
    }
    cx.notify();
}

/// Copy the selected object.
pub(crate) fn copy_object(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(index) = w.layer.selected_index else {
        return;
    };
    let Some(object) = w
        .get_current_slide()
        .and_then(|slide| slide.layers.get(index))
        .cloned()
    else {
        return;
    };
    w.canvas.copied = Some(object);
    cx.notify();
}

/// Paste the copied object onto the slide.
///
/// The copy is nudged, so it does not hide behind the original.
pub(crate) fn paste_object(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    let Some(mut object) = w.canvas.copied.clone() else {
        return;
    };
    w.push_undo();
    let index = {
        let Some(slide) = w.get_current_slide_mut() else {
            return;
        };
        object.id = uuid::Uuid::new_v4().to_string();
        object.position_x = (object.position_x + 0.02).min(1.0);
        object.position_y = (object.position_y + 0.02).min(1.0);
        // The paste goes on top: the picture is drawn back to front, so a
        // copy that keeps the original's depth can hide behind it.
        slide.add_object_on_top(object)
    };
    w.layer.selected_index = Some(index);
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
}

/// Stop editing the text on the canvas.
pub(crate) fn end_text_edit(w: &mut MainWindow, cx: &mut Context<MainWindow>) {
    if w.canvas.editing.take().is_none() {
        return;
    }
    crate::ui::layers::save_current_slide(w, cx);
    cx.notify();
}

/// The size of a resize handle, in logical pixels.
const HANDLE_SIZE: Pixels = px(9.);

/// Render a slide into a canvas element of `size` logical pixels.
///
/// `reference_width` is the output resolution the domain was authored
/// against: font sizes, corner radii and stroke widths are expressed in
/// slide pixels and are scaled down to the canvas.
///
/// In [`CanvasMode::Edit`] the objects can be selected, moved and resized;
/// the selected object is outlined and gets four corner handles.
#[allow(clippy::too_many_arguments)]
pub fn slide_element(
    slide: &Slide,
    size: Size<Pixels>,
    reference_width: f32,
    mode: CanvasMode,
    selected: Option<usize>,
    video_frame: Option<Arc<RenderImage>>,
    editing: Option<(usize, Entity<gpui_kit::component::input::InputState>)>,
    guides: CanvasGuides,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    slide_root(
        slide,
        size,
        reference_width,
        mode,
        selected,
        video_frame,
        editing,
        guides,
        Some(cx),
    )
}

/// Render a slide as a static picture.
///
/// The preview has no interaction and no dependency on the main window, so
/// it can be used anywhere a thumbnail is needed — the slide lists, the
/// stage monitor, the show-mode grid.
pub fn slide_preview(
    slide: &Slide,
    size: Size<Pixels>,
    reference_width: f32,
    video_frame: Option<Arc<RenderImage>>,
) -> AnyElement {
    slide_root(
        slide,
        size,
        reference_width,
        CanvasMode::Preview,
        None,
        video_frame,
        None,
        CanvasGuides::default(),
        None,
    )
}

/// Build the canvas element.
///
/// `cx` is `None` for a picture: without a context there is nothing to
/// attach the editing handlers to.
#[allow(clippy::too_many_arguments)]
fn slide_root(
    slide: &Slide,
    size: Size<Pixels>,
    reference_width: f32,
    mode: CanvasMode,
    selected: Option<usize>,
    video_frame: Option<Arc<RenderImage>>,
    editing: Option<(usize, Entity<gpui_kit::component::input::InputState>)>,
    guides: CanvasGuides,
    mut cx: Option<&mut Context<MainWindow>>,
) -> AnyElement {
    let scale = if reference_width > 0.0 {
        f32::from(size.width) / reference_width
    } else {
        1.0
    };

    // Objects are painted back to front by z-order, but every object keeps
    // its index in `slide.layers`: that index is what the layer selection and
    // the drag messages refer to.
    //
    // `effective_layers` also covers the legacy slides that carry their text
    // in `SlideContent` rather than in an object, so the canvas shows the
    // same thing the compositor draws.
    let layers = slide.effective_layers().into_owned();
    let mut objects: Vec<(usize, &Object)> = layers.iter().enumerate().collect();
    objects.sort_by_key(|(_, object)| object.z_order);

    let background = background_element(slide, video_frame.clone(), mode, cx.as_deref_mut());

    let mut object_elements: Vec<AnyElement> = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter() {
        // A hidden object is not on the slide: the compositor skips it, so
        // the canvas has to skip it too, or the editor would show an object
        // the audience never sees. The Layers tab is where it is shown
        // again.
        if !object.visible {
            continue;
        }
        let editing_input = match &editing {
            Some((editing_index, input)) if *editing_index == *index => Some(input.clone()),
            _ => None,
        };
        object_elements.push(object_element(
            object,
            *index,
            size,
            scale,
            mode,
            video_frame.clone(),
            editing_input,
            cx.as_deref_mut(),
        ));
    }

    // The selection overlay is a sibling of the objects, not a child: a handle
    // must not bubble its mouse-down into the object underneath, or a resize
    // would turn into a move.
    let overlay = match mode {
        CanvasMode::Edit => selection_overlay(slide, size, selected, cx.as_deref_mut()),
        CanvasMode::Preview => div(),
    };

    let canvas_width = f32::from(size.width);
    let canvas_height = f32::from(size.height);

    // The guides are drawn over the objects: they are the operator's marks
    // on the slide, not part of it.
    let guide_overlay = guides_overlay(size, guides);

    let root = div()
        .relative()
        .w(size.width)
        .h(size.height)
        .flex_shrink_0()
        .overflow_hidden()
        .bg(gpui_kit::rgb(0x000000))
        .child(background)
        .child(div().absolute().inset_0().children(object_elements))
        .child(overlay)
        .child(guide_overlay);

    if mode != CanvasMode::Edit {
        // A preview is a picture, not a surface: it must not swallow the
        // clicks of whatever it sits on.
        return root.into_any_element();
    }
    let Some(cx) = cx else {
        return root.into_any_element();
    };

    // The editing canvas carries an id, so it can be addressed: the editor
    // is checked against the surface the operator works on.
    let root = root.id("canvas-slide").test_support();

    root.on_mouse_move(cx.listener(move |w, e: &MouseMoveEvent, window, cx| {
        drag_moved(w, e, window, cx, canvas_width, canvas_height);
    }))
    .on_mouse_up(
        MouseButton::Left,
        cx.listener(|w, _, window, cx| {
            drag_ended(w, window, cx);
        }),
    )
    .on_mouse_up_out(
        MouseButton::Left,
        cx.listener(|w, _, window, cx| {
            drag_ended(w, window, cx);
        }),
    )
    .into_any_element()
}

/// The guides the canvas draws over the slide.
///
/// The guides are the editor's furniture: they are shown while a slide is
/// being edited and never on the stage or in a preview.
#[derive(Debug, Clone, Copy, Default)]
pub struct CanvasGuides {
    /// The slide's centre lines.
    pub slide_guides: bool,
    /// The theme's safe areas.
    pub theme_guides: bool,
    /// Whether the rulers are shown.
    pub show_rulers: bool,
}

/// The guides drawn over the slide.
///
/// The guides are the editor's furniture: they show where the middle of the
/// slide is and where the safe areas are, so an operator can put a title
/// where it will not be cut off. They are drawn over the slide and never
/// take a click — a guide the operator cannot click through is a guide the
/// operator works around.
fn guides_overlay(size: Size<Pixels>, guides: CanvasGuides) -> impl IntoElement {
    let slide_guides = guides.slide_guides;
    let theme_guides = guides.theme_guides;
    let show_rulers = guides.show_rulers;
    let mut overlay = div().absolute().inset_0();

    if show_rulers {
        // The ruler measures the slide in slide pixels, so it is the
        // operator's way of putting an object at a known place.
        let ruler_width = f32::from(size.width);
        let tick_colour = crate::ui::theme::c(crate::ui::theme::TEXT_MUTED).opacity(0.6);
        let mut ruler = div()
            .id("canvas-ruler-top")
            // The ruler is observed, so a test can read that it is on the
            // screen.
            .test_support()
            .absolute()
            .left(px(0.))
            .top(px(0.))
            .w_full()
            .h(px(20.))
            .bg(crate::ui::theme::c(crate::ui::theme::BG_TOOLBAR).opacity(0.85));
        let mut x = 0.0f32;
        while x < ruler_width {
            ruler = ruler.child(
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(12.))
                    .w(px(1.))
                    .h(px(8.))
                    .bg(tick_colour),
            );
            x += 50.0;
        }
        overlay = overlay.child(ruler);
    }

    if theme_guides {
        // The safe areas: the title area is the inner nine tenths and the
        // action area the inner thirteen thirteenths, so a title that is
        // inside them survives a screen that crops the edges.
        for (inset, colour) in [(0.05f32, 0xF03708u32), (0.035f32, 0x6E8BE8u32)] {
            let left = f32::from(size.width) * inset;
            let top = f32::from(size.height) * inset;
            overlay = overlay.child(
                div()
                    .id("canvas-guide-theme")
                    // The guide is observed, so a test can read that it is
                    // on the screen.
                    .test_support()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(f32::from(size.width) - left * 2.0))
                    .h(px(f32::from(size.height) - top * 2.0))
                    .border_1()
                    .border_color(crate::ui::theme::c(colour).opacity(0.5)),
            );
        }
    }

    if slide_guides {
        // The centre lines: the middle of the slide, where a title sits
        // when it is centred.
        let centre_x = f32::from(size.width) / 2.0;
        let centre_y = f32::from(size.height) / 2.0;
        let colour = crate::ui::theme::c(crate::ui::theme::ACCENT_ORANGE).opacity(0.45);
        overlay = overlay
            .child(
                div()
                    .id("canvas-guide-slide")
                    // The guide is observed, so a test can read that it is
                    // on the screen.
                    .test_support()
                    .absolute()
                    .left(px(centre_x))
                    .top(px(0.))
                    .w(px(1.))
                    .h_full()
                    .bg(colour),
            )
            .child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(centre_y))
                    .w_full()
                    .h(px(1.))
                    .bg(colour),
            );
    }

    overlay
}

/// The background layer: a solid colour, an image or the live video frame.
///
/// The background is also the click-away target: clicking empty canvas clears
/// the selection.
fn background_element(
    slide: &Slide,
    video_frame: Option<Arc<RenderImage>>,
    mode: CanvasMode,
    cx: Option<&mut Context<MainWindow>>,
) -> AnyElement {
    let fill = match &slide.background {
        Background::Solid(color) => div().absolute().inset_0().bg(to_hsla(*color)),
        Background::Image(path) if !path.is_empty() => div()
            .absolute()
            .inset_0()
            .child(img(path.clone()).object_fit(ObjectFit::Cover).size_full()),
        Background::Video(_) => match video_frame {
            Some(frame) => div()
                .absolute()
                .inset_0()
                .child(img(frame).object_fit(ObjectFit::Cover).size_full()),
            None => div().absolute().inset_0().bg(gpui_kit::rgb(0x000000)),
        },
        Background::Image(_) => div().absolute().inset_0().bg(gpui_kit::rgb(0x000000)),
    };

    if mode != CanvasMode::Edit {
        return fill.into_any_element();
    }
    let Some(cx) = cx else {
        return fill.into_any_element();
    };

    // The background is the click-away target: it clears the selection when
    // the operator clicks empty canvas.
    fill.id("canvas-background")
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|w, _, window, cx| {
                w.handle_message(
                    Message::Layers(layers::Message::SelectLayer(None)),
                    window,
                    cx,
                );
            }),
        )
        .into_any_element()
}

/// Render one object of the slide.
#[allow(clippy::too_many_arguments)]
fn object_element(
    object: &Object,
    index: usize,
    size: Size<Pixels>,
    scale: f32,
    mode: CanvasMode,
    video_frame: Option<Arc<RenderImage>>,
    editing_input: Option<Entity<gpui_kit::component::input::InputState>>,
    cx: Option<&mut Context<MainWindow>>,
) -> AnyElement {
    let (left, top, width, height) = object_box(object, size);
    let position = Point {
        x: object.position_x,
        y: object.position_y,
    };
    // The layout of a text object, shared by the text and by the editor
    // that replaces it: the operator has to type over the words they are
    // replacing, so the input sits exactly where the text is.
    let text_layout = match &object.content {
        ObjectContent::Text { style, runs, .. } => {
            Some(text_layout(style, runs, position, size, scale, (left, top)))
        }
        _ => None,
    };
    let content = match &object.content {
        ObjectContent::Text { text, style, runs } => {
            text_object(text, style, runs, position, size, scale, (left, top))
        }
        ObjectContent::Shape { .. } => shape_object(object, width, height, scale),
        ObjectContent::Image { path, fit } => image_object(path, *fit, width, height),
        ObjectContent::Video { .. } => match video_frame {
            // The output pipeline stretches the video into the object's box,
            // so the editor shows it the same way.
            Some(frame) => div().child(img(frame).object_fit(ObjectFit::Fill).size_full()),
            None => div().bg(gpui_kit::rgb(0x111111)),
        },
    };

    // A text object that is being edited shows an input in its place, so the
    // operator types straight onto the slide.
    let content: AnyElement = match (&editing_input, &text_layout) {
        (Some(input), Some(layout)) => editing_input_element(input, layout),
        _ => content.into_any_element(),
    };

    let styled = div()
        .absolute()
        .left(left)
        .top(top)
        .w(width)
        .h(height)
        .opacity(object.opacity.clamp(0.0, 1.0))
        .child(content);

    if mode != CanvasMode::Edit {
        return styled.into_any_element();
    }
    let Some(cx) = cx else {
        return styled.into_any_element();
    };

    let start_center = Point {
        x: object.position_x,
        y: object.position_y,
    };
    let start_size = (object.width, object.height);

    styled
        .id(SharedString::from(format!("canvas-object-{index}")))
        // The object is observed, so a test can click it and read where it
        // is drawn: the editor is checked against the surface the operator
        // works on.
        .test_support()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |w, e: &MouseDownEvent, window, cx| {
                if e.button != MouseButton::Left {
                    return;
                }
                // The click belongs to this object. A mouse-down reaches
                // every object it overlaps, so without this the object
                // *underneath* would be the one that is selected and dragged.
                cx.stop_propagation();
                // A double click on a text object edits it in place, the
                // way a presentation editor does.
                if e.click_count >= 2 && is_text_object(w, index) {
                    w.handle_message(Message::CanvasBeginTextEdit(index), window, cx);
                    return;
                }
                // Read the live geometry: the slide may have been edited
                // since the element tree was built.
                let (center, size) = live_geometry(w, index).unwrap_or((start_center, start_size));
                w.canvas.drag = Some(DragState {
                    index,
                    handle: None,
                    start_pointer: e.position,
                    start_center: center,
                    start_size: size,
                    moved: false,
                });
                // Selecting is not an edit: the undo snapshot is taken when
                // the object actually moves.
                w.handle_message(
                    Message::Layers(layers::Message::SelectLayer(Some(index))),
                    window,
                    cx,
                );
            }),
        )
        .into_any_element()
}

/// Whether an object holds text that can be edited in place.
fn is_text_object(w: &MainWindow, index: usize) -> bool {
    w.get_current_slide()
        .and_then(|slide| slide.layers.get(index))
        .is_some_and(|object| matches!(object.content, ObjectContent::Text { .. }))
}

/// The current centre and size of an object, in normalised slide coordinates.
fn live_geometry(w: &MainWindow, index: usize) -> Option<(Point<f32>, (f32, f32))> {
    let object = w.get_current_slide()?.layers.get(index)?;
    Some((
        Point {
            x: object.position_x,
            y: object.position_y,
        },
        (object.width, object.height),
    ))
}

/// The pixel box of an object inside the canvas.
///
/// The domain stores an object's position as the centre of its box and its
/// size as a fraction of the slide, so the box is derived from both.
fn object_box(object: &Object, size: Size<Pixels>) -> (Pixels, Pixels, Pixels, Pixels) {
    let canvas_w = f32::from(size.width);
    let canvas_h = f32::from(size.height);
    let width = (object.width * canvas_w).max(1.0);
    let height = (object.height * canvas_h).max(1.0);
    // The box keeps its place: the object's centre is where the model says
    // it is, even when the box hangs over the edge of the slide. A box that
    // was pushed back onto the slide would draw the object somewhere else
    // than the inspector reports, and the projector would draw it there too.
    let left = object.position_x * canvas_w - width / 2.0;
    let top = object.position_y * canvas_h - height / 2.0;
    (px(left), px(top), px(width), px(height))
}

/// Where a text object's anchor sits inside its box.
///
/// The text is placed inside the object's box, so the anchor is measured
/// from the box's origin: the position is where the text is on the slide,
/// and the box is where the object is. Measuring the anchor from the canvas
/// instead would place the text at its position twice over, and every text
/// would end up in the corner of the slide.
pub(crate) fn text_anchor_in_box(
    position: Point<f32>,
    canvas: Size<Pixels>,
    origin: (Pixels, Pixels),
) -> Point<f32> {
    Point {
        x: position.x * f32::from(canvas.width) - f32::from(origin.0),
        y: position.y * f32::from(canvas.height) - f32::from(origin.1),
    }
}

/// The layout of a text object: where its anchor sits, how tall a line is
/// and how big the type is.
///
/// The size and the place follow the style's baseline: a superscript is
/// drawn smaller and above the line it belongs to, the way the output
/// pipeline draws it.
pub(crate) struct TextLayout {
    /// The anchor, measured from the object's box.
    pub(crate) anchor: Point<f32>,
    /// The height of one line.
    pub(crate) line_height: f32,
    /// The size of the type.
    pub(crate) font_size: f32,
}

/// Where a text object sits and how big it is.
///
/// The text is placed inside the object's box, so the anchor is measured
/// from the box's origin: the position is where the text is on the slide,
/// and the box is where the object is. Measuring the anchor from the canvas
/// instead would place the text at its position twice over, and every text
/// would end up in the corner of the slide.
pub(crate) fn text_layout(
    style: &TextStyle,
    runs: &[TextRun],
    position: Point<f32>,
    canvas: Size<Pixels>,
    scale: f32,
    origin: (Pixels, Pixels),
) -> TextLayout {
    // A run's own size is not applied to a styled text, because GPUI
    // measures a text element as a whole. A text with a single run is the
    // common case, though, and there the size can be applied — so the
    // editor shows the size the projector draws.
    let run_size = match runs {
        [run] => run.font_size.filter(|size| *size > 0.0),
        _ => None,
    };
    let font_size = (run_size.unwrap_or(style.font_size) * scale).max(1.0);
    // A superscript and a subscript are notes: they are drawn smaller, so
    // the size comes from the baseline and not from the font size.
    let font_size = (font_size * style.baseline.size_scale()).max(1.0);
    // A raised baseline moves the text up the slide and a lowered one moves
    // it down. The offset is a fraction of the text's own size, so it is
    // measured against the font size the style carries.
    let baseline_shift = -style.baseline.offset() * style.font_size * scale;
    let anchor = text_anchor_in_box(position, canvas, origin);
    TextLayout {
        // The anchor moves with the text: a raised text is drawn above the
        // line, and the editor's input has to sit over the words the
        // audience will see.
        anchor: Point {
            x: anchor.x,
            y: anchor.y + baseline_shift,
        },
        line_height: font_size * style.line_height_multiplier,
        font_size,
    }
}

/// The inline editor for a text object.
///
/// It sits exactly where the text is, at the text's size, so the operator
/// types over the words they are replacing.
fn editing_input_element(
    input: &Entity<gpui_kit::component::input::InputState>,
    layout: &TextLayout,
) -> AnyElement {
    div()
        .absolute()
        .left(px(layout.anchor.x))
        .top(px(layout.anchor.y - layout.line_height / 2.0))
        .w(px(layout.font_size * 20.0))
        .h(px(layout.line_height))
        .child(
            gpui_kit::component::input::Input::new(input)
                .text_size(px(layout.font_size))
                .w(px(layout.font_size * 20.0))
                .h(px(layout.line_height)),
        )
        .into_any_element()
}

/// A text object.
///
/// The text is anchored at the object's position: the alignment decides
/// which edge sits on the anchor, and the text is centred on it vertically.
/// This is the same rule the output pipeline uses, so the editor shows the
/// text where the audience will see it.
fn text_object(
    text: &str,
    style: &TextStyle,
    runs: &[TextRun],
    position: Point<f32>,
    canvas: Size<Pixels>,
    scale: f32,
    origin: (Pixels, Pixels),
) -> Div {
    // The layout is shared with the inline editor, so the text and the
    // input that replaces it are drawn at the same size and in the same
    // place.
    let layout = text_layout(style, runs, position, canvas, scale, origin);
    let font_size = layout.font_size;
    let line_height = layout.line_height;
    let content = style.text_transform.apply(text);

    let mut inner = div()
        .w_full()
        .text_size(px(font_size))
        .text_color(to_hsla(style.color))
        .line_height(px(line_height))
        .text_align(text_align(style.alignment))
        .font_weight(if style.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        });

    if !style.font_family.is_empty() && style.font_family != "Arial" {
        inner = inner.font_family(style.font_family.clone());
    }
    if style.italic {
        inner = inner.italic();
    }
    // The underline and the strikethrough are drawn from the letters' own
    // metrics, so they belong on the element that carries the text.
    if style.underline {
        inner = inner.underline();
    }
    if style.strikethrough {
        inner = inner.line_through();
    }

    // Shadows and glow are approximated with box shadows: GPUI's element
    // styling shadows the box, not the glyphs.
    let mut shadows: Vec<BoxShadow> = Vec::new();
    if style.shadow {
        shadows.push(BoxShadow {
            color: hsla(0., 0., 0., 0.55),
            offset: point(px(2. * scale), px(2. * scale)),
            blur_radius: px(4. * scale),
            spread_radius: px(0.),
            inset: false,
        });
    }
    if style.glow_enabled {
        shadows.push(BoxShadow {
            color: to_hsla(style.glow_color),
            offset: point(px(0.), px(0.)),
            blur_radius: px(style.glow_radius.max(0.0) * scale),
            spread_radius: px(0.),
            inset: false,
        });
    }
    if !shadows.is_empty() {
        inner = inner.shadow(shadows);
    }

    // The anchor is the point the alignment refers to: the left edge, the
    // centre or the right edge of the text. It follows the baseline, so a
    // raised text is drawn above the line it sits on.
    let canvas_w = f32::from(canvas.width);
    let anchor_x = layout.anchor.x;
    let anchor_y = layout.anchor.y;
    let (left, width) = match style.alignment {
        TextAlignment::Left => (anchor_x, (canvas_w - anchor_x).max(1.0)),
        TextAlignment::Center => (anchor_x - canvas_w / 2.0, canvas_w),
        TextAlignment::Right => (anchor_x - canvas_w, canvas_w),
    };

    // A text object may be styled in parts: the runs carry the pieces and
    // their own look, and the style carries the rest.
    let child: AnyElement = if !runs.is_empty() {
        let (text, gpu_runs) = build_runs(runs, style);
        inner
            .child(gpui_kit::StyledText::new(text).with_runs(gpu_runs))
            .into_any_element()
    } else if style.text_transform.is_small_caps() {
        // Small caps are capitals drawn smaller: the first letter keeps its
        // full size and the rest of the letters are drawn as small
        // capitals.
        inner
            .child(small_caps_text(
                &content,
                font_size,
                line_height,
                style.alignment,
            ))
            .into_any_element()
    } else {
        inner.child(content).into_any_element()
    };

    div()
        .absolute()
        .left(px(left))
        .top(px(anchor_y - line_height / 2.0))
        .w(px(width))
        .h(px(line_height))
        .flex()
        .items_center()
        .child(child)
}

/// The size a small capital is drawn at, as a fraction of the text's size.
///
/// The output pipeline draws the first letter of a small-caps text at its
/// full size and the rest of the letters as small capitals, so the editor
/// does the same.
const SMALL_CAPS_SCALE: f32 = 0.78;

/// The head and the tail of a text in small capitals.
///
/// The head is everything up to and including the first letter: it is drawn
/// at the text's own size, and the tail is drawn as small capitals. The
/// output pipeline splits the text the same way.
fn small_caps_split(text: &str) -> (&str, &str) {
    let head_end = text
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    text.split_at(head_end)
}

/// A text in small capitals.
///
/// GPUI measures a text element as a whole, so a per-letter size cannot be
/// set inside one element: the text is drawn as two elements in a row
/// instead — the first letter at its full size and the rest as small
/// capitals. The output pipeline draws the same thing with rich text.
fn small_caps_text(text: &str, font_size: f32, line_height: f32, alignment: TextAlignment) -> Div {
    let (head, tail) = small_caps_split(text);

    // The two parts stand on the same line: the small capitals are shorter,
    // so they are drawn from the bottom of the full-size letter up. The row
    // carries the alignment, because the letters are laid out in a row
    // rather than in a single text element.
    let mut row = div().flex().flex_row().items_end();
    row = match alignment {
        TextAlignment::Left => row.justify_start(),
        TextAlignment::Center => row.justify_center(),
        TextAlignment::Right => row.justify_end(),
    };
    if !head.is_empty() {
        row = row.child(
            div()
                .text_size(px(font_size))
                .line_height(px(line_height))
                .child(head.to_string()),
        );
    }
    if !tail.is_empty() {
        row = row.child(
            div()
                .text_size(px(font_size * SMALL_CAPS_SCALE))
                .line_height(px(line_height * SMALL_CAPS_SCALE))
                .child(tail.to_string()),
        );
    }
    row
}

/// Build the GPUI runs for a text object styled in parts.
///
/// The runs cover the text in order, so their lengths add up to the text
/// they are laid out over. A run's family, weight, style and colour are
/// applied; a run's own size is not, because GPUI measures a text element
/// as a whole — the output pipeline does honour it.
fn build_runs(runs: &[TextRun], style: &TextStyle) -> (String, Vec<gpui_kit::TextRun>) {
    let mut text = String::new();
    let mut gpu_runs = Vec::with_capacity(runs.len());

    // A run carries its own decoration: a styled text does not inherit the
    // underline or the strikethrough of the element around it.
    let underline = style.underline.then(|| UnderlineStyle {
        thickness: px(1.),
        ..Default::default()
    });
    let strikethrough = style.strikethrough.then(|| StrikethroughStyle {
        thickness: px(1.),
        ..Default::default()
    });

    for run in runs {
        let piece = style.text_transform.apply(&run.text);
        let family = run
            .font_family
            .clone()
            .filter(|family| !family.is_empty())
            .unwrap_or_else(|| style.font_family.clone());
        let weight = match run.bold.unwrap_or(style.bold) {
            true => gpui_kit::FontWeight::BOLD,
            false => gpui_kit::FontWeight::NORMAL,
        };
        let font_style = match run.italic.unwrap_or(style.italic) {
            true => gpui_kit::FontStyle::Italic,
            false => gpui_kit::FontStyle::Normal,
        };
        let color = run
            .color
            .map(to_hsla)
            .unwrap_or_else(|| to_hsla(style.color));

        gpu_runs.push(gpui_kit::TextRun {
            len: piece.len(),
            font: gpui_kit::Font {
                family: family.into(),
                features: Default::default(),
                fallbacks: None,
                weight,
                style: font_style,
            },
            color,
            background_color: None,
            underline,
            strikethrough,
        });
        text.push_str(&piece);
    }

    (text, gpu_runs)
}

/// A shape object.
fn shape_object(object: &Object, width: Pixels, height: Pixels, scale: f32) -> Div {
    let ObjectContent::Shape {
        shape_type,
        fill,
        stroke_color,
        stroke_width,
        corner_radius,
    } = &object.content
    else {
        return div();
    };

    let stroke = (stroke_width * scale).max(0.0);
    match shape_type {
        ShapeType::Rectangle | ShapeType::Line => apply_border(
            div()
                .w(width)
                .h(height)
                .bg(to_hsla(*fill))
                .rounded(px(corner_radius.max(0.0) * scale)),
            stroke,
            to_hsla(*stroke_color),
        ),
        ShapeType::Ellipse => apply_border(
            div().w(width).h(height).rounded_full().bg(to_hsla(*fill)),
            stroke,
            to_hsla(*stroke_color),
        ),
        ShapeType::Triangle => triangle_shape(*fill, *stroke_color, stroke, width, height),
    }
}

/// Apply a stroke to a shape.
///
/// GPUI's border widths come from a fixed scale, so the requested stroke is
/// rounded to the nearest available width.
fn apply_border(element: Div, stroke: f32, color: Hsla) -> Div {
    if stroke <= 0.0 {
        return element;
    }
    let element = match stroke {
        s if s < 1.5 => element.border_1(),
        s if s < 2.5 => element.border_2(),
        s if s < 3.5 => element.border_3(),
        s if s < 4.5 => element.border_4(),
        s if s < 6.5 => element.border_6(),
        s if s < 8.5 => element.border_8(),
        s if s < 12.5 => element.border_12(),
        _ => element.border_16(),
    };
    element.border_color(color)
}

/// A triangle drawn as an inline SVG, inscribed in the object's box.
fn triangle_shape(
    fill: Color,
    stroke: Color,
    stroke_width: f32,
    width: Pixels,
    height: Pixels,
) -> Div {
    let fill_hex = color_hex(fill);
    let stroke_hex = color_hex(stroke);
    let data = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" preserveAspectRatio="none"><polygon points="50,1 99,99 1,99" fill="{fill_hex}" stroke="{stroke_hex}" stroke-width="{stroke_width}"/></svg>"#
    );
    div()
        .w(width)
        .h(height)
        .child(svg().data(data.as_bytes()).size_full())
}

/// An image object.
///
/// The fit modes mean the same here as in the compositor, so the editor
/// shows the picture the audience will see: `Fit` keeps the whole image
/// inside the box, `Fill` covers the box and crops the overflow, `Stretch`
/// ignores the aspect ratio and `Center` keeps the image's own size.
fn image_object(path: &str, fit: ImageFit, width: Pixels, height: Pixels) -> Div {
    if path.is_empty() {
        return div().w(width).h(height);
    }
    let object_fit = match fit {
        ImageFit::Fit => ObjectFit::Contain,
        ImageFit::Fill => ObjectFit::Cover,
        ImageFit::Stretch => ObjectFit::Fill,
        // The image keeps its own size and is centred, and it is scaled down
        // when it is larger than the box — the same rule the output
        // pipeline uses.
        ImageFit::Center => ObjectFit::ScaleDown,
    };
    div()
        .w(width)
        .h(height)
        .child(img(path.to_string()).object_fit(object_fit).size_full())
}

/// The selection outline and its corner handles.
fn selection_overlay(
    slide: &Slide,
    size: Size<Pixels>,
    selected: Option<usize>,
    mut cx: Option<&mut Context<MainWindow>>,
) -> Div {
    let Some(index) = selected else {
        return div();
    };
    let Some(object) = slide.layers.get(index) else {
        return div();
    };
    if !object.visible {
        return div();
    }

    let (left, top, width, height) = object_box(object, size);

    let outline = div()
        .absolute()
        .left(left)
        .top(top)
        .w(width)
        .h(height)
        .border_1()
        .border_color(gpui_kit::rgb(0xF03708));

    let handles = [
        (ResizeHandle::TopLeft, left, top),
        (ResizeHandle::TopRight, left + width - HANDLE_SIZE, top),
        (ResizeHandle::BottomLeft, left, top + height - HANDLE_SIZE),
        (
            ResizeHandle::BottomRight,
            left + width - HANDLE_SIZE,
            top + height - HANDLE_SIZE,
        ),
    ];

    let mut handle_elements: Vec<AnyElement> = Vec::with_capacity(handles.len());
    for (handle, x, y) in handles {
        // A locked object cannot be moved or resized, so it gets no
        // handles: handles that silently do nothing are worse than none.
        if object.locked {
            break;
        }
        handle_elements.push(
            handle_element(handle, x, y, index, object, cx.as_deref_mut()).into_any_element(),
        );
    }

    div()
        .absolute()
        .inset_0()
        .child(outline)
        .children(handle_elements)
}

/// One resize handle.
fn handle_element(
    handle: ResizeHandle,
    x: Pixels,
    y: Pixels,
    index: usize,
    object: &Object,
    cx: Option<&mut Context<MainWindow>>,
) -> impl IntoElement {
    let start_center = Point {
        x: object.position_x,
        y: object.position_y,
    };
    let start_size = (object.width, object.height);

    let handle_div = div()
        .id(SharedString::from(format!(
            "canvas-handle-{index}-{handle:?}"
        )))
        .absolute()
        .left(x)
        .top(y)
        .w(HANDLE_SIZE)
        .h(HANDLE_SIZE)
        .bg(gpui_kit::rgb(0xF03708))
        .border_1()
        .border_color(gpui_kit::rgb(0xFFFFFF));

    let Some(cx) = cx else {
        return handle_div.into_any_element();
    };

    handle_div
        // The handle is observed, so a test can drag it: the handles are how
        // the operator changes an object's size.
        .test_support()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |w, e: &MouseDownEvent, window, cx| {
                if e.button != MouseButton::Left {
                    return;
                }
                // The handle is on top of the object, so the click belongs to
                // the handle: without this the object underneath would be
                // dragged as well.
                cx.stop_propagation();
                let (center, size) = live_geometry(w, index).unwrap_or((start_center, start_size));
                w.canvas.drag = Some(DragState {
                    index,
                    handle: Some(handle),
                    start_pointer: e.position,
                    start_center: center,
                    start_size: size,
                    moved: false,
                });
                w.handle_message(
                    Message::Layers(layers::Message::SelectLayer(Some(index))),
                    window,
                    cx,
                );
            }),
        )
        .into_any_element()
}

/// Apply a pointer movement to the object being dragged.
fn drag_moved(
    w: &mut MainWindow,
    event: &MouseMoveEvent,
    window: &mut gpui_kit::Window,
    cx: &mut Context<MainWindow>,
    canvas_width: f32,
    canvas_height: f32,
) {
    let Some(drag) = w.canvas.drag.as_ref() else {
        return;
    };
    // The drag only applies while the dragged object is still the selected
    // one: the operator may have picked a different layer mid-drag.
    if w.layer.selected_index != Some(drag.index) {
        return;
    }
    if canvas_width <= 0.0 || canvas_height <= 0.0 {
        return;
    }

    // The drag's values are copied out, so the drag state can be changed
    // while the drag is applied.
    let index = drag.index;
    let handle = drag.handle;
    let start_pointer = drag.start_pointer;
    let start_center = drag.start_center;
    let start_size = drag.start_size;
    let moved = drag.moved;

    let dx = f32::from(event.position.x - start_pointer.x) / canvas_width;
    let dy = f32::from(event.position.y - start_pointer.y) / canvas_height;

    // The drag becomes an edit the first time the object actually moves.
    // Until then it is just a click that selected the object, and the undo
    // stack must not fill up with entries that change nothing.
    if !moved && (dx != 0.0 || dy != 0.0) {
        if let Some(drag) = w.canvas.drag.as_mut() {
            drag.moved = true;
        }
        w.handle_message(
            Message::Layers(layers::Message::LayerDragStarted(index)),
            window,
            cx,
        );
    }

    match handle {
        None => {
            // Move: the centre follows the pointer, so the grab offset inside
            // the object is preserved.
            let x = (start_center.x + dx).clamp(0.0, 1.0);
            let y = (start_center.y + dy).clamp(0.0, 1.0);
            w.handle_message(
                Message::Layers(layers::Message::LayerDragged(point(px(x), px(y)))),
                window,
                cx,
            );
        }
        Some(handle) => {
            // Resize: the corner opposite the handle stays anchored.
            let (start_w, start_h) = start_size;
            let (start_cx, start_cy) = (start_center.x, start_center.y);
            let left = start_cx - start_w / 2.0;
            let right = start_cx + start_w / 2.0;
            let top = start_cy - start_h / 2.0;
            let bottom = start_cy + start_h / 2.0;

            let (new_w, new_h) = match handle {
                ResizeHandle::TopLeft => (start_w - dx, start_h - dy),
                ResizeHandle::TopRight => (start_w + dx, start_h - dy),
                ResizeHandle::BottomLeft => (start_w - dx, start_h + dy),
                ResizeHandle::BottomRight => (start_w + dx, start_h + dy),
            };
            let new_w = new_w.clamp(0.02, 1.0);
            let new_h = new_h.clamp(0.02, 1.0);

            let (new_cx, new_cy) = match handle {
                ResizeHandle::TopLeft => (right - new_w / 2.0, bottom - new_h / 2.0),
                ResizeHandle::TopRight => (left + new_w / 2.0, bottom - new_h / 2.0),
                ResizeHandle::BottomLeft => (right - new_w / 2.0, top + new_h / 2.0),
                ResizeHandle::BottomRight => (left + new_w / 2.0, top + new_h / 2.0),
            };

            w.handle_message(
                Message::Layers(layers::Message::LayerResized {
                    position: point(px(new_cx.clamp(0.0, 1.0)), px(new_cy.clamp(0.0, 1.0))),
                    width: new_w,
                    height: new_h,
                }),
                window,
                cx,
            );
        }
    }
}

/// Finish a drag.
fn drag_ended(w: &mut MainWindow, window: &mut gpui_kit::Window, cx: &mut Context<MainWindow>) {
    if w.canvas.drag.take().is_none() {
        return;
    }
    w.handle_message(Message::Layers(layers::Message::LayerDragEnded), window, cx);
}

/// Convert a domain colour into a GPUI colour.
pub(crate) fn to_hsla(color: Color) -> Hsla {
    Rgba {
        r: color.r as f32 / 255.0,
        g: color.g as f32 / 255.0,
        b: color.b as f32 / 255.0,
        a: color.a as f32 / 255.0,
    }
    .into()
}

/// The domain's text alignment as a GPUI one.
fn text_align(alignment: TextAlignment) -> TextAlign {
    match alignment {
        TextAlignment::Left => TextAlign::Left,
        TextAlignment::Center => TextAlign::Center,
        TextAlignment::Right => TextAlign::Right,
    }
}

/// The `#RRGGBB` form of a domain colour, for inline SVG.
fn color_hex(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    use crate::domain::Object;
    use gpui_kit::size;

    /// A text object is drawn where its position says, not at its position
    /// twice over.
    ///
    /// The text sits inside the object's box, and the box already sits at
    /// the object's position. Measuring the anchor from the canvas put every
    /// text in the corner of the slide, so the editor showed something the
    /// audience never saw.
    #[test]
    fn a_text_object_is_drawn_where_its_position_says() {
        let canvas = size(px(600.), px(338.));
        let mut object = Object::new_text("Hello".to_string());
        object.position_x = 0.5;
        object.position_y = 0.5;
        object.width = 0.4;
        object.height = 0.2;

        let (left, top, _, _) = object_box(&object, canvas);
        let anchor = text_anchor_in_box(
            Point {
                x: object.position_x,
                y: object.position_y,
            },
            canvas,
            (left, top),
        );

        // The anchor, measured from the box, plus the box's own origin is
        // the position on the canvas.
        let on_canvas_x = f32::from(left) + anchor.x;
        let on_canvas_y = f32::from(top) + anchor.y;
        assert!(
            (on_canvas_x - object.position_x * f32::from(canvas.width)).abs() < 0.01,
            "the text is at the object's x ({on_canvas_x})"
        );
        assert!(
            (on_canvas_y - object.position_y * f32::from(canvas.height)).abs() < 0.01,
            "the text is at the object's y ({on_canvas_y})"
        );
    }

    /// The same holds for an object in the corner and for one that hangs
    /// over the edge of the slide.
    #[test]
    fn a_text_object_is_drawn_where_its_position_says_wherever_it_is() {
        let canvas = size(px(800.), px(450.));
        for (x, y) in [(0.1f32, 0.2f32), (0.5, 0.5), (0.9, 0.8), (0.0, 1.0)] {
            let mut object = Object::new_text("Hello".to_string());
            object.position_x = x;
            object.position_y = y;
            object.width = 0.3;
            object.height = 0.15;

            let (left, top, _, _) = object_box(&object, canvas);
            let anchor = text_anchor_in_box(
                Point {
                    x: object.position_x,
                    y: object.position_y,
                },
                canvas,
                (left, top),
            );
            let on_canvas_x = f32::from(left) + anchor.x;
            let on_canvas_y = f32::from(top) + anchor.y;
            assert!(
                (on_canvas_x - x * f32::from(canvas.width)).abs() < 0.01,
                "the text is at the object's x for ({x},{y})"
            );
            assert!(
                (on_canvas_y - y * f32::from(canvas.height)).abs() < 0.01,
                "the text is at the object's y for ({x},{y})"
            );
        }
    }

    /// A text with a single run is drawn at that run's size.
    ///
    /// The projector honours a run's own size, so the editor has to as
    /// well: an operator who sets a size and watches the editor keep the
    /// old one has no way to tell what the audience will see.
    #[test]
    fn a_single_run_sizes_the_text_in_the_editor() {
        use crate::domain::{TextRun, TextStyle};

        let canvas = size(px(600.), px(338.));
        let style = TextStyle {
            font_size: 40.0,
            ..Default::default()
        };
        let position = Point { x: 0.5, y: 0.5 };

        let one = vec![TextRun {
            text: "Hello".to_string(),
            font_size: Some(80.0),
            ..Default::default()
        }];
        let layout = text_layout(&style, &one, position, canvas, 1.0, (px(0.), px(0.)));
        assert_eq!(
            layout.font_size, 80.0,
            "the run's own size is the size the editor draws"
        );

        // With several runs the style's size is used: GPUI measures a text
        // element as a whole, so the runs cannot each have their own size.
        let many = vec![
            TextRun {
                text: "a".to_string(),
                font_size: Some(80.0),
                ..Default::default()
            },
            TextRun {
                text: "b".to_string(),
                font_size: Some(20.0),
                ..Default::default()
            },
        ];
        let layout = text_layout(&style, &many, position, canvas, 1.0, (px(0.), px(0.)));
        assert_eq!(
            layout.font_size, 40.0,
            "a styled text keeps the style's size"
        );
    }

    /// A raised or a lowered text is drawn off its line.
    ///
    /// The output pipeline draws a note that way — a superscript is smaller
    /// and sits above the line — so the editor has to as well: an operator
    /// who raises a text and watches the editor keep it on the line has no
    /// way to tell what the audience will see.
    #[test]
    fn a_baseline_moves_the_text_off_its_line() {
        use crate::domain::{Baseline, TextStyle};

        let canvas = size(px(600.), px(338.));
        let position = Point { x: 0.5, y: 0.5 };
        let plain = TextStyle {
            font_size: 40.0,
            ..Default::default()
        };

        let on_the_line = text_layout(&plain, &[], position, canvas, 1.0, (px(0.), px(0.)));

        let raised = TextStyle {
            baseline: Baseline::Raise,
            ..plain.clone()
        };
        let layout = text_layout(&raised, &[], position, canvas, 1.0, (px(0.), px(0.)));
        assert!(
            layout.anchor.y < on_the_line.anchor.y,
            "a raised text is drawn above the line"
        );
        assert_eq!(layout.font_size, 40.0, "a raised text keeps its size");

        let lowered = TextStyle {
            baseline: Baseline::Lower,
            ..plain.clone()
        };
        let layout = text_layout(&lowered, &[], position, canvas, 1.0, (px(0.), px(0.)));
        assert!(
            layout.anchor.y > on_the_line.anchor.y,
            "a lowered text is drawn below the line"
        );

        // A superscript is a note: it is smaller as well as higher.
        let superscript = TextStyle {
            baseline: Baseline::Superscript,
            ..plain.clone()
        };
        let layout = text_layout(&superscript, &[], position, canvas, 1.0, (px(0.), px(0.)));
        assert!(
            (layout.font_size - 40.0 * 0.66).abs() < 0.01,
            "a superscript is drawn smaller ({})",
            layout.font_size
        );
        assert!(
            layout.anchor.y < on_the_line.anchor.y,
            "a superscript is drawn above the line"
        );
    }

    /// A small-caps text keeps its first letter at the full size.
    ///
    /// The rest of the letters are drawn as small capitals, so the head and
    /// the tail are split the way the output pipeline splits them.
    #[test]
    fn a_small_caps_text_keeps_its_first_letter_at_the_full_size() {
        assert_eq!(small_caps_split("Hello"), ("H", "ello"));
        assert_eq!(small_caps_split(" Hello"), (" H", "ello"));
        assert_eq!(small_caps_split(""), ("", ""));
        assert_eq!(small_caps_split("   "), ("", "   "));
    }
}
