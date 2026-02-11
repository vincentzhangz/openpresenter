use crate::Result;
use crate::slides::Slide;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for pixel in self.data.chunks_exact_mut(4) {
            pixel[0] = b; // B
            pixel[1] = g; // G
            pixel[2] = r; // R
            pixel[3] = a; // A
        }
    }
}

pub struct RenderPipeline {
    width: u32,
    height: u32,
}

impl RenderPipeline {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn render_slide(&self, _slide: &Slide) -> Result<Frame> {
        let mut frame = Frame::new(self.width, self.height);

        // TODO: Implement actual slide rendering
        frame.clear(0, 0, 0, 255);

        Ok(frame)
    }
}
