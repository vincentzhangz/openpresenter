mod output;
mod sender;
mod sys;

pub use output::{NdiOutputLoop, OutputCommand};
pub use sender::NdiSender;

use crate::Result;

pub fn initialize() -> Result<()> {
    unsafe {
        if !sys::NDIlib_initialize() {
            anyhow::bail!("Failed to initialize NDI library");
        }
    }
    Ok(())
}

pub fn destroy() {
    unsafe {
        sys::NDIlib_destroy();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VideoFormat {
    BGRA,
    UYVY,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameRate {
    pub numerator: i32,
    pub denominator: i32,
}

impl FrameRate {
    pub const NTSC_30: Self = Self {
        numerator: 30000,
        denominator: 1001,
    };
    pub const PAL_25: Self = Self {
        numerator: 25,
        denominator: 1,
    };
    pub const FPS_60: Self = Self {
        numerator: 60,
        denominator: 1,
    };
    pub const FPS_30: Self = Self {
        numerator: 30,
        denominator: 1,
    };
}
