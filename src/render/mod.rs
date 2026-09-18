use crate::Result;
use crate::domain::{Background, ObjectContent, Slide, TextAlignment, Transition};
use std::num::NonZeroUsize;
use std::sync::Arc;

pub mod gpu_text;
pub use gpu_text::{GpuTextRenderer, create_gpu_text_renderer};

const IMAGE_CACHE_SIZE: usize = 16;

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

pub struct RenderPipeline {
    pub width: u32,
    pub height: u32,
    gpu_text: Option<GpuTextRenderer>,
    image_cache: lru::LruCache<String, image::RgbaImage>,
}

impl RenderPipeline {
    pub fn new(width: u32, height: u32) -> Self {
        let gpu_text = match create_gpu_text_renderer() {
            Ok(r) => {
                println!("✓ GPU text renderer initialised (glyphon)");
                Some(r)
            }
            Err(e) => {
                eprintln!("⚠ GPU text renderer unavailable, using pixel font: {}", e);
                None
            }
        };
        let cache_cap = NonZeroUsize::new(IMAGE_CACHE_SIZE).expect("cache size > 0");
        Self {
            width,
            height,
            gpu_text,
            image_cache: lru::LruCache::new(cache_cap),
        }
    }

    pub fn new_software(width: u32, height: u32) -> Self {
        let cache_cap = NonZeroUsize::new(IMAGE_CACHE_SIZE).expect("cache size > 0");
        Self {
            width,
            height,
            gpu_text: None,
            image_cache: lru::LruCache::new(cache_cap),
        }
    }

    pub fn render_slide(&mut self, slide: &Slide) -> Result<Frame> {
        self.render_slide_with_video(slide, None)
    }

