use crate::Result;
use crate::domain::{
    Background, Color, ImageFit, Mask, ObjectContent, Prop, PropContent, ShapeType, Slide,
    TextAlignment, TextStyle, Transition,
};
use std::num::NonZeroUsize;
use std::sync::Arc;

pub mod frame_slot;
pub mod gpu_text;
pub mod overlay;
pub mod player;
pub mod style;
pub mod test_pattern;
pub use frame_slot::{FrameSlot, frame_to_image};
pub use gpu_text::{GpuTextRenderer, create_gpu_text_renderer};
pub use overlay::Overlay;
pub use player::SlidePlayer;
pub use style::StyleOverride;

const IMAGE_CACHE_SIZE: usize = 16;

/// Scale a colour's alpha by an object's opacity.
///
/// The opacity slider has to reach the pixels for every kind of object, not
/// only the shapes and the images: a text that fades in the editor has to
/// fade on the projector as well.
fn faded(color: Color, opacity: f32) -> Color {
    Color {
        a: (color.a as f32 * opacity.clamp(0.0, 1.0)).round() as u8,
        ..color
    }
}

/// The way an object is turned over.
///
/// The mirror is about the object's own centre: the object keeps its place
/// and its size on the slide, and only the picture inside it is turned over.
/// A text is never turned over — a title that reads backwards is a mistake
/// the operator cannot undo by switching the mirror off again.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Flip {
    horizontal: bool,
    vertical: bool,
}

#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Arc<Vec<u8>>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            width,
            height,
            data: Arc::new(vec![0; size]),
        }
    }

    pub fn from_data(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data: Arc::new(data),
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for pixel in Arc::make_mut(&mut self.data).as_chunks_mut::<4>().0 {
            *pixel = [b, g, r, a];
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let data = Arc::make_mut(&mut self.data);
        data[idx] = b;
        data[idx + 1] = g;
        data[idx + 2] = r;
        data[idx + 3] = a;
    }

    /// Read a pixel back as `(r, g, b, a)`.
    ///
    /// The frame stores pixels in BGRA order; this returns them in the more
    /// familiar RGBA order.
    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8, u8)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let data = self.data.as_ref();
        Some((data[idx + 2], data[idx + 1], data[idx], data[idx + 3]))
    }

    pub fn blend_over(&self, other: &Frame, t: f32) -> Frame {
        assert_eq!(self.width, other.width);
        assert_eq!(self.height, other.height);
        let t = t.clamp(0.0, 1.0);
        let mut out_data = vec![0u8; self.data.len()];
        for (i, (a, b)) in self.data.iter().zip(other.data.iter()).enumerate() {
            out_data[i] = (*a as f32 * (1.0 - t) + *b as f32 * t) as u8;
        }
        Frame::from_data(self.width, self.height, out_data)
    }
}

/// What an image is used for.
///
/// The same file can be a background, an object and a mask, and each draws it
/// at a different size — so the cache has to tell them apart: one file under
/// one key would put the background's crop where the object belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ImageUse {
    /// The slide's background: resized to cover the frame.
    Background,
    /// An image object: drawn at its own size.
    Object,
    /// A custom mask: resized to the frame.
    Mask,
}

/// The cache key of an image: what it is used for, and which file it is.
type ImageKey = (ImageUse, String);

pub struct RenderPipeline {
    pub width: u32,
    pub height: u32,
    /// The resolution the content was authored against.
    ///
    /// The text is sized in points against this width, so an output at
    /// another resolution scales its text to keep the same look: a slide
    /// shown on a 720p screen has to look like the same slide on a 1080p
    /// one, not a bigger version of it.
    reference_width: u32,
    /// The GPU text renderer, shared between the pipelines.
    ///
    /// The renderer owns a wgpu device, which is expensive to create, and it
    /// does not depend on the output size: the size is the viewport the
    /// text is drawn into. So the screens that run at different resolutions
    /// can share one renderer and still have a pipeline of their own size.
    gpu_text: Option<SharedGpuText>,
    image_cache: lru::LruCache<ImageKey, image::RgbaImage>,
    /// Paths that previously failed to load. Without this, a missing image is
    /// re-read from disk and re-logged on *every* frame, which floods the log
    /// and burns I/O for the whole duration of a slide.
    failed_images: std::collections::HashSet<String>,
}

/// A GPU text renderer shared by the pipelines.
pub type SharedGpuText = std::sync::Arc<std::sync::Mutex<GpuTextRenderer>>;

impl RenderPipeline {
    /// Build a pipeline that creates its own GPU text renderer.
    ///
    /// The renderer owns a wgpu device, so creating one is expensive; the
    /// off-thread compositor ([`crate::ui::compositor`]) keeps a single
    /// renderer alive across resolution changes by using
    /// [`Self::with_gpu_text`] and [`Self::take_gpu_text`] instead.
    pub fn new(width: u32, height: u32) -> Self {
        let gpu_text = match create_gpu_text_renderer() {
            Ok(r) => {
                log::info!("GPU text renderer initialised (glyphon)");
                Some(SharedGpuText::new(std::sync::Mutex::new(r)))
            }
            Err(e) => {
                log::warn!("GPU text renderer unavailable, using pixel font: {e}");
                None
            }
        };
        Self::with_gpu_text(gpu_text, width, height)
    }

    /// Build a pipeline around an already-created GPU text renderer.
    pub fn with_gpu_text(gpu_text: Option<SharedGpuText>, width: u32, height: u32) -> Self {
        let cache_cap = NonZeroUsize::new(IMAGE_CACHE_SIZE).expect("cache size > 0");
        Self {
            width,
            height,
            reference_width: width,
            gpu_text,
            image_cache: lru::LruCache::new(cache_cap),
            failed_images: std::collections::HashSet::new(),
        }
    }

    /// Build a pipeline without GPU text rendering (pixel font fallback).
    pub fn new_software(width: u32, height: u32) -> Self {
        Self::with_gpu_text(None, width, height)
    }

    /// The resolution the content was authored against.
    ///
    /// The text is sized in points against it, so an output at another
    /// resolution scales its text to keep the same look.
    pub fn set_reference_width(&mut self, reference_width: u32) {
        if reference_width > 0 {
            self.reference_width = reference_width;
        }
    }

    /// The factor the text is scaled by for this output's resolution.
    fn reference_scale(&self) -> f32 {
        if self.reference_width == 0 {
            1.0
        } else {
            (self.width as f32 / self.reference_width as f32).max(0.01)
        }
    }

    /// Take the GPU text renderer out of the pipeline.
    ///
    /// Used by the compositor worker to move the renderer into a pipeline of
    /// a different size instead of recreating a wgpu device. The renderer is
    /// shared, so the pipelines that keep a copy of it all draw their text
    /// with the same device.
    pub fn take_gpu_text(&mut self) -> Option<SharedGpuText> {
        self.gpu_text.take()
    }

    pub fn render_slide(&mut self, slide: &Slide) -> Result<Frame> {
        self.render_slide_with_video(slide, None)
    }

    pub fn render_slide_with_video(
        &mut self,
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
    ) -> Result<Frame> {
        self.render_slide_with_overlay(slide, video_frame, &Overlay::default())
    }

    /// Render a slide with the overlay (props, mask, live message) composited
    /// on top.
    ///
    /// Every output surface goes through this method, so the preview, the
    /// audience window, the NDI sender and the recorder always show the same
    /// composition.
    pub fn render_slide_with_overlay(
        &mut self,
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
        overlay: &Overlay,
    ) -> Result<Frame> {
        self.render_slide_styled(slide, video_frame, overlay, &StyleOverride::default())
    }

    /// Render a slide with an overlay and a screen-specific style.
    ///
    /// The style override lets one screen show the slide in a different
    /// theme without the slide itself changing.
    pub fn render_slide_styled(
        &mut self,
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
        overlay: &Overlay,
        style: &StyleOverride,
    ) -> Result<Frame> {
        let mut frame = Frame::new(self.width, self.height);
        if overlay.show_slide {
            self.draw_background(
                &mut frame,
                style.background.as_ref().unwrap_or(&slide.background),
                video_frame,
                overlay.show_media,
            );
            self.draw_content(
                &mut frame,
                slide,
                video_frame,
                overlay.show_media,
                style.text_style.as_ref(),
            );
        }
        self.draw_overlay(&mut frame, overlay);
        Ok(frame)
    }

    pub fn render_black(&self) -> Frame {
        let mut frame = Frame::new(self.width, self.height);
        frame.clear(0, 0, 0, 255);
        frame
    }

