use super::FrameRate;
use super::sys::*;
use crate::Result;
use crate::render::Frame;
use std::ffi::CString;

pub struct NdiSender {
    instance: *mut NDIlib_send_instance_t,
    source_name: String,
    width: u32,
    height: u32,
    frame_rate: FrameRate,
}

unsafe impl Send for NdiSender {}
unsafe impl Sync for NdiSender {}

impl NdiSender {
    pub fn new(
        source_name: String,
        width: u32,
        height: u32,
        frame_rate: FrameRate,
    ) -> Result<Self> {
        let name_cstr = CString::new(source_name.clone())?;

        let create_settings = NDIlib_send_create_t {
            p_ndi_name: name_cstr.as_ptr(),
            p_groups: std::ptr::null(),
            clock_video: true,
            clock_audio: true,
        };

        let instance = unsafe { NDIlib_send_create(&create_settings) };

        if instance.is_null() {
            anyhow::bail!("Failed to create NDI sender");
        }

        Ok(Self {
            instance,
            source_name,
            width,
            height,
            frame_rate,
        })
    }

    pub fn send_frame(&mut self, frame: &Frame) -> Result<()> {
        if frame.width != self.width || frame.height != self.height {
            anyhow::bail!(
                "Frame size mismatch: expected {}x{}, got {}x{}",
                self.width,
                self.height,
                frame.width,
                frame.height
            );
        }

        let ndi_frame = NDIlib_video_frame_v2_t {
            xres: self.width as i32,
            yres: self.height as i32,
            fourcc: FOURCC_BGRA,
            frame_rate_n: self.frame_rate.numerator,
            frame_rate_d: self.frame_rate.denominator,
            picture_aspect_ratio: self.width as f32 / self.height as f32,
            frame_format_type: 0,
            timecode: 0,
            p_data: frame.data.as_ptr(),
            line_stride_in_bytes: (self.width * 4) as i32,
            p_metadata: std::ptr::null(),
            timestamp: 0,
        };

        unsafe {
            NDIlib_send_send_video_v2(self.instance, &ndi_frame);
        }

        Ok(())
    }

    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

impl Drop for NdiSender {
    fn drop(&mut self) {
        unsafe {
            NDIlib_send_destroy(self.instance);
        }
    }
}
