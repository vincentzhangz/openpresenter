mod output;
mod sender;
mod sys;

pub use output::{NdiOutputLoop, OutputCommand};
pub use sender::NdiSender;

use crate::Result;

/// Check whether the native NDI SDK runtime library is installed on the host system.
pub fn is_available() -> bool {
    #[cfg(feature = "ndi-sdk")]
    {
        #[cfg(target_os = "macos")]
        {
            std::path::Path::new("/Library/NDI SDK for Apple/lib/macOS/libndi.dylib").exists()
                || std::path::Path::new("/usr/local/lib/libndi.dylib").exists()
        }
        #[cfg(target_os = "windows")]
        {
            std::path::Path::new(
                "C:/Program Files/NDI/NDI 5 SDK/Lib/x64/Processing.NDI.Lib.x64.dll",
            )
            .exists()
        }
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/usr/local/lib/libndi.so").exists()
                || std::path::Path::new("/usr/lib/libndi.so").exists()
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            false
        }
    }
    #[cfg(not(feature = "ndi-sdk"))]
    {
        false
    }
}

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