    fn draw_background(
        &mut self,
        frame: &mut Frame,
        background: &Background,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
        show_media: bool,
    ) {
        match background {
            Background::Solid(color) => {
                frame.clear(color.r, color.g, color.b, 255);
            }
            Background::Image(path) if !path.is_empty() => {
                if self.failed_images.contains(path) {
                    frame.clear(0, 0, 0, 255);
                    return;
                }
                let key = (ImageUse::Background, path.clone());
                if !self.image_cache.contains(&key) {
                    match image::open(path) {
                        Ok(img) => {
                            // The background covers the frame, the way the
                            // editor draws it: the image keeps its shape and
                            // the overflow is cropped. Stretching it to the
                            // frame would show a picture on the projector
                            // that the editor never showed.
                            let src_w = img.width() as f32;
                            let src_h = img.height() as f32;
                            if src_w > 0.0 && src_h > 0.0 {
                                let scale =
                                    (self.width as f32 / src_w).max(self.height as f32 / src_h);
                                let new_w = (src_w * scale).round().max(1.0) as u32;
                                let new_h = (src_h * scale).round().max(1.0) as u32;
                                let resized = img
                                    .resize_exact(
                                        new_w,
                                        new_h,
                                        image::imageops::FilterType::Triangle,
                                    )
                                    .to_rgba8();
                                self.image_cache.put(key.clone(), resized);
                            }
                        }
                        Err(e) => {
                            log::error!("background image load error '{path}': {e}");
                            self.failed_images.insert(path.clone());
                            frame.clear(0, 0, 0, 255);
                            return;
                        }
                    }
                }
                if let Some(img) = self.image_cache.get(&key) {
                    // The image is larger than the frame, so the overflow is
                    // cropped in the middle.
                    let offset_x = (img.width().saturating_sub(self.width)) / 2;
                    let offset_y = (img.height().saturating_sub(self.height)) / 2;
                    let buf = Arc::make_mut(&mut frame.data);
                    for (y, row) in img.rows().enumerate() {
                        let y = y as u32;
                        if y < offset_y || y >= offset_y + self.height {
                            continue;
                        }
                        for (x, pixel) in row.enumerate() {
                            let x = x as u32;
                            if x < offset_x || x >= offset_x + self.width {
                                continue;
                            }
                            let [r, g, b, a] = pixel.0;
                            let alpha = a as f32 / 255.0;
                            let idx = (((y - offset_y) * self.width + (x - offset_x)) * 4) as usize;
                            if idx + 3 < buf.len() {
                                buf[idx] = (b as f32 * alpha) as u8;
                                buf[idx + 1] = (g as f32 * alpha) as u8;
                                buf[idx + 2] = (r as f32 * alpha) as u8;
                                buf[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
            Background::Image(_) => {
                frame.clear(0, 0, 0, 255);
            }
            Background::Video(_path) => {
                if show_media && let Some(vf) = video_frame {
                    composite_rgba_to_bgra(frame, vf);
                } else {
                    frame.clear(0, 0, 0, 255);
                }
            }
        }
    }

    fn draw_content(
        &mut self,
        frame: &mut Frame,
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
        show_media: bool,
        text_style_override: Option<&TextStyle>,
    ) {
        let mut layers = slide.effective_layers().into_owned();
        layers.sort_by_key(|l| l.z_order);

        for layer in &layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            match &layer.content {
                ObjectContent::Text { text, style, runs } => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    // A text is never mirrored: a title that reads backwards
                    // is a mistake the operator cannot undo by switching the
                    // mirror off again.
                    //
                    // The object's position is the text's position: the
                    // canvas, the geometry panel and the position presets all
                    // move the object, so the renderer has to follow it. The
                    // style carries the look, not the place.
                    let mut style = match text_style_override {
                        Some(override_style) => override_style.clone(),
                        None => style.clone(),
                    };
                    style.position_x = layer.position_x;
                    style.position_y = layer.position_y;
                    // The text is sized in points against the resolution the
                    // slide was authored against, so an output at another
                    // resolution scales it: the same slide has to look the
                    // same on a 720p screen as on a 1080p one.
                    let scale = self.reference_scale();
                    style.font_size *= scale;
                    style.letter_spacing *= scale;
                    style.glow_radius *= scale;
                    style.text_stroke_width *= scale;
                    // The object's opacity has to reach the pixels: a text
                    // that fades in the editor has to fade on the projector
                    // as well.
                    let opacity = layer.opacity.clamp(0.0, 1.0);
                    style.color = faded(style.color, opacity);
                    style.glow_color = faded(style.glow_color, opacity);
                    style.text_stroke_color = faded(style.text_stroke_color, opacity);
                    let runs: Vec<crate::domain::TextRun> = runs
                        .iter()
                        .map(|run| crate::domain::TextRun {
                            color: run.color.map(|color| faded(color, opacity)),
                            // A run's own size is in points too, so it
                            // scales with the output's resolution as well.
                            font_size: run.font_size.map(|size| size * scale),
                            ..run.clone()
                        })
                        .collect();

                    if let Some(ref gpu) = self.gpu_text
                        && let Ok(mut gpu) = gpu.lock()
                    {
                        match gpu.render_text(text, &style, &runs, self.width, self.height) {
                            Ok(overlay) => {
                                let buf: &mut Vec<u8> = Arc::make_mut(&mut frame.data);
                                GpuTextRenderer::composite_bgra_at(buf, &overlay, self.width);
                                continue;
                            }
                            Err(e) => {
                                log::warn!("GPU text render failed, using pixel font: {e}");
                            }
                        }
                    }
                    let x = (layer.position_x * self.width as f32) as u32;
                    let y = (layer.position_y * self.height as f32) as u32;
                    self.draw_text_bitmap(
                        frame,
                        text,
                        x,
                        y,
                        style.font_size,
                        style.color,
                        style.alignment,
                        style.shadow,
                        style.outline,
                    );
                }
                ObjectContent::Shape {
                    shape_type,
                    fill,
                    stroke_color,
                    stroke_width,
                    corner_radius,
                } => {
                    // The box keeps its place: the object's centre is where
                    // the model says it is, even when the box hangs over the
                    // edge of the slide. The part that hangs over is clipped
                    // when it is drawn; a box that was pushed back onto the
                    // slide would draw the object somewhere else than the
                    // inspector reports.
                    let box_w = (layer.width * self.width as f32).max(1.0);
                    let box_h = (layer.height * self.height as f32).max(1.0);
                    let start_x = ((layer.position_x * self.width as f32) - box_w / 2.0) as i32;
                    let start_y = ((layer.position_y * self.height as f32) - box_h / 2.0) as i32;
                    let end_x = start_x + box_w as i32;
                    let end_y = start_y + box_h as i32;
                    // The stroke and the corner radius are measured in the
                    // resolution the slide was authored against, so they
                    // scale with the output like the type does.
                    let scale = self.reference_scale();
                    self.draw_shape(
                        frame,
                        *shape_type,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                        *fill,
                        *stroke_color,
                        *stroke_width * scale,
                        *corner_radius * scale,
                        layer.opacity,
                        Flip {
                            horizontal: layer.flip_horizontal,
                            vertical: layer.flip_vertical,
                        },
                    );
                }
                ObjectContent::Image { path, fit } => {
                    if !path.is_empty() {
                        // The box keeps its place: the part that hangs over
                        // the edge of the slide is clipped when it is drawn.
                        let w = ((layer.width * self.width as f32).max(1.0)) as u32;
                        let h = ((layer.height * self.height as f32).max(1.0)) as u32;
                        let start_x =
                            ((layer.position_x * self.width as f32) - w as f32 / 2.0) as i32;
                        let start_y =
                            ((layer.position_y * self.height as f32) - h as f32 / 2.0) as i32;
                        self.draw_image_rect(
                            frame,
                            path,
                            start_x,
                            start_y,
                            w,
                            h,
                            *fit,
                            layer.opacity,
                            Flip {
                                horizontal: layer.flip_horizontal,
                                vertical: layer.flip_vertical,
                            },
                        );
                    }
                }
                ObjectContent::Video { .. } => {
                    // A video object fills its own box, the way an image
                    // object does; only a video *background* covers the
                    // whole frame.
                    if show_media && let Some(vf) = video_frame {
                        let width = ((layer.width * self.width as f32).max(1.0)) as u32;
                        let height = ((layer.height * self.height as f32).max(1.0)) as u32;
                        let start_x =
                            ((layer.position_x * self.width as f32) - width as f32 / 2.0) as i32;
                        let start_y =
                            ((layer.position_y * self.height as f32) - height as f32 / 2.0) as i32;
                        self.draw_video_rect(
                            frame,
                            vf,
                            start_x,
                            start_y,
                            width,
                            height,
                            layer.opacity,
                            Flip {
                                horizontal: layer.flip_horizontal,
                                vertical: layer.flip_vertical,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Draw a decoded video frame into a box on the frame.
    #[allow(clippy::too_many_arguments)]
    fn draw_video_rect(
        &mut self,
        frame: &mut Frame,
        vf: &crate::media::decoder::RgbaFrame,
        dest_x: i32,
        dest_y: i32,
        dest_w: u32,
        dest_h: u32,
        opacity: f32,
        flip: Flip,
    ) {
        if dest_w == 0 || dest_h == 0 {
            return;
        }
        let src_w = vf.width as usize;
        let src_h = vf.height as usize;
        if src_w == 0 || src_h == 0 {
            return;
        }
        let buf = Arc::make_mut(&mut frame.data);

        // The picture is clipped to the frame: the part of the box that
        // hangs over the edge is not drawn, but the box keeps its place.
        let clip_x0 = dest_x.max(0);
        let clip_y0 = dest_y.max(0);
        let clip_x = (dest_x + dest_w as i32).min(self.width as i32);
        let clip_y = (dest_y + dest_h as i32).min(self.height as i32);
        for py in clip_y0..clip_y {
            let dy = (py - dest_y) as usize;
            // The clip is turned over about the middle of its box: the row is
            // read from the other side of that middle, so the clip keeps its
            // place and its size on the slide.
            let src_dy = if flip.vertical {
                dest_h as usize - 1 - dy
            } else {
                dy
            };
            let sy = (src_dy * src_h) / dest_h as usize;
            for px in clip_x0..clip_x {
                let dx = (px - dest_x) as usize;
                let src_dx = if flip.horizontal {
                    dest_w as usize - 1 - dx
                } else {
                    dx
                };
                let sx = (src_dx * src_w) / dest_w as usize;
                let src_idx = (sy * src_w + sx) * 4;
                let dst_idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                if src_idx + 3 >= vf.data.len() || dst_idx + 3 >= buf.len() {
                    continue;
                }
                let a = (vf.data[src_idx + 3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let ia = 1.0 - a;
                // The decoder hands the frame over as RGBA and the frame is
                // BGRA, so the red and the blue are swapped on the way in:
                // without this a clip shows its colours reversed.
                buf[dst_idx] = (vf.data[src_idx + 2] as f32 * a + buf[dst_idx] as f32 * ia) as u8;
                buf[dst_idx + 1] =
                    (vf.data[src_idx + 1] as f32 * a + buf[dst_idx + 1] as f32 * ia) as u8;
                buf[dst_idx + 2] =
                    (vf.data[src_idx] as f32 * a + buf[dst_idx + 2] as f32 * ia) as u8;
                buf[dst_idx + 3] = 255;
            }
        }
    }

    /// Draw the overlay: the props, the mask and the live message.
    fn draw_overlay(&mut self, frame: &mut Frame, overlay: &Overlay) {
        for prop in &overlay.props {
            if prop.visible {
                self.draw_prop(frame, prop);
            }
        }
        self.draw_mask(frame, &overlay.mask);
        if let Some(message) = &overlay.live_message
            && message.visible
            && !message.text.trim().is_empty()
        {
            self.draw_live_message(frame, &message.text);
        }
        // The announcement is drawn last: it is the app's own word, and it
        // is the one the room has to read.
        if let Some(announcement) = &overlay.announcement
            && announcement.visible
            && !announcement.text.trim().is_empty()
        {
            self.draw_announcement(frame, &announcement.text);
        }
    }

    /// Draw the announcement across the top of the output.
    ///
    /// The announcement is the app talking to the room, so it is a banner
    /// across the top of the picture in the app's own colour, rather than
    /// the black of the operator's alert.
    fn draw_announcement(&mut self, frame: &mut Frame, text: &str) {
        let bar = Color {
            r: 240,
            g: 55,
            b: 8,
            a: 235,
        };
        self.draw_banner(frame, text, bar);
    }

    /// Draw one prop (a lower third, a logo, …) onto the frame.
    fn draw_prop(&mut self, frame: &mut Frame, prop: &Prop) {
        let start_x = (prop.x * self.width as f32) as u32;
        let start_y = (prop.y * self.height as f32) as u32;
        let width = ((prop.width * self.width as f32).max(1.0)) as u32;
        let height = ((prop.height * self.height as f32).max(1.0)) as u32;

        match &prop.content {
            PropContent::Rectangle {
                color,
                corner_radius,
            } => {
                self.draw_shape(
                    frame,
                    ShapeType::Rectangle,
                    start_x as i32,
                    start_y as i32,
                    start_x as i32 + width as i32,
                    start_y as i32 + height as i32,
                    color_from_rgba(*color),
                    Color::black(),
                    0.0,
                    // The radius is measured in the resolution the slide was
                    // authored against, so it scales with the output like the
                    // type does.
                    corner_radius * self.reference_scale(),
                    1.0,
                    // A prop is not an object on the slide, so it has no
                    // mirror of its own.
                    Flip::default(),
                );
            }
            PropContent::Image { path } => {
                if !path.is_empty() {
                    // A prop has no fit setting: its image fills the box.
                    self.draw_image_rect(
                        frame,
                        path,
                        start_x as i32,
                        start_y as i32,
                        width,
                        height,
                        ImageFit::Stretch,
                        1.0,
                        // A prop is not an object on the slide, so it has no
                        // mirror of its own.
                        Flip::default(),
                    );
                }
            }
            PropContent::Text {
                text,
                font_size,
                color,
                bold,
                italic,
            } => {
                if text.trim().is_empty() {
                    return;
                }
                self.draw_prop_text(
                    frame, text, *font_size, *color, *bold, *italic, prop.x, prop.y,
                );
            }
        }
    }

    /// Draw a prop's text.
    ///
    /// The GPU text renderer anchors the text at the style position, so the
    /// prop's position is used as the anchor point.
    #[allow(clippy::too_many_arguments)]
    fn draw_prop_text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        font_size: f32,
        color: [f32; 4],
        bold: bool,
        italic: bool,
        x: f32,
        y: f32,
    ) {
        let style = TextStyle {
            font_family: "Arial".to_string(),
            // The prop's text is sized in points against the resolution the
            // slide was authored against, so it scales with the output.
            font_size: font_size * self.reference_scale(),
            color: color_from_rgba(color),
            alignment: TextAlignment::Left,
            bold,
            italic,
            position_x: x,
            position_y: y,
            ..Default::default()
        };

        if let Some(ref gpu) = self.gpu_text
            && let Ok(mut gpu) = gpu.lock()
        {
            match gpu.render_text(text, &style, &[], self.width, self.height) {
                Ok(overlay) => {
                    let buf: &mut Vec<u8> = Arc::make_mut(&mut frame.data);
                    GpuTextRenderer::composite_bgra_at(buf, &overlay, self.width);
                    return;
                }
                Err(e) => log::warn!("GPU text render failed for a prop: {e}"),
            }
        }

        // Fallback: the pixel font anchors at the top-left, so the anchor is
        // moved up by half a line to match the GPU renderer's centring.
        let px = (x * self.width as f32) as u32;
        let py = (y * self.height as f32) as u32;
        self.draw_text_bitmap(
            frame,
            text,
            px,
            py,
            // The size is the scaled one: the fallback has to draw the prop's
            // text the size the GPU path draws it.
            style.font_size,
            style.color,
            TextAlignment::Left,
            false,
            false,
        );
    }

    /// Draw the mask that frames the output.
    fn draw_mask(&mut self, frame: &mut Frame, mask: &Mask) {
        match mask {
            Mask::None => {}
            Mask::Letterbox { bar_fraction } => {
                let bar = ((bar_fraction.clamp(0.0, 0.45)) * self.height as f32) as u32;
                if bar == 0 {
                    return;
                }
                for y in 0..bar {
                    self.fill_row(frame, y, Color::black());
                    self.fill_row(frame, self.height - 1 - y, Color::black());
                }
            }
            Mask::Oval { .. } => {
                // The oval mask dims everything outside a centred ellipse.
                // The feather is approximated by a hard edge.
                let cx = self.width as f32 / 2.0;
                let cy = self.height as f32 / 2.0;
                let rx = cx * 0.9;
                let ry = cy * 0.9;
                // The buffer is taken out of the arc once: asking for it
                // per pixel is a refcount check for every pixel of the frame.
                let buf = Arc::make_mut(&mut frame.data);
                for y in 0..self.height {
                    for x in 0..self.width {
                        let dx = (x as f32 - cx) / rx;
                        let dy = (y as f32 - cy) / ry;
                        if dx * dx + dy * dy > 1.0 {
                            Self::blend_pixel(&mut buf[..], self.width, x, y, Color::black(), 1.0);
                        }
                    }
                }
            }
            Mask::FrameBorder { thickness, color } => {
                let t = ((thickness.clamp(0.0, 0.25)) * self.width as f32) as u32;
                if t == 0 {
                    return;
                }
                let border = color_from_rgba(*color);
                // The buffer is taken out of the arc once: asking for it
                // per pixel is a refcount check for every pixel of the frame.
                let buf = Arc::make_mut(&mut frame.data);
                for y in 0..self.height {
                    for x in 0..self.width {
                        if x < t || y < t || x >= self.width - t || y >= self.height - t {
                            Self::blend_pixel(&mut buf[..], self.width, x, y, border, 1.0);
                        }
                    }
                }
            }
            Mask::Custom { path } => {
                // A custom mask is an image: the brighter it is, the more of
                // the output it hides.
                self.draw_custom_mask(frame, path);
            }
        }
    }

    /// Hide the parts of the frame a custom mask image covers.
    ///
    /// The mask image is scaled to the frame; its brightness decides how much
    /// of the output stays visible, so a white mask hides everything and a
    /// black one hides nothing.
    fn draw_custom_mask(&mut self, frame: &mut Frame, path: &str) {
        if path.is_empty() || self.failed_images.contains(path) {
            return;
        }
        let key = (ImageUse::Mask, path.to_string());
        if !self.image_cache.contains(&key) {
            match image::open(path) {
                Ok(img) => {
                    let resized = img
                        .resize_exact(
                            self.width,
                            self.height,
                            image::imageops::FilterType::Triangle,
                        )
                        .to_rgba8();
                    self.image_cache.put(key.clone(), resized);
                }
                Err(e) => {
                    log::error!("custom mask load error '{path}': {e}");
                    self.failed_images.insert(path.to_string());
                    return;
                }
            }
        }
        let Some(mask) = self.image_cache.get(&key) else {
            return;
        };
        let buf = Arc::make_mut(&mut frame.data);
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let pixel = mask.get_pixel(x as u32, y as u32);
                // The brightness of the mask decides how much is hidden.
                let brightness =
                    (f32::from(pixel.0[0]) + f32::from(pixel.0[1]) + f32::from(pixel.0[2]))
                        / (3.0 * 255.0);
                if brightness <= 0.0 {
                    continue;
                }
                let idx = (y * self.width as usize + x) * 4;
                let keep = 1.0 - brightness;
                buf[idx] = (buf[idx] as f32 * keep) as u8;
                buf[idx + 1] = (buf[idx + 1] as f32 * keep) as u8;
                buf[idx + 2] = (buf[idx + 2] as f32 * keep) as u8;
            }
        }
    }

    /// Fill one row of the frame with a colour.
    fn fill_row(&self, frame: &mut Frame, y: u32, color: Color) {
        if y >= self.height {
            return;
        }
        // The buffer is taken out of the arc once: asking for it per pixel
        // is a refcount check for every pixel of the row.
        let buf = Arc::make_mut(&mut frame.data);
        for x in 0..self.width {
            Self::blend_pixel(&mut buf[..], self.width, x, y, color, 1.0);
        }
    }

    /// Draw the live alert banner across the top of the frame.
    fn draw_live_message(&mut self, frame: &mut Frame, text: &str) {
        self.draw_banner(frame, text, Color::black());
    }

    /// Draw a banner across the top of the output with the text in it.
    fn draw_banner(&mut self, frame: &mut Frame, text: &str, bar: Color) {
        let bar_height = (self.height as f32 * 0.09).max(24.0) as u32;
        for y in 0..bar_height.min(self.height) {
            self.fill_row(frame, y, bar);
        }

        let style = TextStyle {
            font_family: "Arial".to_string(),
            font_size: (bar_height as f32 * 0.55).max(12.0),
            color: Color::white(),
            alignment: TextAlignment::Center,
            position_x: 0.5,
            position_y: bar_height as f32 / self.height as f32 / 2.0,
            ..Default::default()
        };

        if let Some(ref gpu) = self.gpu_text
            && let Ok(mut gpu) = gpu.lock()
        {
            match gpu.render_text(text, &style, &[], self.width, self.height) {
                Ok(overlay) => {
                    let buf: &mut Vec<u8> = Arc::make_mut(&mut frame.data);
                    GpuTextRenderer::composite_bgra_at(buf, &overlay, self.width);
                    return;
                }
                Err(e) => log::warn!("GPU text render failed for the live message: {e}"),
            }
        }

        // The fallback anchors at the top-left, so the anchor is moved to the
        // middle of the bar and the text is centred there: the GPU path
        // centres it, and the two have to agree.
        self.draw_text_bitmap(
            frame,
            text,
            (self.width as f32 * 0.5) as u32,
            (bar_height as f32 * 0.5) as u32,
            style.font_size,
            style.color,
            TextAlignment::Center,
            false,
            false,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shape(
        &self,
        frame: &mut Frame,
        shape_type: crate::domain::ShapeType,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
        fill: crate::domain::Color,
        stroke_color: crate::domain::Color,
        stroke_width: f32,
        corner_radius: f32,
        opacity: f32,
        flip: Flip,
    ) {
        if start_x >= end_x || start_y >= end_y {
            return;
        }
        let w = (end_x - start_x) as u32;
        let h = (end_y - start_y) as u32;
        // The stroke cannot be thicker than half the shape: a thicker one
        // would swallow the fill and the shape would be drawn entirely in
        // the stroke colour.
        let stroke_px = (stroke_width.max(0.0) as u32).min(w / 2).min(h / 2);
        let radius = corner_radius
            .max(0.0)
            .min((w as f32 / 2.0).min(h as f32 / 2.0));
        let buf = Arc::make_mut(&mut frame.data);

        // The shape is clipped to the frame: the part that hangs over the
        // edge is not drawn, but the shape keeps its size and its place.
        let clip_x0 = start_x.max(0);
        let clip_y0 = start_y.max(0);
        let clip_x = end_x.min(self.width as i32);
        let clip_y = end_y.min(self.height as i32);
        for y in clip_y0..clip_y {
            for x in clip_x0..clip_x {
                // The mirror is about the shape's own centre: the pixel is
                // filled with what the shape holds on the other side of that
                // centre, so a flipped shape keeps its place and its size.
                let sx = if flip.horizontal {
                    start_x + end_x - 1 - x
                } else {
                    x
                };
                let sy = if flip.vertical {
                    start_y + end_y - 1 - y
                } else {
                    y
                };
                let is_inside = match shape_type {
                    crate::domain::ShapeType::Rectangle | crate::domain::ShapeType::Line => {
                        inside_rounded_rect(sx, sy, start_x, start_y, end_x, end_y, radius)
                    }
                    crate::domain::ShapeType::Ellipse => {
                        let cx = start_x as f32 + w as f32 / 2.0;
                        let cy = start_y as f32 + h as f32 / 2.0;
                        let rx = (w as f32 / 2.0).max(1.0);
                        let ry = (h as f32 / 2.0).max(1.0);
                        let dx = (sx as f32 - cx) / rx;
                        let dy = (sy as f32 - cy) / ry;
                        dx * dx + dy * dy <= 1.0
                    }
                    crate::domain::ShapeType::Triangle => {
                        let norm_x = (sx - start_x) as f32 / w.max(1) as f32;
                        let norm_y = (sy - start_y) as f32 / h.max(1) as f32;
                        norm_y >= 0.0
                            && norm_x >= (1.0 - norm_y) / 2.0
                            && norm_x <= 1.0 - (1.0 - norm_y) / 2.0
                    }
                };
                if !is_inside {
                    continue;
                }
                // The stroke is the rim of the shape, so it is the part that
                // is inside the shape but outside the shape drawn one stroke
                // width smaller.
                let is_border = stroke_px > 0
                    && !inside_rounded_rect(
                        sx,
                        sy,
                        start_x + stroke_px as i32,
                        start_y + stroke_px as i32,
                        (end_x - stroke_px as i32).max(start_x),
                        (end_y - stroke_px as i32).max(start_y),
                        (radius - stroke_px as f32).max(0.0),
                    );
                let col = if is_border { stroke_color } else { fill };
                Self::blend_pixel(buf, self.width, x as u32, y as u32, col, opacity);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_rect(
        &mut self,
        frame: &mut Frame,
        path: &str,
        dest_x: i32,
        dest_y: i32,
        dest_w: u32,
        dest_h: u32,
        fit: ImageFit,
        opacity: f32,
        flip: Flip,
    ) {
        if dest_w == 0 || dest_h == 0 {
            return;
        }
        if self.failed_images.contains(path) {
            return;
        }
        let key = (ImageUse::Object, path.to_string());
        if !self.image_cache.contains(&key) {
            match image::open(path) {
                Ok(img) => {
                    self.image_cache.put(key.clone(), img.to_rgba8());
                }
                Err(e) => {
                    log::error!("draw_image_rect load error '{path}': {e}");
                    self.failed_images.insert(path.to_string());
                    return;
                }
            }
        }
        if let Some(img) = self.image_cache.get(&key) {
            let src_w = img.width() as f32;
            let src_h = img.height() as f32;
            if src_w <= 0.0 || src_h <= 0.0 {
                return;
            }

            // The fit decides how the image lands in its box, and the canvas
            // draws the same way, so the editor and the projector agree:
            //  - `Fit` keeps the whole image inside the box, so the box keeps
            //    its shape and the image is letterboxed.
            //  - `Fill` covers the box and crops the overflow in the middle.
            //  - `Stretch` ignores the aspect ratio and fills the box.
            //  - `Center` keeps the image's own size and centres it.
            let (src_x, src_y, src_w, src_h, origin_x, origin_y, dw, dh) = match fit {
                ImageFit::Stretch => (0.0, 0.0, src_w, src_h, dest_x, dest_y, dest_w, dest_h),
                ImageFit::Fit => {
                    let scale = (dest_w as f32 / src_w).min(dest_h as f32 / src_h);
                    let w = (src_w * scale).round() as u32;
                    let h = (src_h * scale).round() as u32;
                    (
                        0.0,
                        0.0,
                        src_w,
                        src_h,
                        dest_x + (dest_w as i32 - w as i32) / 2,
                        dest_y + (dest_h as i32 - h as i32) / 2,
                        w,
                        h,
                    )
                }
                ImageFit::Fill => {
                    // Cover the box: the image is scaled up until it covers
                    // it, and the overflow is cropped in the middle.
                    let scale = (dest_w as f32 / src_w).max(dest_h as f32 / src_h);
                    let crop_w = dest_w as f32 / scale;
                    let crop_h = dest_h as f32 / scale;
                    (
                        (src_w - crop_w) / 2.0,
                        (src_h - crop_h) / 2.0,
                        crop_w,
                        crop_h,
                        dest_x,
                        dest_y,
                        dest_w,
                        dest_h,
                    )
                }
                ImageFit::Center => {
                    // The image keeps its own size, or is scaled down to fit
                    // the box without changing its shape: the canvas draws
                    // the same way, so the editor and the projector agree.
                    let scale = (dest_w as f32 / src_w).min(dest_h as f32 / src_h).min(1.0);
                    let w = (src_w * scale).round() as u32;
                    let h = (src_h * scale).round() as u32;
                    (
                        0.0,
                        0.0,
                        src_w,
                        src_h,
                        dest_x + (dest_w as i32 - w as i32) / 2,
                        dest_y + (dest_h as i32 - h as i32) / 2,
                        w,
                        h,
                    )
                }
            };
            if dw == 0 || dh == 0 {
                return;
            }

            let fw = self.width;
            let buf = Arc::make_mut(&mut frame.data);
            let max_x = img.width().saturating_sub(1);
            let max_y = img.height().saturating_sub(1);

            // The picture is clipped to the frame: the part of the box that
            // hangs over the edge is not drawn, but the box keeps its place.
            let clip_x0 = origin_x.max(0);
            let clip_y0 = origin_y.max(0);
            let clip_x = (origin_x + dw as i32).min(self.width as i32);
            let clip_y = (origin_y + dh as i32).min(self.height as i32);
            for py in clip_y0..clip_y {
                let row = (py - origin_y) as u32;
                if row >= dh {
                    break;
                }
                // The picture is turned over about the middle of the drawn
                // image: the row is read from the other side of that middle,
                // so the image keeps its place and its size on the slide.
                let src_row = if flip.vertical { dh - 1 - row } else { row };
                let sy = (src_y + (src_row as f32 * src_h / dh as f32)) as u32;
                let sy = sy.min(max_y);
                for px in clip_x0..clip_x {
                    let col = (px - origin_x) as u32;
                    if col >= dw {
                        break;
                    }
                    let src_col = if flip.horizontal { dw - 1 - col } else { col };
                    let sx = (src_x + (src_col as f32 * src_w / dw as f32)) as u32;
                    let sx = sx.min(max_x);
                    let pixel = img.get_pixel(sx, sy);
                    let [r, g, b, a] = pixel.0;
                    Self::blend_pixel(
                        buf,
                        fw,
                        px as u32,
                        py as u32,
                        crate::domain::Color { r, g, b, a },
                        opacity,
                    );
                }
            }
        }
    }

    fn blend_pixel(
        buf: &mut [u8],
        frame_width: u32,
        x: u32,
        y: u32,
        color: crate::domain::Color,
        layer_opacity: f32,
    ) {
        let idx = ((y * frame_width + x) * 4) as usize;
        if idx + 3 >= buf.len() {
            return;
        }
        let alpha = (color.a as f32 / 255.0) * layer_opacity.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        if alpha >= 1.0 {
            buf[idx] = color.b;
            buf[idx + 1] = color.g;
            buf[idx + 2] = color.r;
            buf[idx + 3] = 255;
        } else {
            let ia = 1.0 - alpha;
            buf[idx] = (buf[idx] as f32 * ia + color.b as f32 * alpha) as u8;
            buf[idx + 1] = (buf[idx + 1] as f32 * ia + color.g as f32 * alpha) as u8;
            buf[idx + 2] = (buf[idx + 2] as f32 * ia + color.r as f32 * alpha) as u8;
            buf[idx + 3] = 255;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text_bitmap(
        &self,
        frame: &mut Frame,
        text: &str,
        x: u32,
        y: u32,
        font_size: f32,
        color: crate::domain::Color,
        alignment: TextAlignment,
        shadow: bool,
        outline: bool,
    ) {
        let scale = (font_size / 14.0).max(1.0) as u32;
        let cell_w = 6 * scale;

        let line_px_w = text.chars().count() as u32 * cell_w;
        let draw_x = match alignment {
            TextAlignment::Left => x,
            TextAlignment::Center => x.saturating_sub(line_px_w / 2),
            TextAlignment::Right => x.saturating_sub(line_px_w),
        };
        // The text is centred on the anchor, the way the GPU renderer and
        // the editor draw it: the anchor is the middle of the line, not its
        // top.
        let draw_y = y.saturating_sub(PIXEL_FONT_HEIGHT * scale / 2);

        let buf = Arc::make_mut(&mut frame.data);
        let fw = self.width;

        if shadow {
            Self::rasterise_string(
                buf,
                fw,
                text,
                draw_x + scale,
                draw_y + scale,
                scale,
                cell_w,
                crate::domain::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 180,
                },
            );
        }
        if outline {
            for dx in 0u32..=2 {
                for dy in 0u32..=2 {
                    if dx == 1 && dy == 1 {
                        continue;
                    }
                    Self::rasterise_string(
                        buf,
                        fw,
                        text,
                        draw_x + dx,
                        draw_y + dy,
                        scale,
                        cell_w,
                        crate::domain::Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 200,
                        },
                    );
                }
            }
        }

        Self::rasterise_string(buf, fw, text, draw_x, draw_y, scale, cell_w, color);
    }

    #[allow(clippy::too_many_arguments)]
    fn rasterise_string(
        buf: &mut [u8],
        frame_width: u32,
        text: &str,
        x: u32,
        y: u32,
        scale: u32,
        cell_w: u32,
        color: crate::domain::Color,
    ) {
        for (i, ch) in text.chars().enumerate() {
            let glyph = pixel_font(ch);
            let cx = x + i as u32 * cell_w;
            for (row, &bits) in glyph.iter().enumerate() {
                for col in 0..5u32 {
                    if bits & (1 << (4 - col)) != 0 {
                        let alpha = color.a as f32 / 255.0;
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cx + col * scale + sx;
                                let py = y + row as u32 * scale + sy;
                                let idx = ((py * frame_width + px) * 4) as usize;
                                if idx + 3 >= buf.len() {
                                    continue;
                                }
                                if alpha >= 1.0 {
                                    buf[idx] = color.b;
                                    buf[idx + 1] = color.g;
                                    buf[idx + 2] = color.r;
                                    buf[idx + 3] = 255;
                                } else {
                                    let ia = 1.0 - alpha;
                                    buf[idx] =
                                        (buf[idx] as f32 * ia + color.b as f32 * alpha) as u8;
                                    buf[idx + 1] =
                                        (buf[idx + 1] as f32 * ia + color.g as f32 * alpha) as u8;
                                    buf[idx + 2] =
                                        (buf[idx + 2] as f32 * ia + color.r as f32 * alpha) as u8;
                                    buf[idx + 3] = 255;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// How many frames a transition runs for.
///
/// A cut has no frames: the slide changes straight away.
pub fn transition_length(transition: Transition, fps: f32) -> u32 {
    match transition {
        Transition::Cut => 0,
        Transition::Fade { duration_ms }
        | Transition::Slide { duration_ms }
        | Transition::Dissolve { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Wipe { duration_ms, .. } => transition_steps(duration_ms, fps),
    }
}

/// One frame of a transition.
///
/// The motion is drawn a frame at a time: the player asks for the frame it
/// needs and the two pictures are blended then and there. A transition that
/// worked out all of its frames up front would hold the whole motion in
/// memory — a 1080p fade is fifteen pictures of eight megabytes each — and
/// the slide would change a moment after the operator pressed the key
/// rather than when they did.
pub fn transition_frame_at(
    from: &Frame,
    to: &Frame,
    transition: Transition,
    fps: f32,
    step: u32,
) -> Option<Frame> {
    let steps = transition_length(transition, fps);
    if steps == 0 {
        // A cut: there is no motion to draw.
        return None;
    }
    let t = (step + 1) as f32 / steps as f32;
    Some(match transition {
        Transition::Cut => unreachable!("a cut has no frames"),
        Transition::Fade { .. } => from.blend_over(to, t),
        Transition::Slide { .. } => slide_frame(from, to, t),
        Transition::Dissolve { .. } => dissolve_frame(from, to, t),
        Transition::Push { direction, .. } => push_frame(from, to, direction, t),
        Transition::Zoom { .. } => zoom_frame(from, to, t),
        Transition::Flip { .. } => flip_frame(from, to, steps, step),
        Transition::Clock { .. } => clock_frame(from, to, t),
        Transition::Wipe { angle_deg, .. } => edge_frame(from, to, angle_deg, t),
    })
}

/// Every frame of a transition.
///
/// The player draws the motion a frame at a time, so this is for the tests
/// and for a caller that wants the whole motion at once.
pub fn transition_frames(from: &Frame, to: &Frame, transition: Transition, fps: f32) -> Vec<Frame> {
    let steps = transition_length(transition, fps);
    (0..steps)
        .filter_map(|step| transition_frame_at(from, to, transition, fps, step))
        .collect()
}

/// The old slide slides away and the new one takes its place.
fn slide_frame(from: &Frame, to: &Frame, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    let offset = (w as f32 * (1.0 - t)) as u32;
    frame_from_picker(w, h, |x, y| {
        if x >= offset {
            // The new slide.
            let idx = ((y * w + (x - offset)) * 4) as usize;
            [
                to.data[idx],
                to.data[idx + 1],
                to.data[idx + 2],
                to.data[idx + 3],
            ]
        } else {
            // The old slide, sliding out of the way.
            let src_x = x + (w - offset);
            if src_x < w {
                let idx = ((y * w + src_x) * 4) as usize;
                [
                    from.data[idx],
                    from.data[idx + 1],
                    from.data[idx + 2],
                    from.data[idx + 3],
                ]
            } else {
                [0, 0, 0, 0]
            }
        }
    })
}

/// The number of frames a transition runs for.
fn transition_steps(duration_ms: u64, fps: f32) -> u32 {
    ((duration_ms as f32 / 1000.0) * fps).max(1.0) as u32
}

/// The pixel of a frame at the given position.
///
/// A position outside the frame is black: the transitions draw the two
/// slides next to each other, so a pixel can fall outside one of them.
#[inline]
fn sample(data: &[u8], width: u32, height: u32, x: i64, y: i64) -> [u8; 4] {
    if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
        return [0, 0, 0, 255];
    }
    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]
}

/// Build a frame from a function that says which picture a pixel shows.
fn frame_from_picker(width: u32, height: u32, mut pick: impl FnMut(u32, u32) -> [u8; 4]) -> Frame {
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let pixel = pick(x, y);
            let idx = ((y * width + x) * 4) as usize;
            data[idx..idx + 4].copy_from_slice(&pixel);
        }
    }
    Frame::from_data(width, height, data)
}

/// A stable pseudo-random number for a pixel, between zero and one.
///
/// The dissolve has to look the same every time the transition runs, so the
/// number comes from the pixel's place and not from a random source.
fn dissolve_threshold(x: u32, y: u32) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// The slides dissolve into each other, pixel by pixel.
fn dissolve_frame(from: &Frame, to: &Frame, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    frame_from_picker(w, h, |x, y| {
        if dissolve_threshold(x, y) < t {
            sample(&to.data, w, h, x as i64, y as i64)
        } else {
            sample(&from.data, w, h, x as i64, y as i64)
        }
    })
}

/// The new slide pushes the old one out of the way.
fn push_frame(from: &Frame, to: &Frame, direction: u8, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    // The new slide comes in from the right and pushes the old one to the
    // left; the other way round is the mirror of it.
    let from_right = direction != 0;
    let offset = (w as f32 * t).round() as i64;
    let edge = w as i64 - offset;
    frame_from_picker(w, h, |x, y| {
        let x = x as i64;
        if (from_right && x >= edge) || (!from_right && x < offset) {
            // The new slide.
            let source = if from_right { x - edge } else { x + edge };
            sample(&to.data, w, h, source, y as i64)
        } else {
            // The old slide, pushed out of the way.
            let source = if from_right { x + offset } else { x - offset };
            sample(&from.data, w, h, source, y as i64)
        }
    })
}

/// The new slide grows out of the middle of the old one.
fn zoom_frame(from: &Frame, to: &Frame, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let scale = t.max(0.02);
    frame_from_picker(w, h, |x, y| {
        let source_x = cx + (x as f32 - cx) / scale;
        let source_y = cy + (y as f32 - cy) / scale;
        if source_x >= 0.0 && source_y >= 0.0 && source_x < w as f32 && source_y < h as f32 {
            sample(
                &to.data,
                w,
                h,
                source_x.floor() as i64,
                source_y.floor() as i64,
            )
        } else {
            sample(&from.data, w, h, x as i64, y as i64)
        }
    })
}

/// The old slide turns away and the new one turns in.
fn flip_frame(from: &Frame, to: &Frame, steps: u32, step: u32) -> Frame {
    let (w, h) = (from.width, from.height);
    let cx = w as f32 / 2.0;
    let half = (steps / 2).max(1);
    // The first half turns the old slide away, the second half turns the
    // new one in.
    let (source, scale) = if step < half {
        let progress = (step + 1) as f32 / (half + 1) as f32;
        (&from.data, (1.0 - progress).max(0.0))
    } else {
        let progress = (step - half + 1) as f32 / (steps - half).max(1) as f32;
        (&to.data, progress.min(1.0))
    };
    let scale = scale.max(0.02);
    frame_from_picker(w, h, |x, y| {
        let source_x = cx + (x as f32 - cx) / scale;
        if source_x >= 0.0 && source_x < w as f32 {
            sample(source, w, h, source_x.floor() as i64, y as i64)
        } else {
            [0, 0, 0, 255]
        }
    })
}

/// A hand sweeps around the picture and the new slide comes out behind it.
fn clock_frame(from: &Frame, to: &Frame, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let radius = ((w as f32) * (w as f32) + (h as f32) * (h as f32)).sqrt() / 2.0;
    // The hand starts at twelve o'clock and turns clockwise.
    let hand = t * std::f32::consts::TAU;
    frame_from_picker(w, h, |x, y| {
        let dx = x as f32 - cx;
        let dy = y as f32 - cy;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > radius {
            return sample(&from.data, w, h, x as i64, y as i64);
        }
        // The angle of the pixel, measured clockwise from twelve.
        let mut angle = dx.atan2(-dy);
        if angle < 0.0 {
            angle += std::f32::consts::TAU;
        }
        if angle <= hand {
            sample(&to.data, w, h, x as i64, y as i64)
        } else {
            sample(&from.data, w, h, x as i64, y as i64)
        }
    })
}

/// A straight edge crosses the picture and the new slide comes out behind it.
fn edge_frame(from: &Frame, to: &Frame, angle_deg: i32, t: f32) -> Frame {
    let (w, h) = (from.width, from.height);
    let radians = (angle_deg as f32).to_radians();
    let (dx, dy) = (radians.cos(), radians.sin());
    // The distance the edge travels across the whole picture.
    let low =
        (if dx < 0.0 { w as f32 * dx } else { 0.0 }) + (if dy < 0.0 { h as f32 * dy } else { 0.0 });
    let high =
        (if dx > 0.0 { w as f32 * dx } else { 0.0 }) + (if dy > 0.0 { h as f32 * dy } else { 0.0 });
    let span = (high - low).max(1.0);
    let edge = low + span * t;
    frame_from_picker(w, h, |x, y| {
        let along = (x as f32 + 0.5) * dx + (y as f32 + 0.5) * dy;
        if along < edge {
            sample(&to.data, w, h, x as i64, y as i64)
        } else {
            sample(&from.data, w, h, x as i64, y as i64)
        }
    })
}

/// Convert a normalised `[r, g, b, a]` colour into a domain colour.
fn color_from_rgba(rgba: [f32; 4]) -> Color {
    Color {
        r: (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
        g: (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
        b: (rgba[2].clamp(0.0, 1.0) * 255.0) as u8,
        a: (rgba[3].clamp(0.0, 1.0) * 255.0) as u8,
    }
}

/// Whether a pixel is inside a rectangle with rounded corners.
///
/// The radius is in the same pixels as the rectangle, and a rectangle
/// without a radius contains every pixel inside it.
fn inside_rounded_rect(x: i32, y: i32, x0: i32, y0: i32, x1: i32, y1: i32, radius: f32) -> bool {
    if x < x0 || x >= x1 || y < y0 || y >= y1 {
        return false;
    }
    if radius <= 0.5 {
        return true;
    }
    let r = radius.min(((x1 - x0) as f32 / 2.0).min((y1 - y0) as f32 / 2.0));
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    // The distance outside the inner rectangle, in each direction.
    let fx = if px < x0 as f32 + r {
        x0 as f32 + r - px
    } else if px > x1 as f32 - r {
        px - (x1 as f32 - r)
    } else {
        0.0
    };
    let fy = if py < y0 as f32 + r {
        y0 as f32 + r - py
    } else if py > y1 as f32 - r {
        py - (y1 as f32 - r)
    } else {
        0.0
    };
    fx * fx + fy * fy <= r * r
}

fn composite_rgba_to_bgra(frame: &mut Frame, vf: &crate::media::decoder::RgbaFrame) {
    let dst_w = frame.width as usize;
    let dst_h = frame.height as usize;
    let src_w = vf.width as usize;
    let src_h = vf.height as usize;
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return;
    }
    let buf = Arc::make_mut(&mut frame.data);

    // The video covers the frame, the way the editor draws it: the picture
    // keeps its shape and the overflow is cropped. Stretching it to the frame
    // would show a distorted picture that the editor never showed.
    let scale_x = dst_w as f32 / src_w as f32;
    let scale_y = dst_h as f32 / src_h as f32;
    let scale = scale_x.max(scale_y);
    let visible_w = ((dst_w as f32 / scale).round() as usize).clamp(1, src_w);
    let visible_h = ((dst_h as f32 / scale).round() as usize).clamp(1, src_h);
    let offset_x = src_w.saturating_sub(visible_w) / 2;
    let offset_y = src_h.saturating_sub(visible_h) / 2;

    for dy in 0..dst_h {
        let sy = offset_y + (dy * visible_h / dst_h).min(visible_h - 1);
        for dx in 0..dst_w {
            let sx = offset_x + (dx * visible_w / dst_w).min(visible_w - 1);
            let src_idx = (sy * src_w + sx) * 4;
            let dst_idx = (dy * dst_w + dx) * 4;
            if src_idx + 3 < vf.data.len() && dst_idx + 3 < buf.len() {
                let r = vf.data[src_idx];
                let g = vf.data[src_idx + 1];
                let b = vf.data[src_idx + 2];
                let a = vf.data[src_idx + 3];
                buf[dst_idx] = b;
                buf[dst_idx + 1] = g;
                buf[dst_idx + 2] = r;
                buf[dst_idx + 3] = a;
            }
        }
    }
}

/// The height of the pixel font's glyph, in rows.
const PIXEL_FONT_HEIGHT: u32 = 7;

#[rustfmt::skip]
fn pixel_font(c: char) -> [u8; 7] {
    match c {
        ' ' => [0x00,0x00,0x00,0x00,0x00,0x00,0x00],
        '!' => [0x04,0x04,0x04,0x04,0x00,0x04,0x00],
        '"' => [0x0A,0x0A,0x00,0x00,0x00,0x00,0x00],
        '#' => [0x0A,0x1F,0x0A,0x0A,0x1F,0x0A,0x00],
        '$' => [0x04,0x0F,0x14,0x0E,0x05,0x1E,0x04],
        '%' => [0x18,0x19,0x02,0x04,0x08,0x13,0x03],
        '&' => [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D],
        '\'' => [0x04,0x04,0x00,0x00,0x00,0x00,0x00],
        '(' => [0x02,0x04,0x08,0x08,0x08,0x04,0x02],
        ')' => [0x08,0x04,0x02,0x02,0x02,0x04,0x08],
        '*' => [0x00,0x04,0x15,0x0E,0x15,0x04,0x00],
        '+' => [0x00,0x04,0x04,0x1F,0x04,0x04,0x00],
        ',' => [0x00,0x00,0x00,0x00,0x06,0x04,0x08],
        '-' => [0x00,0x00,0x00,0x1F,0x00,0x00,0x00],
        '.' => [0x00,0x00,0x00,0x00,0x00,0x06,0x00],
        '/' => [0x01,0x02,0x02,0x04,0x08,0x08,0x10],
        '0' => [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E],
        '1' => [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E],
        '2' => [0x0E,0x11,0x01,0x06,0x08,0x10,0x1F],
        '3' => [0x1F,0x02,0x04,0x06,0x01,0x11,0x0E],
        '4' => [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02],
        '5' => [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E],
        '6' => [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E],
        '7' => [0x1F,0x01,0x02,0x04,0x08,0x08,0x08],
        '8' => [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E],
        '9' => [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C],
        ':' => [0x00,0x06,0x00,0x00,0x06,0x00,0x00],
        ';' => [0x00,0x06,0x00,0x00,0x06,0x04,0x08],
        '<' => [0x02,0x04,0x08,0x10,0x08,0x04,0x02],
        '=' => [0x00,0x00,0x1F,0x00,0x1F,0x00,0x00],
        '>' => [0x08,0x04,0x02,0x01,0x02,0x04,0x08],
        '?' => [0x0E,0x11,0x01,0x06,0x04,0x00,0x04],
        '@' => [0x0E,0x11,0x01,0x0D,0x15,0x15,0x0E],
        'A' => [0x0E,0x11,0x11,0x1F,0x11,0x11,0x11],
        'B' => [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E],
        'C' => [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E],
        'D' => [0x1C,0x12,0x11,0x11,0x11,0x12,0x1C],
        'E' => [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F],
        'F' => [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10],
        'G' => [0x0E,0x11,0x10,0x17,0x11,0x11,0x0F],
        'H' => [0x11,0x11,0x11,0x1F,0x11,0x11,0x11],
        'I' => [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E],
        'J' => [0x07,0x02,0x02,0x02,0x12,0x12,0x0C],
        'K' => [0x11,0x12,0x14,0x18,0x14,0x12,0x11],
        'L' => [0x10,0x10,0x10,0x10,0x10,0x10,0x1F],
        'M' => [0x11,0x1B,0x15,0x15,0x11,0x11,0x11],
        'N' => [0x11,0x19,0x15,0x13,0x11,0x11,0x11],
        'O' => [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E],
        'P' => [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10],
        'Q' => [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D],
        'R' => [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11],
        'S' => [0x0F,0x10,0x10,0x0E,0x01,0x01,0x1E],
        'T' => [0x1F,0x04,0x04,0x04,0x04,0x04,0x04],
        'U' => [0x11,0x11,0x11,0x11,0x11,0x11,0x0E],
        'V' => [0x11,0x11,0x11,0x11,0x11,0x0A,0x04],
        'W' => [0x11,0x11,0x11,0x15,0x15,0x1B,0x11],
        'X' => [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11],
        'Y' => [0x11,0x11,0x0A,0x04,0x04,0x04,0x04],
        'Z' => [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F],
        '[' => [0x0E,0x08,0x08,0x08,0x08,0x08,0x0E],
        '\\' => [0x10,0x08,0x08,0x04,0x02,0x02,0x01],
        ']' => [0x0E,0x02,0x02,0x02,0x02,0x02,0x0E],
        '^' => [0x04,0x0A,0x11,0x00,0x00,0x00,0x00],
        '_' => [0x00,0x00,0x00,0x00,0x00,0x00,0x1F],
        '`' => [0x08,0x04,0x00,0x00,0x00,0x00,0x00],
        'a' => [0x00,0x00,0x0E,0x01,0x0F,0x11,0x0F],
        'b' => [0x10,0x10,0x1E,0x11,0x11,0x11,0x1E],
        'c' => [0x00,0x00,0x0E,0x10,0x10,0x11,0x0E],
        'd' => [0x01,0x01,0x0F,0x11,0x11,0x11,0x0F],
        'e' => [0x00,0x00,0x0E,0x11,0x1F,0x10,0x0F],
        'f' => [0x06,0x09,0x08,0x1C,0x08,0x08,0x08],
        'g' => [0x00,0x00,0x0F,0x11,0x0F,0x01,0x0E],
        'h' => [0x10,0x10,0x1C,0x12,0x12,0x12,0x12],
        'i' => [0x00,0x04,0x00,0x0C,0x04,0x04,0x0E],
        'j' => [0x00,0x02,0x00,0x06,0x02,0x12,0x0C],
        'k' => [0x10,0x10,0x12,0x14,0x18,0x14,0x12],
        'l' => [0x0C,0x04,0x04,0x04,0x04,0x04,0x0E],
        'm' => [0x00,0x00,0x1A,0x15,0x15,0x11,0x11],
        'n' => [0x00,0x00,0x1C,0x12,0x12,0x12,0x12],
        'o' => [0x00,0x00,0x0E,0x11,0x11,0x11,0x0E],
        'p' => [0x00,0x00,0x1E,0x11,0x1E,0x10,0x10],
        'q' => [0x00,0x00,0x0F,0x11,0x0F,0x01,0x01],
        'r' => [0x00,0x00,0x16,0x19,0x10,0x10,0x10],
        's' => [0x00,0x00,0x0F,0x10,0x0E,0x01,0x1E],
        't' => [0x08,0x08,0x1C,0x08,0x08,0x09,0x06],
        'u' => [0x00,0x00,0x12,0x12,0x12,0x12,0x0D],
        'v' => [0x00,0x00,0x11,0x11,0x11,0x0A,0x04],
        'w' => [0x00,0x00,0x11,0x11,0x15,0x15,0x0A],
        'x' => [0x00,0x00,0x11,0x0A,0x04,0x0A,0x11],
        'y' => [0x00,0x00,0x11,0x11,0x0F,0x01,0x0E],
        'z' => [0x00,0x00,0x1F,0x02,0x04,0x08,0x1F],
        '{' => [0x02,0x04,0x04,0x08,0x04,0x04,0x02],
        '|' => [0x04,0x04,0x04,0x04,0x04,0x04,0x04],
        '}' => [0x08,0x04,0x04,0x02,0x04,0x04,0x08],
        '~' => [0x00,0x08,0x15,0x02,0x00,0x00,0x00],
        _   => [0x0E,0x11,0x15,0x1B,0x15,0x11,0x0E],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Object, ShapeType, Slide};

    /// The text is sized in points against the resolution the slide was
    /// authored against, so an output at another resolution has to scale it:
    /// the same slide has to look the same on a 720p screen as on a 1080p
    /// one, not a bigger version of it.
    #[test]
    fn the_text_scales_with_the_output_resolution() {
        use crate::domain::{ObjectContent, TextStyle};

        // The width of the text's ink on the frame.
        fn ink_width(frame: &Frame) -> usize {
            let mut left = frame.width as usize;
            let mut right = 0usize;
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let idx = ((y * frame.width + x) * 4) as usize;
                    // The background is black; the text is white.
                    if frame.data[idx] > 40 || frame.data[idx + 1] > 40 {
                        left = left.min(x as usize);
                        right = right.max(x as usize);
                    }
                }
            }
            right.saturating_sub(left)
        }

        let mut slide = Slide::new_text(String::new());
        let mut object = Object::new_text("I".to_string());
        if let ObjectContent::Text { style, .. } = &mut object.content {
            style.font_size = 72.0;
            style.color = crate::domain::Color::white();
            *style = TextStyle {
                font_size: 72.0,
                color: crate::domain::Color::white(),
                ..TextStyle::default()
            };
        }
        object.width = 0.5;
        object.height = 0.5;
        slide.layers = vec![object];

        // At the reference resolution the text is its authored size.
        let mut reference = RenderPipeline::new_software(1920, 1080);
        reference.set_reference_width(1920);
        let full = reference.render_slide(&slide).expect("render");

        // The same slide on a smaller screen, scaled: the text is smaller in
        // pixels, but the same size relative to the frame.
        let mut smaller = RenderPipeline::new_software(1280, 720);
        smaller.set_reference_width(1920);
        let scaled = smaller.render_slide(&slide).expect("render");

        // Without the scaling the text would be the same size in pixels on
        // both, so it would look bigger on the smaller screen.
        let mut unscaled = RenderPipeline::new_software(1280, 720);
        unscaled.set_reference_width(1280);
        let plain = unscaled.render_slide(&slide).expect("render");

        let full_ink = ink_width(&full);
        let scaled_ink = ink_width(&scaled);
        let plain_ink = ink_width(&plain);

        assert!(full_ink > 0, "the text is drawn");
        assert!(
            scaled_ink < plain_ink,
            "the scaled text is smaller in pixels ({scaled_ink} < {plain_ink})"
        );
        // The relative size is what the audience sees, and it is the same on
        // both screens.
        let full_relative = full_ink as f32 / 1920.0;
        let scaled_relative = scaled_ink as f32 / 1280.0;
        assert!(
            (full_relative - scaled_relative).abs() < 0.01,
            "the text keeps its size relative to the frame ({full_relative} vs {scaled_relative})"
        );
    }

    #[test]
    fn frame_new_and_clear() {
        let mut frame = Frame::new(10, 10);
        assert_eq!(frame.width, 10);
        assert_eq!(frame.height, 10);
        assert_eq!(frame.data.len(), 400);

        frame.clear(255, 128, 64, 255);
        // BGRA format: byte 0 is B, 1 is G, 2 is R, 3 is A
        assert_eq!(frame.data[0], 64);
        assert_eq!(frame.data[1], 128);
        assert_eq!(frame.data[2], 255);
        assert_eq!(frame.data[3], 255);
    }

    #[test]
    fn frame_set_pixel() {
        let mut frame = Frame::new(4, 4);
        frame.set_pixel(2, 2, 200, 100, 50, 255);
        let idx = (2 * 4 + 2) * 4;
        assert_eq!(frame.data[idx], 50); // B
        assert_eq!(frame.data[idx + 1], 100); // G
        assert_eq!(frame.data[idx + 2], 200); // R
        assert_eq!(frame.data[idx + 3], 255); // A

        // Out of bounds should be a safe no-op
        frame.set_pixel(10, 10, 0, 0, 0, 0);
    }

    #[test]
    fn frame_blend_over() {
        let mut f1 = Frame::new(2, 2);
        f1.clear(0, 0, 0, 255);
        let mut f2 = Frame::new(2, 2);
        f2.clear(200, 200, 200, 255);

        let blended = f1.blend_over(&f2, 0.5);
        assert_eq!(blended.width, 2);
        assert_eq!(blended.height, 2);
        assert_eq!(blended.data[0], 100);
    }

    #[test]
    fn transition_frames_generation() {
        let f1 = Frame::new(10, 10);
        let f2 = Frame::new(10, 10);

        let cut = transition_frames(&f1, &f2, Transition::Cut, 30.0);
        assert!(cut.is_empty());

        let fade = transition_frames(&f1, &f2, Transition::Fade { duration_ms: 100 }, 30.0);
        assert_eq!(fade.len(), 3);

        let slide = transition_frames(&f1, &f2, Transition::Slide { duration_ms: 100 }, 30.0);
        assert_eq!(slide.len(), 3);
    }

    #[test]
    fn render_pipeline_software_render_slide_with_objects() {
        let mut pipeline = RenderPipeline::new_software(64, 64);
        let black = pipeline.render_black();
        assert_eq!(black.width, 64);
        assert_eq!(black.height, 64);
        assert_eq!(black.data[0], 0);

        let mut slide = Slide::new_text("Hello".to_string());
        slide.layers.push(Object::new_shape(ShapeType::Rectangle));
        let frame = pipeline.render_slide(&slide).unwrap();
        assert_eq!(frame.width, 64);
        assert_eq!(frame.height, 64);
        // The frame should contain rendered non-zero pixel data
        assert!(frame.data.iter().any(|&b| b > 0));
    }

    #[test]
    fn pixel_font_contains_ascii() {
        for c in 'A'..='Z' {
            let glyph = pixel_font(c);
            assert!(glyph.iter().any(|&row| row > 0));
        }
        for c in '0'..='9' {
            let glyph = pixel_font(c);
            assert!(glyph.iter().any(|&row| row > 0));
        }
    }
}

#[cfg(test)]
mod negative_cache_tests {
    use super::*;

    /// A missing image must be recorded as failed so the renderer does not retry
    /// the disk read (and re-log the error) on every single frame.
    #[test]
    fn failed_image_path_is_remembered() {
        let mut pipeline = RenderPipeline::new_software(8, 8);
        let path = "/nonexistent/definitely-missing-image.png";
        assert!(!pipeline.failed_images.contains(path));

        let mut frame = Frame::new(8, 8);
        pipeline.draw_image_rect(
            &mut frame,
            path,
            0,
            0,
            4,
            4,
            ImageFit::Stretch,
            1.0,
            Flip::default(),
        );
        assert!(
            pipeline.failed_images.contains(path),
            "failed path should be cached after the first attempt"
        );

        // Subsequent draws short-circuit without another cache insert or log.
        pipeline.draw_image_rect(
            &mut frame,
            path,
            0,
            0,
            4,
            4,
            ImageFit::Stretch,
            1.0,
            Flip::default(),
        );
        assert!(pipeline.failed_images.contains(path));
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;
    use crate::domain::Transition;

    /// A frame of one colour.
    fn solid(width: u32, height: u32, r: u8, g: u8, b: u8) -> Frame {
        let mut frame = Frame::new(width, height);
        frame.clear(r, g, b, 255);
        frame
    }

    /// The red channel of a pixel.
    fn red_at(frame: &Frame, x: u32, y: u32) -> u8 {
        let idx = ((y * frame.width + x) * 4) as usize;
        frame.data[idx + 2]
    }

    /// Every transition is its own motion.
    ///
    /// The operator picks "Wipe Right" or "Clock" and has to get it: a
    /// transition that is really a cross-fade is a lie in the panel.
    #[test]
    fn the_transitions_are_not_all_a_cross_fade() {
        let from = solid(8, 8, 255, 0, 0);
        let to = solid(8, 8, 0, 0, 255);

        // A cross-fade: the middle frame is a mix of the two colours.
        let fade = transition_frames(&from, &to, Transition::Fade { duration_ms: 1000 }, 10.0);
        let middle = &fade[fade.len() / 2];
        assert_eq!(red_at(middle, 0, 0), red_at(middle, 7, 7));

        // A wipe: the middle frame has the old colour on one side and the
        // new one on the other.
        let wipe = transition_frames(
            &from,
            &to,
            Transition::Wipe {
                duration_ms: 1000,
                angle_deg: 0,
            },
            10.0,
        );
        let middle = &wipe[wipe.len() / 2];
        assert_ne!(
            red_at(middle, 0, 0),
            red_at(middle, 7, 7),
            "the wipe draws an edge, it does not blend"
        );

        // A push: the two slides are next to each other.
        let push = transition_frames(
            &from,
            &to,
            Transition::Push {
                duration_ms: 1000,
                direction: 1,
            },
            10.0,
        );
        let middle = &push[push.len() / 2];
        assert_ne!(
            red_at(middle, 0, 0),
            red_at(middle, 7, 7),
            "the push moves the old slide out of the way"
        );

        // A zoom: the new slide grows out of the middle, so the corners
        // still show the old slide while the centre shows the new one.
        let zoom = transition_frames(&from, &to, Transition::Zoom { duration_ms: 1000 }, 10.0);
        let early = &zoom[zoom.len() / 4];
        assert_eq!(
            red_at(early, 0, 0),
            255,
            "the corner still shows the old slide"
        );
        assert_eq!(red_at(early, 4, 4), 0, "the middle shows the new slide");

        // A dissolve: the two colours are mixed in the same pixel.
        let dissolve =
            transition_frames(&from, &to, Transition::Dissolve { duration_ms: 1000 }, 10.0);
        let middle = &dissolve[dissolve.len() / 2];
        let reds: Vec<u8> = (0..8).map(|y| red_at(middle, 0, y)).collect();
        assert!(
            reds.iter().any(|&r| r > 0) && reds.iter().any(|&r| r < 255),
            "the dissolve mixes the two slides pixel by pixel"
        );
    }

    /// Every transition ends on the new slide.
    #[test]
    fn every_transition_ends_on_the_new_slide() {
        let from = solid(8, 8, 255, 0, 0);
        let to = solid(8, 8, 0, 0, 255);

        for transition in [
            Transition::Dissolve { duration_ms: 500 },
            Transition::Zoom { duration_ms: 500 },
            Transition::Flip { duration_ms: 500 },
            Transition::Clock { duration_ms: 500 },
            Transition::Push {
                duration_ms: 500,
                direction: 1,
            },
            Transition::Wipe {
                duration_ms: 500,
                angle_deg: 90,
            },
            Transition::Fade { duration_ms: 500 },
            Transition::Slide { duration_ms: 500 },
        ] {
            let frames = transition_frames(&from, &to, transition, 10.0);
            let last = frames.last().expect("the transition has frames");
            assert_eq!(
                red_at(last, 4, 4),
                0,
                "{transition:?} ends on the new slide"
            );
        }
    }
}

#[cfg(test)]
mod flip_tests {
    use super::*;
    use crate::domain::{Object, ShapeType, Slide};

    /// The box the test objects are drawn in, in pixels.
    ///
    /// The objects sit in the middle of a 64 by 64 slide and are half of it
    /// wide, so their box is the middle half of the frame.
    const BOX_START: u32 = 16;
    const BOX_END: u32 = 48;

    /// A slide with one shape, in the middle of the slide.
    fn slide_with_shape(flip_horizontal: bool, flip_vertical: bool) -> Slide {
        let mut slide = Slide::new_text(String::new());
        let mut object = Object::new_shape(ShapeType::Triangle);
        object.position_x = 0.5;
        object.position_y = 0.5;
        object.width = 0.5;
        object.height = 0.5;
        object.flip_horizontal = flip_horizontal;
        object.flip_vertical = flip_vertical;
        slide.layers.push(object);
        slide
    }

    /// The pixel a mirrored frame holds at `(x, y)`.
    ///
    /// The mirror is about the middle of the box the object was drawn in, so
    /// the first pixel of the box and the last one swap.
    fn mirrored_pixel(
        frame: &Frame,
        x: u32,
        y: u32,
        horizontal: bool,
        vertical: bool,
    ) -> Option<(u8, u8, u8, u8)> {
        let source_x = if horizontal && (BOX_START..BOX_END).contains(&x) {
            BOX_START + BOX_END - 1 - x
        } else {
            x
        };
        let source_y = if vertical && (BOX_START..BOX_END).contains(&y) {
            BOX_START + BOX_END - 1 - y
        } else {
            y
        };
        frame.pixel(source_x, source_y)
    }

    /// A flipped shape is the shape turned over about its own centre.
    ///
    /// The object keeps its place and its size on the slide: only the
    /// picture inside its box is mirrored.
    #[test]
    fn a_flipped_shape_is_mirrored_about_its_own_centre() {
        let mut pipeline = RenderPipeline::new_software(64, 64);
        let plain = pipeline
            .render_slide(&slide_with_shape(false, false))
            .unwrap();
        let flipped = pipeline
            .render_slide(&slide_with_shape(true, false))
            .unwrap();

        for y in 0..64u32 {
            for x in 0..64u32 {
                assert_eq!(
                    flipped.pixel(x, y),
                    mirrored_pixel(&plain, x, y, true, false),
                    "pixel ({x}, {y})"
                );
            }
        }
        // The triangle is not symmetric, so the mirror has to change it.
        assert_ne!(plain.data, flipped.data);
    }

    /// A shape turned over the other way is mirrored top to bottom.
    #[test]
    fn a_shape_flipped_vertically_is_mirrored_top_to_bottom() {
        let mut pipeline = RenderPipeline::new_software(64, 64);
        let plain = pipeline
            .render_slide(&slide_with_shape(false, false))
            .unwrap();
        let flipped = pipeline
            .render_slide(&slide_with_shape(false, true))
            .unwrap();

        for y in 0..64u32 {
            for x in 0..64u32 {
                assert_eq!(
                    flipped.pixel(x, y),
                    mirrored_pixel(&plain, x, y, false, true),
                    "pixel ({x}, {y})"
                );
            }
        }
        assert_ne!(plain.data, flipped.data);
    }

    /// A flipped image is the picture turned over in its own box.
    #[test]
    fn a_flipped_image_is_mirrored_about_its_own_centre() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("flip.png");
        // A picture that is not symmetric: the left half is white, the right
        // half is black.
        let mut source = image::RgbaImage::new(4, 2);
        for (x, _y, pixel) in source.enumerate_pixels_mut() {
            *pixel = if x < 2 {
                image::Rgba([255, 255, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 255])
            };
        }
        source.save(&path).unwrap();

        let mut slide = Slide::new_text(String::new());
        let mut object = Object::new_image(path.to_string_lossy().to_string(), ImageFit::Stretch);
        object.position_x = 0.5;
        object.position_y = 0.5;
        object.width = 0.5;
        object.height = 0.5;
        slide.layers.push(object);
        let plain = render_with_flip(&slide, false);
        let flipped = render_with_flip(&slide, true);

        for y in 0..64u32 {
            for x in 0..64u32 {
                assert_eq!(
                    flipped.pixel(x, y),
                    mirrored_pixel(&plain, x, y, true, false),
                    "pixel ({x}, {y})"
                );
            }
        }
        assert_ne!(plain.data, flipped.data);
    }

    /// Render a slide with its image object turned over, or not.
    fn render_with_flip(slide: &Slide, flip_horizontal: bool) -> Frame {
        let mut pipeline = RenderPipeline::new_software(64, 64);
        let mut slide = slide.clone();
        slide.layers[0].flip_horizontal = flip_horizontal;
        pipeline.render_slide(&slide).unwrap()
    }
}
