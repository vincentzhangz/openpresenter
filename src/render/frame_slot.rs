//! The shared frame slot.
//!
//! Every output surface — the audience window, the dock preview, the stage
//! monitor, the NDI sender — draws its own composition, and the surfaces
//! live in separate windows. The slot is how a finished frame reaches them:
//! the owner of a composition publishes it here, and the surfaces read the
//! frame of the screen they belong to.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui_kit::RenderImage;

use crate::render::Frame;

/// The frames of every screen, keyed by screen id.
///
/// The generation is the composition's counter, so a surface can tell a new
/// frame from the one it already painted.
#[derive(Default)]
pub struct FrameSlot {
    frames: Mutex<HashMap<String, (u64, Arc<RenderImage>)>>,
}

impl FrameSlot {
    /// Publish a new frame for a screen.
    pub fn publish(&self, screen_id: &str, generation: u64, image: Arc<RenderImage>) {
        if let Ok(mut guard) = self.frames.lock() {
            guard.insert(screen_id.to_string(), (generation, image));
        }
    }

    /// The latest frame of a screen and the generation it was composed for.
    pub fn latest(&self, screen_id: &str) -> Option<(u64, Arc<RenderImage>)> {
        self.frames
            .lock()
            .ok()
            .and_then(|guard| guard.get(screen_id).cloned())
    }

    /// Drop the current frame of a screen.
    pub fn clear(&self, screen_id: &str) {
        if let Ok(mut guard) = self.frames.lock() {
            guard.remove(screen_id);
        }
    }

    /// Drop every frame.
    pub fn clear_all(&self) {
        if let Ok(mut guard) = self.frames.lock() {
            guard.clear();
        }
    }

    /// Whether any screen has a frame.
    pub fn is_empty(&self) -> bool {
        self.frames
            .lock()
            .map(|guard| guard.is_empty())
            .unwrap_or(true)
    }
}

/// Convert a composited frame into a GPUI image.
///
/// GPUI's image pipeline stores pixels in **BGRA** order (its own loaders swap
/// RGBA sources before building a [`RenderImage`]), and [`Frame`] already
/// writes `[b, g, r, a]`, so the bytes are handed over unchanged.
pub fn frame_to_image(frame: &Frame) -> Option<Arc<RenderImage>> {
    let buffer =
        image::RgbaImage::from_raw(frame.width, frame.height, frame.data.as_ref().clone())?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}
