//! Overlay content composited on top of a slide.
//!
//! A "look" decides what sits on top of the slide: the props (lower thirds,
//! logos), the mask that frames the output, and any live alert message.
//! The same overlay is drawn by every output surface — the audience window,
//! the dock preview, the stage, the NDI sender and the recorder — so the
//! operator always sees exactly what the audience sees.

use crate::domain::{LiveAlertMessage, Mask, Prop};

/// The overlay content for one output.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlay {
    /// The props to draw, in order: later props sit on top.
    pub props: Vec<Prop>,
    /// The mask that frames the output.
    pub mask: Mask,
    /// A live alert banner.
    pub live_message: Option<LiveAlertMessage>,
    /// The announcement the app is showing.
    ///
    /// An announcement is what the app says to the room on its own — the
    /// service starts in five minutes, the room is being recorded. It sits
    /// at the top of the output, above the alert, so the two can be on the
    /// screen at once.
    pub announcement: Option<LiveAlertMessage>,
    /// Whether the slide itself is drawn. A screen that routes only media or
    /// only props switches the slide layer off.
    pub show_slide: bool,
    /// Whether the slide's video is drawn.
    pub show_media: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            props: Vec::new(),
            mask: Mask::None,
            live_message: None,
            announcement: None,
            show_slide: true,
            show_media: true,
        }
    }
}

impl Overlay {
    /// Whether the overlay would draw anything at all.
    pub fn is_empty(&self) -> bool {
        self.props.is_empty()
            && matches!(self.mask, Mask::None)
            && self.live_message.is_none()
            && self.announcement.is_none()
    }
}