    pub fn render_slide_with_video(
        &mut self,
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
    ) -> Result<Frame> {
        let mut frame = Frame::new(self.width, self.height);
        self.draw_background(&mut frame, slide, video_frame);
        self.draw_content(&mut frame, slide, video_frame);
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
        slide: &Slide,
        video_frame: Option<&crate::media::decoder::RgbaFrame>,
    ) {
        match &slide.background {
            Background::Solid(color) => {
                frame.clear(color.r, color.g, color.b, 255);
            }
            Background::Image(path) if !path.is_empty() => {
                if !self.image_cache.contains(path) {
                    match image::open(path) {
                        Ok(img) => {
                            let resized = img
                                .resize_exact(
                                    self.width,
                                    self.height,
                                    image::imageops::FilterType::Triangle,
                                )
                                .to_rgba8();
                            self.image_cache.put(path.clone(), resized);
                        }
                        Err(e) => {
                            eprintln!("background image load error '{path}': {e}");
                            frame.clear(0, 0, 0, 255);
                            return;
                        }
                    }
                }
                if let Some(img) = self.image_cache.get(path) {
                    let buf = Arc::make_mut(&mut frame.data);
                    for (y, row) in img.rows().enumerate() {
                        for (x, pixel) in row.enumerate() {
                            let [r, g, b, a] = pixel.0;
                            let alpha = a as f32 / 255.0;
                            let idx = ((y as u32 * self.width + x as u32) * 4) as usize;
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
                if let Some(vf) = video_frame {
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
    ) {
        let mut layers = slide.effective_layers().into_owned();
        layers.sort_by_key(|l| l.z_order);

        for layer in &layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            match &layer.content {
                ObjectContent::Text { text, style, .. } => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    if let Some(ref mut gpu) = self.gpu_text {
                        match gpu.render_text(text, style, &[], self.width, self.height) {
                            Ok(overlay) => {
                                let buf: &mut Vec<u8> = Arc::make_mut(&mut frame.data);
                                GpuTextRenderer::composite_bgra(buf, &overlay);
                                continue;
                            }
                            Err(e) => {
                                eprintln!("⚠ GPU text render failed, using pixel font: {}", e);
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
                    ..
                } => {
                    let start_x = ((layer.position_x - layer.width / 2.0).max(0.0)
                        * self.width as f32) as u32;
                    let start_y = ((layer.position_y - layer.height / 2.0).max(0.0)
                        * self.height as f32) as u32;
                    let end_x = ((layer.position_x + layer.width / 2.0).min(1.0)
                        * self.width as f32) as u32;
                    let end_y = ((layer.position_y + layer.height / 2.0).min(1.0)
                        * self.height as f32) as u32;
                    self.draw_shape(
                        frame,
                        *shape_type,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                        *fill,
                        *stroke_color,
                        *stroke_width,
                        layer.opacity,
                    );
                }
                ObjectContent::Image { path, .. } => {
                    if !path.is_empty() {
                        let start_x = ((layer.position_x - layer.width / 2.0).max(0.0)
                            * self.width as f32) as u32;
                        let start_y = ((layer.position_y - layer.height / 2.0).max(0.0)
                            * self.height as f32) as u32;
                        let w = ((layer.width * self.width as f32).max(1.0)) as u32;
                        let h = ((layer.height * self.height as f32).max(1.0)) as u32;
                        self.draw_image_rect(frame, path, start_x, start_y, w, h, layer.opacity);
                    }
                }
                ObjectContent::Video { .. } => {
                    if let Some(vf) = video_frame {
                        composite_rgba_to_bgra(frame, vf);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shape(
        &self,
        frame: &mut Frame,
        shape_type: crate::domain::ShapeType,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
        fill: crate::domain::Color,
        stroke_color: crate::domain::Color,
        stroke_width: f32,
        opacity: f32,
    ) {
        if start_x >= end_x || start_y >= end_y {
            return;
        }
        let w = end_x - start_x;
        let h = end_y - start_y;
        let stroke_px = stroke_width.max(0.0) as u32;
        let buf = Arc::make_mut(&mut frame.data);

        for y in start_y..end_y {
            for x in start_x..end_x {
                let is_inside = match shape_type {
                    crate::domain::ShapeType::Rectangle | crate::domain::ShapeType::Line => true,
                    crate::domain::ShapeType::Ellipse => {
                        let cx = start_x as f32 + w as f32 / 2.0;
                        let cy = start_y as f32 + h as f32 / 2.0;
                        let rx = (w as f32 / 2.0).max(1.0);
                        let ry = (h as f32 / 2.0).max(1.0);
                        let dx = (x as f32 - cx) / rx;
                        let dy = (y as f32 - cy) / ry;
                        dx * dx + dy * dy <= 1.0
                    }
                    crate::domain::ShapeType::Triangle => {
                        let norm_x = (x - start_x) as f32 / w.max(1) as f32;
                        let norm_y = (y - start_y) as f32 / h.max(1) as f32;
                        norm_y >= 0.0
                            && norm_x >= (1.0 - norm_y) / 2.0
                            && norm_x <= 1.0 - (1.0 - norm_y) / 2.0
                    }
                };
                if !is_inside {
                    continue;
                }
                let is_border = stroke_px > 0
                    && (x < start_x + stroke_px
                        || x + stroke_px >= end_x
                        || y < start_y + stroke_px
                        || y + stroke_px >= end_y);
                let col = if is_border { stroke_color } else { fill };
                Self::blend_pixel(buf, self.width, x, y, col, opacity);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_rect(
        &mut self,
        frame: &mut Frame,
        path: &str,
        dest_x: u32,
        dest_y: u32,
        dest_w: u32,
        dest_h: u32,
        opacity: f32,
    ) {
        if dest_w == 0 || dest_h == 0 {
            return;
        }
        if !self.image_cache.contains(path) {
            match image::open(path) {
                Ok(img) => {
                    self.image_cache.put(path.to_string(), img.to_rgba8());
                }
                Err(e) => {
                    eprintln!("draw_image_rect load error '{path}': {e}");
                    return;
                }
            }
        }
        if let Some(img) = self.image_cache.get(path) {
            let src_w = img.width() as usize;
            let src_h = img.height() as usize;
            let fw = self.width;
            let buf = Arc::make_mut(&mut frame.data);

            for dy in 0..dest_h {
                let py = dest_y + dy;
                if py >= self.height {
                    break;
                }
                let sy = (dy as usize * src_h) / dest_h as usize;
                for dx in 0..dest_w {
                    let px = dest_x + dx;
                    if px >= self.width {
                        break;
                    }
                    let sx = (dx as usize * src_w) / dest_w as usize;
                    let pixel = img.get_pixel(sx as u32, sy as u32);
                    let [r, g, b, a] = pixel.0;
                    Self::blend_pixel(
                        buf,
                        fw,
                        px,
                        py,
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

        let buf = Arc::make_mut(&mut frame.data);
        let fw = self.width;

        if shadow {
            Self::rasterise_string(
                buf,
                fw,
                text,
                draw_x + scale,
                y + scale,
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
                        y + dy,
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

        Self::rasterise_string(buf, fw, text, draw_x, y, scale, cell_w, color);
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

pub fn transition_frames(from: &Frame, to: &Frame, transition: Transition, fps: f32) -> Vec<Frame> {
    match transition {
        Transition::Cut => vec![],
        Transition::Fade { duration_ms } => {
            let total = ((duration_ms as f32 / 1000.0) * fps).max(1.0) as u32;
            (0..total)
                .map(|i| {
                    let t = (i + 1) as f32 / total as f32;
                    from.blend_over(to, t)
                })
                .collect()
        }
        Transition::Slide { duration_ms } => {
            let total = ((duration_ms as f32 / 1000.0) * fps).max(1.0) as u32;
            let w = from.width;
            let h = from.height;
            (0..total)
                .map(|i| {
                    let t = (i + 1) as f32 / total as f32;
                    let offset = (w as f32 * (1.0 - t)) as u32;
                    let mut out_data = vec![0u8; (w * h * 4) as usize];
                    for y in 0..h {
                        for x in 0..w {
                            let idx = ((y * w + x) * 4) as usize;
                            if x >= offset {
                                let src_idx = ((y * w + (x - offset)) * 4) as usize;
                                out_data[idx..idx + 4]
                                    .copy_from_slice(&to.data[src_idx..src_idx + 4]);
                            } else {
                                let src_x = x + (w - offset);
                                if src_x < w {
                                    let src_idx = ((y * w + src_x) * 4) as usize;
                                    out_data[idx..idx + 4]
                                        .copy_from_slice(&from.data[src_idx..src_idx + 4]);
                                }
                            }
                        }
                    }
                    Frame::from_data(w, h, out_data)
                })
                .collect()
        }
        Transition::Dissolve { duration_ms }
        | Transition::Zoom { duration_ms }
        | Transition::Flip { duration_ms }
        | Transition::Clock { duration_ms }
        | Transition::Push { duration_ms, .. }
        | Transition::Wipe { duration_ms, .. } => {
            let total = ((duration_ms as f32 / 1000.0) * fps).max(1.0) as u32;
            (0..total)
                .map(|i| {
                    let t = (i + 1) as f32 / total as f32;
                    from.blend_over(to, t)
                })
                .collect()
        }
    }
}

fn composite_rgba_to_bgra(frame: &mut Frame, vf: &crate::media::decoder::RgbaFrame) {
    let dst_w = frame.width as usize;
    let dst_h = frame.height as usize;
    let src_w = vf.width as usize;
    let src_h = vf.height as usize;
    let buf = Arc::make_mut(&mut frame.data);

    for dy in 0..dst_h {
        let sy = dy * src_h / dst_h;
        for dx in 0..dst_w {
            let sx = dx * src_w / dst_w;
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
