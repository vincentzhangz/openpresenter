use std::ffi::c_char;

#[repr(C)]
pub struct NDIlib_send_instance_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct NDIlib_video_frame_v2_t {
    pub xres: i32,
    pub yres: i32,
    pub fourcc: u32,
    pub frame_rate_n: i32,
    pub frame_rate_d: i32,
    pub picture_aspect_ratio: f32,
    pub frame_format_type: i32,
    pub timecode: i64,
    pub p_data: *const u8,
    pub line_stride_in_bytes: i32,
    pub p_metadata: *const c_char,
    pub timestamp: i64,
}

impl Default for NDIlib_video_frame_v2_t {
    fn default() -> Self {
        Self {
            xres: 0,
            yres: 0,
            fourcc: FOURCC_BGRA,
            frame_rate_n: 60,
            frame_rate_d: 1,
            picture_aspect_ratio: 16.0 / 9.0,
            frame_format_type: 0, // Progressive
            timecode: 0,
            p_data: std::ptr::null(),
            line_stride_in_bytes: 0,
            p_metadata: std::ptr::null(),
            timestamp: 0,
        }
    }
}

#[repr(C)]
pub struct NDIlib_send_create_t {
    pub p_ndi_name: *const c_char,
    pub p_groups: *const c_char,
    pub clock_video: bool,
    pub clock_audio: bool,
}

pub const FOURCC_BGRA: u32 = 0x41524742; // 'BGRA'
#[allow(dead_code)]
pub const FOURCC_UYVY: u32 = 0x59565955; // 'UYVY' (reserved for future use)

#[cfg(not(feature = "ndi-sdk"))]
#[allow(non_snake_case)]
pub unsafe fn NDIlib_initialize() -> bool {
    eprintln!("NDI stub: initialize called (no-op)");
    true
}

#[cfg(not(feature = "ndi-sdk"))]
#[allow(non_snake_case)]
pub unsafe fn NDIlib_destroy() {
    eprintln!("NDI stub: destroy called (no-op)");
}

#[cfg(not(feature = "ndi-sdk"))]
#[allow(non_snake_case)]
pub unsafe fn NDIlib_send_create(
    _p_create_settings: *const NDIlib_send_create_t,
) -> *mut NDIlib_send_instance_t {
    eprintln!("NDI stub: send_create called (returning dummy pointer)");
    0x1 as *mut NDIlib_send_instance_t
}

#[cfg(not(feature = "ndi-sdk"))]
#[allow(non_snake_case)]
pub unsafe fn NDIlib_send_destroy(_p_instance: *mut NDIlib_send_instance_t) {
    eprintln!("NDI stub: send_destroy called (no-op)");
}

#[cfg(not(feature = "ndi-sdk"))]
#[allow(non_snake_case)]
pub unsafe fn NDIlib_send_send_video_v2(
    _p_instance: *mut NDIlib_send_instance_t,
    p_video_data: *const NDIlib_video_frame_v2_t,
) {
    unsafe {
        let frame = &*p_video_data;
        eprintln!(
            "NDI stub: send_video_v2 called ({}x{}, no-op)",
            frame.xres, frame.yres
        );
    }
}

#[cfg(feature = "ndi-sdk")]
unsafe extern "C" {
    pub fn NDIlib_initialize() -> bool;
    pub fn NDIlib_destroy();
    pub fn NDIlib_send_create(
        p_create_settings: *const NDIlib_send_create_t,
    ) -> *mut NDIlib_send_instance_t;
    pub fn NDIlib_send_destroy(p_instance: *mut NDIlib_send_instance_t);
    pub fn NDIlib_send_send_video_v2(
        p_instance: *mut NDIlib_send_instance_t,
        p_video_data: *const NDIlib_video_frame_v2_t,
    );
}
