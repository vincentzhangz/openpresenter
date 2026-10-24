use crate::Result;
use crate::domain::{Ligatures, TextAlignment, TextRun, TextStyle};
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};
use wgpu::{
    BufferUsages, CommandEncoderDescriptor, Device, Extent3d, LoadOp, MultisampleState, Operations,
    Queue, RenderPassColorAttachment, RenderPassDescriptor, StoreOp, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

#[inline]
fn align_up(n: u32, align: u32) -> u32 {
    (n + align - 1) & !(align - 1)
}

/// The region of the frame a piece of text is drawn into.
///
/// The text is drawn into a texture of its own size, so only the glyphs and
/// their effects are copied back. The region is clamped to the frame, and
/// it is the region's own origin that the overlay is composited at.
pub(crate) fn text_region(
    left: i32,
    top: i32,
    text_w: u32,
    text_h: u32,
    margin: i32,
    width: u32,
    height: u32,
) -> (u32, u32, u32, u32) {
    let region_left = (left - margin).max(0);
    let region_top = (top - margin).max(0);
    let region_right = (left + text_w as i32 + margin).min(width as i32);
    let region_bottom = (top + text_h as i32 + margin).min(height as i32);
    let region_w = (region_right - region_left).max(0) as u32;
    let region_h = (region_bottom - region_top).max(0) as u32;
    (region_left as u32, region_top as u32, region_w, region_h)
}

/// A rendered piece of text, and where it sits in the frame.
///
/// The text is rendered into a texture of its own size rather than the whole
/// frame. A full-frame readback per text object means megabytes of copying
/// for every line on the slide, and that copying is what makes a 4K output
/// stutter.
pub struct TextOverlay {
    /// The overlay's left edge in the frame.
    pub origin_x: u32,
    /// The overlay's top edge in the frame.
    pub origin_y: u32,
    /// The overlay's width in pixels.
    pub width: u32,
    /// The overlay's height in pixels.
    pub height: u32,
    /// The overlay's pixels, in BGRA order.
    pub pixels: Vec<u8>,
}

pub struct GpuTextRenderer {
    device: Device,
    queue: Queue,
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    atlas: TextAtlas,
    render_count: u32,
}

impl GpuTextRenderer {
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!("No suitable GPU adapter found: {:?}", e))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let atlas = TextAtlas::new(&device, &queue, &cache, TextureFormat::Rgba8Unorm);

        Ok(Self {
            device,
            queue,
            font_system,
            swash_cache,
            cache,
            atlas,
            render_count: 0,
        })
    }

    pub fn render_text(
        &mut self,
        text: &str,
        style: &TextStyle,
        runs: &[TextRun],
        width: u32,
        height: u32,
    ) -> Result<TextOverlay> {
        let format = TextureFormat::Rgba8Unorm;

        self.render_count = self.render_count.wrapping_add(1);
        if self.render_count.is_multiple_of(256) {
            self.atlas.trim();
        }
        let mut text_renderer = TextRenderer::new(
            &mut self.atlas,
            &self.device,
            MultisampleState::default(),
            None,
        );

        let display_text: String = style.text_transform.apply(text);

        // A superscript and a subscript are notes: they are drawn smaller and
        // shifted off the line, so the size and the offset come from the
        // baseline and not from the font size.
        let size_scale = style.baseline.size_scale();
        let font_size = style.font_size * size_scale;
        let line_height = font_size * style.line_height_multiplier;
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(font_size, line_height));
        buffer.set_size(Some(width as f32), Some(height as f32));

        let base_family_name = style.font_family.as_str();
        let base_family = if base_family_name.is_empty() || base_family_name == "Arial" {
            Family::SansSerif
        } else {
            Family::Name(base_family_name)
        };
        // The style carries the look of the text as a whole; a run may
        // override parts of it.
        let base_color = Color::rgba(style.color.r, style.color.g, style.color.b, style.color.a);
        let base_weight = if style.bold {
            Weight::BOLD
        } else {
            Weight::NORMAL
        };
        let base_style = if style.italic {
            Style::Italic
        } else {
            Style::Normal
        };
        let mut base_attrs = Attrs::new()
            .family(base_family)
            .color(base_color)
            .weight(base_weight)
            .style(base_style);
        if style.letter_spacing != 0.0 {
            base_attrs = base_attrs.letter_spacing(style.letter_spacing);
        }

        // Ligatures are the font's own joined letters. "Use None" draws the
        // letters on their own, which is what basic shaping does.
        let shaping = match style.ligatures {
            Ligatures::Default => Shaping::Advanced,
            Ligatures::None => Shaping::Basic,
        };

        if !runs.is_empty() {
            let transformed_runs: Vec<(String, Attrs<'_>)> = runs
                .iter()
                .map(|r| {
                    let txt = style.text_transform.apply(&r.text);
                    let run_family_name = r.font_family.as_deref().unwrap_or(base_family_name);
                    let run_family = if run_family_name.is_empty() || run_family_name == "Arial" {
                        Family::SansSerif
                    } else {
                        Family::Name(run_family_name)
                    };
                    let mut attrs = Attrs::new()
                        .family(run_family)
                        .color(base_color)
                        .weight(base_weight)
                        .style(base_style);
                    if style.letter_spacing != 0.0 {
                        attrs = attrs.letter_spacing(style.letter_spacing);
                    }

                    if let Some(bold) = r.bold {
                        attrs = attrs.weight(if bold { Weight::BOLD } else { Weight::NORMAL });
                    }
                    if let Some(italic) = r.italic {
                        attrs = attrs.style(if italic { Style::Italic } else { Style::Normal });
                    }
                    if let Some(color) = r.color {
                        attrs = attrs.color(Color::rgba(color.r, color.g, color.b, color.a));
                    }
                    if let Some(run_size) = r.font_size {
                        let run_size = run_size * size_scale;
                        attrs = attrs.metrics(Metrics::new(
                            run_size,
                            run_size * style.line_height_multiplier,
                        ));
                    }
                    (txt, attrs)
                })
                .collect();
            if style.text_transform.is_small_caps() {
                let small = small_caps_segments(
                    &transformed_runs
                        .iter()
                        .map(|(s, a)| (s.as_str(), a.clone()))
                        .collect::<Vec<_>>(),
                );
                buffer.set_rich_text(
                    small.iter().map(|(s, a)| (*s, a.clone())),
                    &base_attrs,
                    shaping,
                    None,
                );
            } else {
                buffer.set_rich_text(
                    transformed_runs
                        .iter()
                        .map(|(s, a)| (s.as_str(), a.clone())),
                    &base_attrs,
                    shaping,
                    None,
                );
            }
        } else if style.text_transform.is_small_caps() {
            // Small caps are capitals drawn smaller: the first letter of the
            // text keeps its full size and the rest of the letters are drawn
            // as small capitals.
            let segments = small_caps_segments(&[(&display_text, base_attrs.clone())]);
            buffer.set_rich_text(
                segments.iter().map(|(s, a)| (*s, a.clone())),
                &base_attrs,
                shaping,
                None,
            );
        } else {
            buffer.set_text(&display_text, &base_attrs, shaping, None);
        }
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (text_w, text_h, baseline) = measure_buffer(&buffer);

        let center_x = (style.position_x * width as f32) as i32;
        let center_y = (style.position_y * height as f32) as i32;

        let left = match style.alignment {
            TextAlignment::Left => center_x,
            TextAlignment::Center => center_x - (text_w / 2) as i32,
            TextAlignment::Right => center_x - text_w as i32,
        };
        // A raised or lowered baseline moves the glyphs off the line they
        // sit on, without moving the line itself.
        let baseline_shift = -(style.baseline.offset() * style.font_size).round() as i32;
        let top = center_y - (text_h / 2) as i32 + baseline_shift;

        // The text is rendered into a texture of its own size, not of the
        // whole frame: only the glyphs and their effects are copied back, so
        // the cost of a line of text does not grow with the size of the
        // screen.
        let stroke_width = style.text_stroke_width.max(0.0).round() as i32;
        let shadow_offset = ((style.font_size * 0.06).round() as i32).max(1);
        let glow_radius = if style.glow_enabled {
            style.glow_radius.max(1.0).round() as i32
        } else {
            0
        };
        // The effects reach around the glyphs, so the region has to hold
        // them.
        let margin = stroke_width.max(glow_radius + 1).max(shadow_offset).max(1);
        if text_w == 0 || text_h == 0 {
            return Ok(TextOverlay {
                origin_x: 0,
                origin_y: 0,
                width: 0,
                height: 0,
                pixels: Vec::new(),
            });
        }
        let (region_left, region_top, region_w, region_h) =
            text_region(left, top, text_w, text_h, margin, width, height);
        if region_w == 0 || region_h == 0 {
            return Ok(TextOverlay {
                origin_x: 0,
                origin_y: 0,
                width: 0,
                height: 0,
                pixels: Vec::new(),
            });
        }

        // The viewport maps the region onto the texture, so the text is
        // placed relative to the region's own origin.
        let mut region_viewport = Viewport::new(&self.device, &self.cache);
        region_viewport.update(
            &self.queue,
            Resolution {
                width: region_w,
                height: region_h,
            },
        );

        let gc = glyphon::Color::rgba(style.color.r, style.color.g, style.color.b, style.color.a);
        text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &region_viewport,
                [TextArea {
                    buffer: &buffer,
                    left: (left - region_left as i32) as f32,
                    top: (top - region_top as i32) as f32,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: region_w as i32,
                        bottom: region_h as i32,
                    },
                    default_color: gc,
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            )
            .map_err(|e| anyhow::anyhow!("glyphon prepare error: {:?}", e))?;

        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("text_render_target"),
            size: Extent3d {
                width: region_w,
                height: region_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            text_renderer
                .render(&self.atlas, &region_viewport, &mut pass)
                .map_err(|e| anyhow::anyhow!("glyphon render error: {:?}", e))?;
        }

        let bytes_per_pixel = 4u32;
        let unpadded_row = region_w * bytes_per_pixel;
        let padded_row = align_up(unpadded_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_readback"),
            size: (padded_row * region_h) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &readback_buf,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(region_h),
                },
            },
            Extent3d {
                width: region_w,
                height: region_h,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let buf_slice = readback_buf.slice(..);
        buf_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| anyhow::anyhow!("wgpu poll error: {:?}", e))?;

        let mapped = buf_slice
            .get_mapped_range()
            .map_err(|e| anyhow::anyhow!("wgpu map error: {:?}", e))?;

        let mut out = vec![0u8; (region_w * region_h * 4) as usize];
        for row in 0..region_h {
            let src_start = (row * padded_row) as usize;
            let src_row = &mapped[src_start..src_start + unpadded_row as usize];
            let dst_start = (row * unpadded_row) as usize;
            let dst_row = &mut out[dst_start..dst_start + unpadded_row as usize];
            for px in 0..region_w as usize {
                // The glyphs are drawn with alpha blending onto a transparent
                // texture, so their colour comes back premultiplied. It is
                // divided out here: everything downstream — the effects, the
                // merge and the composite — works with straight alpha, and a
                // premultiplied buffer would darken every edge twice.
                let alpha = src_row[px * 4 + 3] as u32;
                let unpremultiply = 255u32.checked_div(alpha).unwrap_or(0);
                dst_row[px * 4] = ((src_row[px * 4 + 2] as u32 * unpremultiply).min(255)) as u8;
                dst_row[px * 4 + 1] = ((src_row[px * 4 + 1] as u32 * unpremultiply).min(255)) as u8;
                dst_row[px * 4 + 2] = ((src_row[px * 4] as u32 * unpremultiply).min(255)) as u8;
                dst_row[px * 4 + 3] = alpha as u8;
            }
        }
        drop(mapped);
        readback_buf.unmap();

        // The underline and the strikethrough are lines, not glyphs: the text
        // renderer does not draw them, so they are drawn into the overlay
        // themselves. The underline sits just under the baseline and the
        // strikethrough about half a letter above it.
        if style.underline || style.strikethrough {
            let rule_color = (style.color.r, style.color.g, style.color.b, style.color.a);
            let thickness = (style.font_size * 0.06).round().max(1.0) as i32;
            let text_left = left - region_left as i32;
            let text_right = text_left + text_w as i32;
            // The baseline is the top of the text box plus the ascent of the
            // line: the rules hang off it.
            let baseline_y = (top - region_top as i32) + baseline.round() as i32;
            if style.underline {
                draw_text_rule(
                    &mut out,
                    region_w,
                    region_h,
                    text_left,
                    text_right,
                    baseline_y + (style.font_size * 0.12).round() as i32,
                    thickness,
                    rule_color,
                );
            }
            if style.strikethrough {
                draw_text_rule(
                    &mut out,
                    region_w,
                    region_h,
                    text_left,
                    text_right,
                    baseline_y - (style.font_size * 0.28).round() as i32,
                    thickness,
                    rule_color,
                );
            }
        }

        // The shadow, the outline, the stroke and the glow go under the
        // glyphs, so they are merged into the overlay rather than composited
        // separately. The effects work in the region's own space, which is
        // where the glyphs are.
        let stroke_width = style.text_stroke_width.max(0.0).round() as i32;
        if style.shadow || style.outline || stroke_width > 0 || style.glow_enabled {
            let mut under = vec![0u8; out.len()];

            if style.shadow {
                let offset = ((style.font_size * 0.06).round() as i32).max(1);
                let shadow = offset_overlay(&out, region_w, region_h, offset, offset, 0.7, BLACK);
                merge_over(&mut under, &shadow);
            }

            if style.outline {
                for (dx, dy) in [
                    (-1, -1),
                    (-1, 0),
                    (-1, 1),
                    (0, -1),
                    (0, 1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                ] {
                    let edge = offset_overlay(&out, region_w, region_h, dx, dy, 1.0, BLACK);
                    merge_over(&mut under, &edge);
                }
            }

            if stroke_width > 0 {
                let tint = bgra(
                    style.text_stroke_color.r,
                    style.text_stroke_color.g,
                    style.text_stroke_color.b,
                    style.text_stroke_color.a,
                );
                for (dx, dy) in stroke_offsets(stroke_width) {
                    let edge = offset_overlay(&out, region_w, region_h, dx, dy, 1.0, tint);
                    merge_over(&mut under, &edge);
                }
            }

            if style.glow_enabled {
                let tint = bgra(
                    style.glow_color.r,
                    style.glow_color.g,
                    style.glow_color.b,
                    style.glow_color.a,
                );
                let glow = glow_overlay(
                    &out,
                    region_w,
                    region_h,
                    // The glyphs' place in the region.
                    left - region_left as i32,
                    top - region_top as i32,
                    text_w,
                    text_h,
                    style.glow_radius.max(1.0).round() as i32,
                    tint,
                );
                merge_over(&mut under, &glow);
            }

            merge_over(&mut under, &out);
            return Ok(TextOverlay {
                origin_x: region_left,
                origin_y: region_top,
                width: region_w,
                height: region_h,
                pixels: under,
            });
        }

        Ok(TextOverlay {
            origin_x: region_left,
            origin_y: region_top,
            width: region_w,
            height: region_h,
            pixels: out,
        })
    }

    /// Composite an overlay into a frame at its own origin.
    pub fn composite_bgra_at(base: &mut [u8], overlay: &TextOverlay, frame_width: u32) {
        if overlay.width == 0 || overlay.height == 0 {
            return;
        }
        for row in 0..overlay.height {
            let dst_y = overlay.origin_y + row;
            if dst_y >= base.len() as u32 / 4 / frame_width.max(1) {
                break;
            }
            for col in 0..overlay.width {
                let dst_x = overlay.origin_x + col;
                let src_idx = ((row * overlay.width + col) * 4) as usize;
                let dst_idx = ((dst_y * frame_width + dst_x) * 4) as usize;
                if src_idx + 3 >= overlay.pixels.len() || dst_idx + 3 >= base.len() {
                    continue;
                }
                let a = overlay.pixels[src_idx + 3] as f32 / 255.0;
                if a <= 0.0 {
                    continue;
                }
                let ia = 1.0 - a;
                base[dst_idx] =
                    (base[dst_idx] as f32 * ia + overlay.pixels[src_idx] as f32 * a) as u8;
                base[dst_idx + 1] =
                    (base[dst_idx + 1] as f32 * ia + overlay.pixels[src_idx + 1] as f32 * a) as u8;
                base[dst_idx + 2] =
                    (base[dst_idx + 2] as f32 * ia + overlay.pixels[src_idx + 2] as f32 * a) as u8;
                base[dst_idx + 3] = 255;
            }
        }
    }
}

