use crate::slides::{Background, ImageFit, LayerContent, ShapeType, Slide, SlideContent};
use crate::ui::messages::Message;
use iced::{
    Background as IcedBg, Border, Color, ContentFit, Element, Event, Font, Length, Pixels, Point,
    Rectangle, Renderer, Size, Theme, alignment, font, mouse,
    widget::{
        canvas::{self, Action, Canvas, Frame, Geometry, Path},
        container,
        image::{Handle as ImageHandle, Image as IcedImage},
        text,
    },
};

pub const CANVAS_W: f32 = 960.0;
pub const CANVAS_H: f32 = 540.0;

#[derive(Default, Debug, Clone)]
pub struct CanvasState {
    is_dragging: bool,
    drag_start_mouse: Option<Point>,
    drag_start_text: (f32, f32),
    dragging_layer: Option<usize>,
}

pub struct SlideProgram {
    pub slide: Option<Slide>,
}

impl canvas::Program<Message> for SlideProgram {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut CanvasState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let slide = self.slide.as_ref()?;
        let (slide_w, slide_h, off_x, off_y) = letterbox(bounds);

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds)
                    && pos.x >= off_x
                    && pos.x <= off_x + slide_w
                    && pos.y >= off_y
                    && pos.y <= off_y + slide_h
                {
                    let norm_x = (pos.x - off_x) / slide_w;
                    let norm_y = (pos.y - off_y) / slide_h;

                    let layers = slide.effective_layers();
                    if layers.is_empty() {
                        return None;
                    }
                    let mut sorted = layers.iter().enumerate().collect::<Vec<_>>();
                    sorted.sort_by(|a, b| b.1.z_order.cmp(&a.1.z_order));

                    for (idx, layer) in sorted {
                        if !layer.visible || layer.locked {
                            continue;
                        }
                        let lx = layer.position_x - layer.width / 2.0;
                        let ly = layer.position_y - layer.height / 2.0;
                        if norm_x >= lx
                            && norm_x <= lx + layer.width
                            && norm_y >= ly
                            && norm_y <= ly + layer.height
                        {
                            state.is_dragging = true;
                            state.drag_start_mouse = Some(Point::new(norm_x, norm_y));
                            state.drag_start_text = (layer.position_x, layer.position_y);
                            state.dragging_layer = Some(idx);
                            return Some(Action::publish(Message::LayerDragStarted(idx)));
                        }
                    }

                    if let SlideContent::Text { style, .. } = &slide.content {
                        state.is_dragging = true;
                        state.drag_start_mouse = Some(Point::new(norm_x, norm_y));
                        state.drag_start_text = (style.position_x, style.position_y);
                        state.dragging_layer = None;
                        return Some(Action::publish(Message::TextDragStarted));
                    }
                }
                None
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.is_dragging => {
                let pos = cursor.position_in(bounds).or_else(|| cursor.position())?;
                let norm_x = ((pos.x - off_x) / slide_w).clamp(0.0, 1.0);
                let norm_y = ((pos.y - off_y) / slide_h).clamp(0.0, 1.0);
                if let Some(start) = state.drag_start_mouse {
                    let new_x = (state.drag_start_text.0 + norm_x - start.x).clamp(0.0, 1.0);
                    let new_y = (state.drag_start_text.1 + norm_y - start.y).clamp(0.0, 1.0);
                    let msg = if state.dragging_layer.is_some() {
                        Message::LayerDragged(Point::new(new_x, new_y))
                    } else {
                        Message::TextDragged(Point::new(new_x, new_y))
                    };
                    return Some(Action::publish(msg));
                }
                Some(Action::capture())
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.is_dragging =>
            {
                state.is_dragging = false;
                state.drag_start_mouse = None;
                state.dragging_layer = None;
                Some(Action::publish(Message::TextDragEnded))
            }

            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &CanvasState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.10, 0.10, 0.10),
        );

        let Some(slide) = &self.slide else {
            return vec![frame.into_geometry()];
        };

        let (slide_w, slide_h, off_x, off_y) = letterbox(bounds);
        let scale = slide_h / CANVAS_H;

        let bg = bg_to_color(&slide.background);
        frame.fill_rectangle(Point::new(off_x, off_y), Size::new(slide_w, slide_h), bg);

        let mut layers = slide.effective_layers().into_owned();
        layers.sort_by_key(|l| l.z_order);

        for layer in &layers {
            if !layer.visible {
                continue;
            }
            let alpha = layer.opacity;
            let cx = off_x + layer.position_x * slide_w;
            let cy = off_y + layer.position_y * slide_h;
            let lw = layer.width * slide_w;
            let lh = layer.height * slide_h;
            let lx = cx - lw / 2.0;
            let ly = cy - lh / 2.0;

            match &layer.content {
                LayerContent::Text {
                    text: content,
                    style,
                    ..
                } => {
                    let display_content = style.text_transform.apply(content);
                    let text_color = Color::from_rgba8(
                        style.color.r,
                        style.color.g,
                        style.color.b,
                        (style.color.a as f32 / 255.0) * alpha,
                    );
                    let font_size = Pixels(style.font_size * scale);
                    let the_font = Font {
                        weight: if style.bold {
                            font::Weight::Bold
                        } else {
                            font::Weight::Normal
                        },
                        style: if style.italic {
                            font::Style::Italic
                        } else {
                            font::Style::Normal
                        },
                        ..Font::DEFAULT
                    };
                    let align_x = match style.alignment {
                        crate::slides::TextAlignment::Left => text::Alignment::Left,
                        crate::slides::TextAlignment::Center => text::Alignment::Center,
                        crate::slides::TextAlignment::Right => text::Alignment::Right,
                    };
                    if style.glow_enabled && style.glow_radius > 0.0 {
                        let glow_steps = 8;
                        let gc = Color::from_rgba8(
                            style.glow_color.r,
                            style.glow_color.g,
                            style.glow_color.b,
                            1.0,
                        );
                        for i in 1..=glow_steps {
                            let r = style.glow_radius * scale * (i as f32 / glow_steps as f32);
                            let a = alpha * 0.35 * (1.0 - i as f32 / glow_steps as f32);
                            for &(dx, dy) in &[
                                (-r, -r),
                                (0.0, -r),
                                (r, -r),
                                (-r, 0.0),
                                (r, 0.0),
                                (-r, r),
                                (0.0, r),
                                (r, r),
                            ] {
                                frame.fill_text(canvas::Text {
                                    content: display_content.clone(),
                                    position: Point::new(cx + dx, cy + dy),
                                    color: Color { a, ..gc },
                                    size: font_size,
                                    font: the_font,
                                    max_width: lw.max(slide_w),
                                    align_x,
                                    align_y: alignment::Vertical::Center,
                                    ..canvas::Text::default()
                                });
                            }
                        }
                    }
                    if style.shadow {
                        let soff = 2.0_f32.max(scale * 2.0);
                        frame.fill_text(canvas::Text {
                            content: display_content.clone(),
                            position: Point::new(cx + soff, cy + soff),
                            color: Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.65 * alpha,
                            },
                            size: font_size,
                            font: the_font,
                            max_width: lw.max(slide_w),
                            align_x,
                            align_y: alignment::Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }
                    if style.outline {
                        let r = scale.max(1.0);
                        for &(dx, dy) in &[
                            (-r, -r),
                            (0.0, -r),
                            (r, -r),
                            (-r, 0.0),
                            (r, 0.0),
                            (-r, r),
                            (0.0, r),
                            (r, r),
                        ] {
                            frame.fill_text(canvas::Text {
                                content: display_content.clone(),
                                position: Point::new(cx + dx, cy + dy),
                                color: Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: alpha,
                                },
                                size: font_size,
                                font: the_font,
                                max_width: lw.max(slide_w),
                                align_x,
                                align_y: alignment::Vertical::Center,
                                ..canvas::Text::default()
                            });
                        }
                    }
                    if style.text_stroke_width > 0.0 {
                        let sw = style.text_stroke_width * scale;
                        let sc = Color::from_rgba8(
                            style.text_stroke_color.r,
                            style.text_stroke_color.g,
                            style.text_stroke_color.b,
                            1.0,
                        );
                        for &(dx, dy) in &[
                            (-sw, -sw),
                            (0.0, -sw),
                            (sw, -sw),
                            (-sw, 0.0),
                            (sw, 0.0),
                            (-sw, sw),
                            (0.0, sw),
                            (sw, sw),
                        ] {
                            frame.fill_text(canvas::Text {
                                content: display_content.clone(),
                                position: Point::new(cx + dx, cy + dy),
                                color: Color { a: alpha, ..sc },
                                size: font_size,
                                font: the_font,
                                max_width: lw.max(slide_w),
                                align_x,
                                align_y: alignment::Vertical::Center,
                                ..canvas::Text::default()
                            });
                        }
                    }
                    frame.fill_text(canvas::Text {
                        content: display_content.clone(),
                        position: Point::new(cx, cy),
                        color: text_color,
                        size: font_size,
                        font: the_font,
                        max_width: lw.max(slide_w),
                        align_x,
                        align_y: alignment::Vertical::Center,
                        ..canvas::Text::default()
                    });
                }

                LayerContent::Shape {
                    shape_type,
                    fill,
                    stroke_color,
                    stroke_width,
                    ..
                } => {
                    let fill_color =
                        Color::from_rgba8(fill.r, fill.g, fill.b, (fill.a as f32 / 255.0) * alpha);
                    let stroke_col = Color::from_rgba8(
                        stroke_color.r,
                        stroke_color.g,
                        stroke_color.b,
                        (stroke_color.a as f32 / 255.0) * alpha,
                    );
                    let sw = stroke_width * scale;

                    match shape_type {
                        ShapeType::Rectangle => {
                            frame.fill_rectangle(Point::new(lx, ly), Size::new(lw, lh), fill_color);
                            if sw > 0.0 {
                                let path = Path::rectangle(Point::new(lx, ly), Size::new(lw, lh));
                                frame.stroke(
                                    &path,
                                    canvas::Stroke::default()
                                        .with_color(stroke_col)
                                        .with_width(sw),
                                );
                            }
                        }
                        ShapeType::Ellipse => {
                            let path = Path::new(|b| {
                                b.ellipse(canvas::path::arc::Elliptical {
                                    center: Point::new(cx, cy),
                                    radii: iced::Vector::new(lw / 2.0, lh / 2.0),
                                    rotation: iced::Radians(0.0),
                                    start_angle: iced::Radians(0.0),
                                    end_angle: iced::Radians(std::f32::consts::TAU),
                                });
                            });
                            frame.fill(&path, fill_color);
                            if sw > 0.0 {
                                frame.stroke(
                                    &path,
                                    canvas::Stroke::default()
                                        .with_color(stroke_col)
                                        .with_width(sw),
                                );
                            }
                        }
                        ShapeType::Triangle => {
                            let path = Path::new(|b| {
                                b.move_to(Point::new(cx, ly));
                                b.line_to(Point::new(lx + lw, ly + lh));
                                b.line_to(Point::new(lx, ly + lh));
                                b.close();
                            });
                            frame.fill(&path, fill_color);
                            if sw > 0.0 {
                                frame.stroke(
                                    &path,
                                    canvas::Stroke::default()
                                        .with_color(stroke_col)
                                        .with_width(sw),
                                );
                            }
                        }
                        ShapeType::Line => {
                            let path = Path::new(|b| {
                                b.move_to(Point::new(lx, cy));
                                b.line_to(Point::new(lx + lw, cy));
                            });
                            frame.stroke(
                                &path,
                                canvas::Stroke::default()
                                    .with_color(fill_color)
                                    .with_width(sw.max(1.0)),
                            );
                        }
                    }
                }

                LayerContent::Image { .. } | LayerContent::Video { .. } => {}
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &CanvasState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.is_dragging {
            return mouse::Interaction::Grabbing;
        }
        if let Some(pos) = cursor.position_in(bounds) {
            let (slide_w, slide_h, off_x, off_y) = letterbox(bounds);
            if pos.x >= off_x
                && pos.x <= off_x + slide_w
                && pos.y >= off_y
                && pos.y <= off_y + slide_h
                && let Some(slide) = &self.slide
            {
                let norm_x = (pos.x - off_x) / slide_w;
                let norm_y = (pos.y - off_y) / slide_h;
                let layers = slide.effective_layers();
                let any_hit = layers.iter().any(|l| {
                    if !l.visible || l.locked {
                        return false;
                    }
                    let lx = l.position_x - l.width / 2.0;
                    let ly = l.position_y - l.height / 2.0;
                    norm_x >= lx
                        && norm_x <= lx + l.width
                        && norm_y >= ly
                        && norm_y <= ly + l.height
                });
                if any_hit {
                    return mouse::Interaction::Crosshair;
                }
            }
        }
        mouse::Interaction::default()
    }
}

