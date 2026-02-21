use crate::slides::{
    Background, ImageFit, LayerContent, ShapeType, Slide, SlideContent, TextAlignment, Transition,
};
use crate::ui::editor::canvas::{
    CANVAS_H, image_widget_panel, image_widget_panel_handle, letterbox,
};
use crate::ui::messages::Message;
use iced::{
    Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme,
    alignment, font, mouse,
    widget::{
        canvas::{self, Action, Canvas, Frame, Geometry, Path},
        text,
    },
};

pub struct PresenterProgram {
    pub current: Option<Slide>,
    pub from: Option<Slide>,
    pub transition: Transition,
    pub progress: f32,
}

impl canvas::Program<Message> for PresenterProgram {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        _event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        None
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        const DARK: Color = Color {
            r: 0.10,
            g: 0.10,
            b: 0.10,
            a: 1.0,
        };

        frame.fill_rectangle(Point::ORIGIN, bounds.size(), DARK);

        let Some(current) = &self.current else {
            return vec![frame.into_geometry()];
        };

        let (slide_w, slide_h, off_x, off_y) = letterbox(bounds);
        let scale = slide_h / CANVAS_H;

        match (&self.from, self.transition) {
            (Some(from), Transition::Fade { .. } | Transition::Dissolve { .. })
                if self.progress < 1.0 =>
            {
                draw_slide(&mut frame, from, off_x, off_y, slide_w, slide_h, scale, 1.0);
                draw_slide(
                    &mut frame,
                    current,
                    off_x,
                    off_y,
                    slide_w,
                    slide_h,
                    scale,
                    self.progress,
                );
            }

            (Some(from), Transition::Slide { .. }) if self.progress < 1.0 => {
                let shift = self.progress * slide_w;
                draw_slide(
                    &mut frame,
                    from,
                    off_x - shift,
                    off_y,
                    slide_w,
                    slide_h,
                    scale,
                    1.0,
                );
                draw_slide(
                    &mut frame,
                    current,
                    off_x + slide_w - shift,
                    off_y,
                    slide_w,
                    slide_h,
                    scale,
                    1.0,
                );
            }

            (Some(from), Transition::Push { direction, .. }) if self.progress < 1.0 => {
                let p = self.progress;
                match direction {
                    0 => {
                        draw_slide(
                            &mut frame,
                            from,
                            off_x + p * slide_w,
                            off_y,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                        draw_slide(
                            &mut frame,
                            current,
                            off_x - (1.0 - p) * slide_w,
                            off_y,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                    }
                    1 => {
                        draw_slide(
                            &mut frame,
                            from,
                            off_x - p * slide_w,
                            off_y,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                        draw_slide(
                            &mut frame,
                            current,
                            off_x + (1.0 - p) * slide_w,
                            off_y,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                    }
                    2 => {
                        draw_slide(
                            &mut frame,
                            from,
                            off_x,
                            off_y + p * slide_h,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                        draw_slide(
                            &mut frame,
                            current,
                            off_x,
                            off_y - (1.0 - p) * slide_h,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                    }
                    _ => {
                        draw_slide(
                            &mut frame,
                            from,
                            off_x,
                            off_y - p * slide_h,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                        draw_slide(
                            &mut frame,
                            current,
                            off_x,
                            off_y + (1.0 - p) * slide_h,
                            slide_w,
                            slide_h,
                            scale,
                            1.0,
                        );
                    }
                }
            }

            (Some(from), Transition::Zoom { .. }) if self.progress < 1.0 => {
                draw_slide(
                    &mut frame,
                    from,
                    off_x,
                    off_y,
                    slide_w,
                    slide_h,
                    scale,
                    1.0 - self.progress * 0.3,
                );
                let zoom_scale = self.progress;
                let zw = slide_w * zoom_scale;
                let zh = slide_h * zoom_scale;
                let zx = off_x + (slide_w - zw) / 2.0;
                let zy = off_y + (slide_h - zh) / 2.0;
                draw_slide(
                    &mut frame,
                    current,
                    zx,
                    zy,
                    zw,
                    zh,
                    scale * zoom_scale,
                    self.progress,
                );
            }

            (Some(from), Transition::Flip { .. }) if self.progress < 1.0 => {
                if self.progress < 0.5 {
                    let squeeze = 1.0 - self.progress * 2.0;
                    let zw = slide_w * squeeze;
                    let zx = off_x + (slide_w - zw) / 2.0;
                    draw_slide(&mut frame, from, zx, off_y, zw, slide_h, scale, 1.0);
                } else {
                    let squeeze = (self.progress - 0.5) * 2.0;
                    let zw = slide_w * squeeze;
                    let zx = off_x + (slide_w - zw) / 2.0;
                    draw_slide(&mut frame, current, zx, off_y, zw, slide_h, scale, 1.0);
                }
            }

            (Some(from), Transition::Clock { .. }) if self.progress < 1.0 => {
                draw_slide(&mut frame, from, off_x, off_y, slide_w, slide_h, scale, 1.0);
                draw_slide(
                    &mut frame,
                    current,
                    off_x,
                    off_y,
                    slide_w,
                    slide_h,
                    scale,
                    self.progress,
                );
            }

            (Some(from), Transition::Wipe { angle_deg, .. }) if self.progress < 1.0 => {
                draw_slide(&mut frame, from, off_x, off_y, slide_w, slide_h, scale, 1.0);
                draw_slide(
                    &mut frame, current, off_x, off_y, slide_w, slide_h, scale, 1.0,
                );
                let cover_alpha = 1.0 - self.progress;
                let _ = angle_deg;
                let revealed_w = slide_w * self.progress;
                let cover_x = off_x + revealed_w;
                let cover_w = slide_w - revealed_w;
                if cover_w > 0.5 {
                    frame.fill_rectangle(
                        Point::new(cover_x, off_y),
                        Size::new(cover_w, slide_h),
                        iced::Color::from_rgba(0.0, 0.0, 0.0, cover_alpha),
                    );
                }
            }

            _ => {
                draw_slide(
                    &mut frame, current, off_x, off_y, slide_w, slide_h, scale, 1.0,
                );
            }
        }

        let need_mask = matches!(
            self.transition,
            Transition::Slide { .. } | Transition::Push { .. }
        ) && self.from.is_some()
            && self.progress < 1.0;

        if need_mask {
            if off_x > 0.5 {
                frame.fill_rectangle(Point::ORIGIN, Size::new(off_x, bounds.height), DARK);
                frame.fill_rectangle(
                    Point::new(off_x + slide_w, 0.0),
                    Size::new(bounds.width - off_x - slide_w, bounds.height),
                    DARK,
                );
            }
            if off_y > 0.5 {
                frame.fill_rectangle(Point::ORIGIN, Size::new(bounds.width, off_y), DARK);
                frame.fill_rectangle(
                    Point::new(0.0, off_y + slide_h),
                    Size::new(bounds.width, bounds.height - off_y - slide_h),
                    DARK,
                );
            }
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(off_x.max(0.0), bounds.height),
                DARK,
            );
            frame.fill_rectangle(
                Point::new(off_x + slide_w, 0.0),
                Size::new(bounds.width, bounds.height),
                DARK,
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

pub fn presenter_canvas_panel<'a>(
    current: Option<&'a Slide>,
    from: Option<&'a Slide>,
    transition: Transition,
    progress: f32,
    video_frame: Option<&'a iced::widget::image::Handle>,
) -> Element<'a, Message> {
    if let Some(s) = current {
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
    Canvas::new(PresenterProgram {
        current: current.cloned(),
        from: from.cloned(),
        transition,
        progress,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub fn next_slide_canvas_panel<'a>(slide: Option<&'a Slide>) -> Element<'a, Message> {
    if let Some(s) = slide {
        match &s.content {
            SlideContent::Image { path, fit } if !path.is_empty() => {
                return image_widget_panel(path, *fit);
            }
            SlideContent::Video { thumbnail, .. } => {
                if let Some(t) = thumbnail
                    && !t.is_empty()
                {
                    return image_widget_panel(t, ImageFit::Fit);
                }
            }
            _ => {}
        }
    }
    Canvas::new(PresenterProgram {
        current: slide.cloned(),
        from: None,
        transition: Transition::Cut,
        progress: 1.0,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[allow(clippy::too_many_arguments)]
fn draw_slide(
    frame: &mut Frame,
    slide: &Slide,
    lx: f32,
    ty: f32,
    w: f32,
    h: f32,
    scale: f32,
    alpha: f32,
) {
    let bg = bg_to_color_alpha(&slide.background, alpha);
    frame.fill_rectangle(Point::new(lx, ty), Size::new(w, h), bg);

    let mut layers = slide.effective_layers().into_owned();
    layers.sort_by_key(|l| l.z_order);

    for layer in &layers {
        if !layer.visible {
            continue;
        }
        let layer_alpha = layer.opacity * alpha;
        let cx = lx + layer.position_x * w;
        let cy = ty + layer.position_y * h;
        let lw = layer.width * w;
        let lh = layer.height * h;
        let rx = cx - lw / 2.0;
        let ry = cy - lh / 2.0;

        match &layer.content {
            LayerContent::Text {
                text: content,
                style,
                ..
            } => {
                let display_content = style.text_transform.apply(content);
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
                    TextAlignment::Left => text::Alignment::Left,
                    TextAlignment::Center => text::Alignment::Center,
                    TextAlignment::Right => text::Alignment::Right,
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
                        let a = layer_alpha * 0.35 * (1.0 - i as f32 / glow_steps as f32);
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
                                max_width: lw.max(w),
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
                            a: 0.65 * layer_alpha,
                        },
                        size: font_size,
                        font: the_font,
                        max_width: lw.max(w),
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
                                a: layer_alpha,
                            },
                            size: font_size,
                            font: the_font,
                            max_width: lw.max(w),
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
                            color: Color {
                                a: layer_alpha,
                                ..sc
                            },
                            size: font_size,
                            font: the_font,
                            max_width: lw.max(w),
                            align_x,
                            align_y: alignment::Vertical::Center,
                            ..canvas::Text::default()
                        });
                    }
                }
                frame.fill_text(canvas::Text {
                    content: display_content.clone(),
                    position: Point::new(cx, cy),
                    color: Color::from_rgba8(
                        style.color.r,
                        style.color.g,
                        style.color.b,
                        (style.color.a as f32 / 255.0) * layer_alpha,
                    ),
                    size: font_size,
                    font: the_font,
                    max_width: lw.max(w),
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
                let fill_color = Color::from_rgba8(
                    fill.r,
                    fill.g,
                    fill.b,
                    (fill.a as f32 / 255.0) * layer_alpha,
                );
                let stroke_col = Color::from_rgba8(
                    stroke_color.r,
                    stroke_color.g,
                    stroke_color.b,
                    (stroke_color.a as f32 / 255.0) * layer_alpha,
                );
                let sw = stroke_width * scale;

                match shape_type {
                    ShapeType::Rectangle => {
                        frame.fill_rectangle(Point::new(rx, ry), Size::new(lw, lh), fill_color);
                        if sw > 0.0 {
                            let path = Path::rectangle(Point::new(rx, ry), Size::new(lw, lh));
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
                            b.move_to(Point::new(cx, ry));
                            b.line_to(Point::new(rx + lw, ry + lh));
                            b.line_to(Point::new(rx, ry + lh));
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
                            b.move_to(Point::new(rx, cy));
                            b.line_to(Point::new(rx + lw, cy));
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
}

fn bg_to_color_alpha(bg: &Background, alpha: f32) -> Color {
    match bg {
        Background::Solid(c) => Color::from_rgba8(c.r, c.g, c.b, (c.a as f32 / 255.0) * alpha),
        _ => Color {
            a: alpha,
            ..Color::BLACK
        },
    }
}