/// Opaque black, in the overlay's BGRA order.
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// A colour in the overlay's BGRA order.
fn bgra(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
    [b, g, r, a]
}

/// The offsets that dilate the glyphs into a stroke of the given width.
fn stroke_offsets(width: i32) -> Vec<(i32, i32)> {
    let mut offsets = Vec::new();
    for dy in -width..=width {
        for dx in -width..=width {
            if dx == 0 && dy == 0 {
                continue;
            }
            offsets.push((dx, dy));
        }
    }
    offsets
}

/// A tinted copy of the glyphs, shifted by the given offset.
///
/// The copy is used for the drop shadow, the outline and the text stroke: the
/// glyphs are repeated behind the text in another colour.
fn offset_overlay(
    overlay: &[u8],
    width: u32,
    height: u32,
    dx: i32,
    dy: i32,
    strength: f32,
    tint: [u8; 4],
) -> Vec<u8> {
    let mut out = vec![0u8; overlay.len()];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let sx = x - dx;
            let sy = y - dy;
            if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                continue;
            }
            let src = ((sy as u32 * width + sx as u32) * 4) as usize;
            let dst = ((y as u32 * width + x as u32) * 4) as usize;
            out[dst] = tint[0];
            out[dst + 1] = tint[1];
            out[dst + 2] = tint[2];
            out[dst + 3] = (overlay[src + 3] as f32 * strength) as u8;
        }
    }
    out
}