pub fn canvas_panel<'a>(
    slide: Option<&'a Slide>,
    video_frame: Option<&'a iced::widget::image::Handle>,
) -> Element<'a, Message> {
    if let Some(s) = slide {
        match &s.content {
            SlideContent::Image { path, fit } if !path.is_empty() => {
                return image_widget_panel(path, *fit);
            }
            SlideContent::Video { thumbnail, .. } => {
                if let Some(handle) = video_frame {
                    return image_widget_panel_handle(handle.clone());
                }
                if let Some(t) = thumbnail
                    && !t.is_empty()
                {
                    return image_widget_panel(t, ImageFit::Fit);
                }
            }
            _ => {}
        }
    }
    Canvas::new(SlideProgram {
        slide: slide.cloned(),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn letterbox(bounds: Rectangle) -> (f32, f32, f32, f32) {
    let aspect = CANVAS_W / CANVAS_H;
    let (slide_w, slide_h) = if bounds.width / bounds.height > aspect {
        (bounds.height * aspect, bounds.height)
    } else {
        (bounds.width, bounds.width / aspect)
    };
    let off_x = (bounds.width - slide_w) / 2.0;
    let off_y = (bounds.height - slide_h) / 2.0;
    (slide_w, slide_h, off_x, off_y)
}

pub fn bg_to_color(bg: &Background) -> Color {
    match bg {
        Background::Solid(c) => Color::from_rgba8(c.r, c.g, c.b, c.a as f32 / 255.0),
        _ => Color::BLACK,
    }
}

pub fn image_widget_panel<'a>(path: &str, fit: ImageFit) -> Element<'a, Message> {
    let content_fit = image_fit_to_content_fit(fit);
    container(
        IcedImage::new(ImageHandle::from_path(path))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(content_fit),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(IcedBg::Color(Color::BLACK)),
        border: Border::default(),
        ..Default::default()
    })
    .into()
}

pub fn image_widget_panel_handle<'a>(handle: ImageHandle) -> Element<'a, Message> {
    container(
        IcedImage::new(handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Contain),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_: &iced::Theme| iced::widget::container::Style {
        background: Some(IcedBg::Color(Color::BLACK)),
        border: Border::default(),
        ..Default::default()
    })
    .into()
}

pub fn image_fit_to_content_fit(fit: ImageFit) -> ContentFit {
    match fit {
        ImageFit::Fit => ContentFit::Contain,
        ImageFit::Fill => ContentFit::Cover,
        ImageFit::Stretch => ContentFit::Fill,
        ImageFit::Center => ContentFit::None,
    }
}
