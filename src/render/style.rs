//! A render-time style override for one screen.
//!
//! A screen can show a slide in a different theme without the slide itself
//! changing: the theme's background, text style and transition replace the
//! slide's own for that screen only. That is what lets the broadcast carry
//! a different look from the projector while both show the same slide.

use crate::domain::{Background, TextStyle, Transition};

/// The style a screen draws the slide with.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleOverride {
    /// The background to draw instead of the slide's own.
    pub background: Option<Background>,
    /// The text style to use instead of each text object's own.
    pub text_style: Option<TextStyle>,
    /// The transition to use instead of the slide's own.
    pub transition: Option<Transition>,
}

impl StyleOverride {
    /// Whether the override changes anything.
    pub fn is_empty(&self) -> bool {
        self.background.is_none() && self.text_style.is_none() && self.transition.is_none()
    }
}