/// A blurred halo of the glyphs, in the glow's colour.
///
/// The halo is built from the text's own region, so the blur stays cheap
/// even on a full-resolution output.
#[allow(clippy::too_many_arguments)]
fn glow_overlay(
    overlay: &[u8],
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    text_w: u32,
    text_h: u32,
    radius: i32,
    tint: [u8; 4],
) -> Vec<u8> {
    let mut out = vec![0u8; (width * height * 4) as usize];
    let radius = radius.max(1);

    // The region the halo can reach.
    let x0 = (left - radius).max(0) as u32;
    let y0 = (top - radius).max(0) as u32;
    let x1 = ((left + text_w as i32 + radius).min(width as i32 - 1)).max(0) as u32;
    let y1 = ((top + text_h as i32 + radius).min(height as i32 - 1)).max(0) as u32;
    if x1 <= x0 || y1 <= y0 {
        return out;
    }
    let region_w = (x1 - x0 + 1) as usize;
    let region_h = (y1 - y0 + 1) as usize;

    // The glyph coverage in that region.
    let mut alpha = vec![0u8; region_w * region_h];
    for y in 0..region_h {
        for x in 0..region_w {
            let idx = ((y0 as usize + y) * width as usize + (x0 as usize + x)) * 4 + 3;
            alpha[y * region_w + x] = overlay[idx];
        }
    }

    // A separable box blur over the coverage.
    let mut horizontal = vec![0u32; region_w * region_h];
    for y in 0..region_h {
        for x in 0..region_w {
            let mut sum = 0u32;
            let mut count = 0u32;
            for k in -radius..=radius {
                let sx = x as i32 + k;
                if sx < 0 || sx >= region_w as i32 {
                    continue;
                }
                sum += alpha[y * region_w + sx as usize] as u32;
                count += 1;
            }
            horizontal[y * region_w + x] = sum / count.max(1);
        }
    }
    let mut blurred = vec![0u32; region_w * region_h];
    for y in 0..region_h {
        for x in 0..region_w {
            let mut sum = 0u32;
            let mut count = 0u32;
            for k in -radius..=radius {
                let sy = y as i32 + k;
                if sy < 0 || sy >= region_h as i32 {
                    continue;
                }
                sum += horizontal[sy as usize * region_w + x];
                count += 1;
            }
            blurred[y * region_w + x] = sum / count.max(1);
        }
    }

    // The halo, in the glow's colour.
    for y in 0..region_h {
        for x in 0..region_w {
            let a = blurred[y * region_w + x];
            if a == 0 {
                continue;
            }
            let idx = ((y0 as usize + y) * width as usize + (x0 as usize + x)) * 4;
            out[idx] = tint[0];
            out[idx + 1] = tint[1];
            out[idx + 2] = tint[2];
            out[idx + 3] = a.min(255) as u8;
        }
    }
    out
}

