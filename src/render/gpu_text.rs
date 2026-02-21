use crate::Result;
use crate::slides::{TextAlignment, TextRun, TextStyle};
use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
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
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
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
    ) -> Result<Vec<u8>> {
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

        let mut viewport = Viewport::new(&self.device, &self.cache);
        viewport.update(&self.queue, Resolution { width, height });

        let display_text: String = style.text_transform.apply(text);

        let line_height = style.font_size * style.line_height_multiplier;
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(style.font_size, line_height),
        );
        buffer.set_size(
            &mut self.font_system,
            Some(width as f32),
            Some(height as f32),
        );

        let base_family_name = style.font_family.as_str();
        let base_family = if base_family_name.is_empty() || base_family_name == "Arial" {
            Family::SansSerif
        } else {
            Family::Name(base_family_name)
        };
        let base_attrs = Attrs::new().family(base_family);

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
                    (txt, Attrs::new().family(run_family))
                })
                .collect();
            buffer.set_rich_text(
                &mut self.font_system,
                transformed_runs
                    .iter()
                    .map(|(s, a)| (s.as_str(), a.clone())),
                &base_attrs,
                Shaping::Advanced,
                None,
            );
        } else {
            buffer.set_text(
                &mut self.font_system,
                &display_text,
                &base_attrs,
                Shaping::Advanced,
                None,
            );
        }
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (text_w, text_h) = measure_buffer(&buffer);

        let center_x = (style.position_x * width as f32) as i32;
        let center_y = (style.position_y * height as f32) as i32;

        let left = match style.alignment {
            TextAlignment::Left => center_x,
            TextAlignment::Center => center_x - (text_w / 2) as i32,
            TextAlignment::Right => center_x - text_w as i32,
        };
        let top = center_y - (text_h / 2) as i32;

        let gc = glyphon::Color::rgba(style.color.r, style.color.g, style.color.b, style.color.a);
        text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                &viewport,
                [TextArea {
                    buffer: &buffer,
                    left: left as f32,
                    top: top as f32,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
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
                width,
                height,
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
                .render(&self.atlas, &viewport, &mut pass)
                .map_err(|e| anyhow::anyhow!("glyphon render error: {:?}", e))?;
        }

        let bytes_per_pixel = 4u32;
        let unpadded_row = width * bytes_per_pixel;
        let padded_row = align_up(unpadded_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

        let readback_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_readback"),
            size: (padded_row * height) as u64,
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
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let buf_slice = readback_buf.slice(..);
        buf_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| anyhow::anyhow!("wgpu poll error: {:?}", e))?;

        let mapped = buf_slice.get_mapped_range();

        let mut out = vec![0u8; (width * height * 4) as usize];
        for row in 0..height {
            let src_start = (row * padded_row) as usize;
            let src_row = &mapped[src_start..src_start + unpadded_row as usize];
            let dst_start = (row * unpadded_row) as usize;
            let dst_row = &mut out[dst_start..dst_start + unpadded_row as usize];
            for px in 0..width as usize {
                dst_row[px * 4] = src_row[px * 4 + 2];
                dst_row[px * 4 + 1] = src_row[px * 4 + 1];
                dst_row[px * 4 + 2] = src_row[px * 4];
                dst_row[px * 4 + 3] = src_row[px * 4 + 3];
            }
        }
        drop(mapped);
        readback_buf.unmap();

        Ok(out)
    }

    pub fn composite_bgra(base: &mut [u8], overlay: &[u8]) {
        debug_assert_eq!(base.len(), overlay.len());
        for (b_px, o_px) in base.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
            let a = o_px[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let ia = 1.0 - a;
            b_px[0] = (b_px[0] as f32 * ia + o_px[0] as f32 * a) as u8;
            b_px[1] = (b_px[1] as f32 * ia + o_px[1] as f32 * a) as u8;
            b_px[2] = (b_px[2] as f32 * ia + o_px[2] as f32 * a) as u8;
            b_px[3] = 255;
        }
    }
}

fn measure_buffer(buf: &Buffer) -> (u32, u32) {
    let mut max_w: f32 = 0.0;
    let mut total_h: f32 = 0.0;
    for run in buf.layout_runs() {
        if run.line_w > max_w {
            max_w = run.line_w;
        }
        total_h += run.line_height;
    }
    (max_w.ceil() as u32, total_h.ceil() as u32)
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
}