/// Draw `src` over `dst`, keeping the alpha coverage of both.
fn merge_over(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.chunks_mut(4).zip(src.chunks(4)) {
        let sa = s[3] as f32 / 255.0;
        if sa <= 0.0 {
            continue;
        }
        let da = d[3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            continue;
        }
        for c in 0..3 {
            d[c] = ((s[c] as f32 * sa + d[c] as f32 * da * (1.0 - sa)) / out_a) as u8;
        }
        d[3] = (out_a * 255.0).round() as u8;
    }
}

fn measure_buffer(buf: &Buffer) -> (u32, u32, f32) {
    let mut max_w: f32 = 0.0;
    let mut total_h: f32 = 0.0;
    let mut baseline: f32 = 0.0;
    for run in buf.layout_runs() {
        if run.line_w > max_w {
            max_w = run.line_w;
        }
        total_h += run.line_height;
        if run.line_y > baseline {
            baseline = run.line_y;
        }
    }
    (max_w.ceil() as u32, total_h.ceil() as u32, baseline)
}

/// The pieces of a text that are drawn in small capitals.
///
/// The first letter of the text keeps its full size and the rest of the
/// letters are drawn as small capitals: the text reads as small caps rather
/// than as a smaller shout.
fn small_caps_segments<'a>(segments: &[(&'a str, Attrs<'a>)]) -> Vec<(&'a str, Attrs<'a>)> {
    let mut out: Vec<(&'a str, Attrs<'a>)> = Vec::new();
    let mut seen_letter = false;
    for (text, attrs) in segments {
        if text.is_empty() {
            continue;
        }
        // The head is everything up to and including the first letter: it is
        // drawn at the text's own size.
        let head_end = if seen_letter {
            0
        } else {
            match text.char_indices().find(|(_, ch)| !ch.is_whitespace()) {
                Some((index, ch)) => {
                    seen_letter = true;
                    index + ch.len_utf8()
                }
                None => 0,
            }
        };
        if head_end > 0 {
            out.push((&text[..head_end], attrs.clone()));
        }
        if head_end < text.len() {
            let mut small = attrs.clone();
            let metrics = attrs
                .metrics_opt
                .map(Metrics::from)
                .unwrap_or(Metrics::new(16.0, 19.0));
            small = small.metrics(Metrics::new(
                metrics.font_size * SMALL_CAPS_SCALE,
                metrics.line_height * SMALL_CAPS_SCALE,
            ));
            out.push((&text[head_end..], small));
        }
    }
    out
}

/// The size a small capital is drawn at, as a fraction of the text's size.
const SMALL_CAPS_SCALE: f32 = 0.78;

/// Draw a horizontal line into a rendered text region.
///
/// The underline sits just below the baseline and the strikethrough about
/// half a letter above it, the way a word processor draws them.
#[allow(clippy::too_many_arguments)]
fn draw_text_rule(
    pixels: &mut [u8],
    region_w: u32,
    region_h: u32,
    x0: i32,
    x1: i32,
    y: i32,
    thickness: i32,
    color: (u8, u8, u8, u8),
) {
    if thickness <= 0 || x1 <= x0 {
        return;
    }
    let half = thickness / 2;
    for row in (y - half)..=(y - half + thickness.max(1) - 1) {
        if row < 0 || row >= region_h as i32 {
            continue;
        }
        for col in x0.max(0)..x1.min(region_w as i32) {
            let index = ((row as u32 * region_w + col as u32) * 4) as usize;
            if index + 3 >= pixels.len() {
                continue;
            }
            pixels[index] = color.0;
            pixels[index + 1] = color.1;
            pixels[index + 2] = color.2;
            pixels[index + 3] = color.3;
        }
    }
}

pub fn create_gpu_text_renderer() -> Result<GpuTextRenderer> {
    pollster::block_on(GpuTextRenderer::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_up_works() {
        assert_eq!(align_up(0, 256), 0);
        assert_eq!(align_up(1, 256), 256);
        assert_eq!(align_up(256, 256), 256);
        assert_eq!(align_up(257, 256), 512);
    }

    /// An overlay with one opaque pixel at the given position.
    fn overlay_with_pixel(width: u32, height: u32, x: u32, y: u32) -> Vec<u8> {
        let mut out = vec![0u8; (width * height * 4) as usize];
        let idx = ((y * width + x) * 4) as usize;
        out[idx] = 10;
        out[idx + 1] = 20;
        out[idx + 2] = 30;
        out[idx + 3] = 255;
        out
    }

    /// The text is drawn into a texture of its own size, so the region has
    /// to hold the glyphs and the effects that reach around them — and the
    /// text has to land in the same place in the frame as it always did.
    #[test]
    fn the_text_region_holds_the_glyphs_and_their_effects() {
        let width = 1920u32;
        let height = 1080u32;

        // A line of text in the middle of the frame.
        let (left, top, region_w, region_h) = text_region(760, 500, 400, 60, 4, width, height);
        assert_eq!((left, top), (756, 496));
        assert_eq!(region_w, 408, "the region holds the glyphs and a margin");
        assert_eq!(region_h, 68);

        // The text's place in the frame is the region's origin plus its
        // place in the region: the text has not moved.
        assert_eq!(left as i32 + (760 - 756), 760);
        assert_eq!(top as i32 + (500 - 496), 500);
    }

    /// A text at the edge of the frame is not drawn outside it.
    #[test]
    fn the_text_region_stays_inside_the_frame() {
        let (left, top, region_w, region_h) = text_region(0, 0, 100, 40, 8, 1920, 1080);
        assert_eq!((left, top), (0, 0));
        assert_eq!(region_w, 108);
        assert_eq!(region_h, 48);

        // A text that hangs over the edge is clipped to the frame: the
        // region runs from 1892 to the frame's edge at 1920.
        let (_, _, region_w, region_h) = text_region(1900, 1060, 100, 40, 8, 1920, 1080);
        assert_eq!(region_w, 28, "the region is clipped to the frame");
        assert_eq!(region_h, 28);
    }

    #[test]
    fn the_shadow_is_the_glyphs_shifted_and_turned_black() {
        let width = 4u32;
        let overlay = overlay_with_pixel(width, width, 1, 1);
        let shadow = offset_overlay(&overlay, width, width, 1, 1, 0.7, BLACK);

        // The glyph moved down and right by the offset.
        let moved = ((2 * width + 2) * 4) as usize;
        assert_eq!(shadow[moved + 3], 178, "the shadow is softer than the text");
        assert_eq!(shadow[moved], 0);
        assert_eq!(shadow[moved + 1], 0);
        assert_eq!(shadow[moved + 2], 0);

        // The original position is empty.
        let (glyph_x, glyph_y) = (1u32, 1u32);
        let original = ((glyph_y * width + glyph_x) * 4) as usize;
        assert_eq!(shadow[original + 3], 0);
    }

    #[test]
    fn a_stroke_dilates_the_glyphs() {
        assert!(stroke_offsets(0).is_empty());

        let offsets = stroke_offsets(1);
        assert_eq!(offsets.len(), 8, "the eight neighbours of the glyph");
        assert!(
            !offsets.contains(&(0, 0)),
            "the glyph covers its own centre"
        );
    }

    #[test]
    fn the_glow_spreads_the_glyphs_into_a_halo() {
        let width = 8u32;
        let height = 8u32;
        let overlay = overlay_with_pixel(width, height, 4, 4);
        let glow = glow_overlay(&overlay, width, height, 4, 4, 1, 1, 2, bgra(0, 255, 0, 255));

        // The halo reaches further than the glyph itself, and it is green.
        let halo_pixel = ((4 * width + 5) * 4) as usize;
        assert!(
            glow[halo_pixel + 3] > 0,
            "the halo spreads next to the glyph"
        );
        assert_eq!(glow[halo_pixel + 1], 255, "the halo is green");
        assert_eq!(glow[halo_pixel + 2], 0);
    }

    #[test]
    fn merging_puts_the_text_over_the_shadow() {
        let mut shadow = offset_overlay(&overlay_with_pixel(2, 2, 0, 0), 2, 2, 0, 0, 0.7, BLACK);
        assert_eq!(shadow[3], 178);

        // The text is opaque and covers the shadow where they meet.
        let text = overlay_with_pixel(2, 2, 0, 0);
        merge_over(&mut shadow, &text);
        assert_eq!(shadow[3], 255, "the glyph is fully covered");
        assert_eq!(shadow[2], 30, "the text's colour wins");
    }
}
