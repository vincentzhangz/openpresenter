//! The test patterns.
//!
//! A test pattern is what a screen shows when there is nothing to show: it
//! tells the operator that the projector is alive, that the colours are the
//! colours they should be, and where the edges of the picture are.
//!
//! The patterns are drawn over the output, not into a slide: a pattern is
//! a property of the screen, not of the presentation.

use std::sync::Arc;

use crate::render::Frame;

/// The pattern a screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pattern {
    /// Nothing: the screen shows the output as it is.
    #[default]
    Clear,
    /// The colour bars.
    ///
    /// The seven bars of the SMPTE pattern, from left to right: white,
    /// yellow, cyan, green, magenta, red and blue.
    ColorBars,
    /// A ramp of grey, from black to white.
    GrayScale,
    /// A grid of lines over black.
    Grid,
    /// A circle in the middle of the screen.
    CenterCircle,
    /// The centre lines: one across and one down.
    CenterLines,
    /// A small cross in the middle, with the corners marked.
    Focus,
    /// A grid that fades towards the middle.
    BlendGrid,
    /// The colours turned over.
    InvertColors,
    /// A ramp from black to the chosen grey.
    BlackLevel(u8),
    /// A ramp from the chosen grey to white.
    WhiteLevel(u8),
    /// The screen filled with the chosen black.
    BlackColor,
    /// The screen filled with the chosen white.
    WhiteColor,
    /// The screen filled with the chosen colour.
    CustomColor(u8, u8, u8),
}

impl Pattern {
    /// The name the pattern is shown under.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::ColorBars => "Color Bars",
            Self::GrayScale => "Gray Scale",
            Self::Grid => "Grid",
            Self::CenterCircle => "Center Circle",
            Self::CenterLines => "Center Lines",
            Self::Focus => "Focus",
            Self::BlendGrid => "Blend Grid",
            Self::InvertColors => "Invert Colors",
            Self::BlackLevel(_) => "Black Level",
            Self::WhiteLevel(_) => "White Level",
            Self::BlackColor => "Black Color",
            Self::WhiteColor => "White Color",
            Self::CustomColor(..) => "Custom Color",
        }
    }

    /// Every pattern the operator can pick.
    pub const ALL: [Pattern; 13] = [
        Pattern::Clear,
        Pattern::ColorBars,
        Pattern::GrayScale,
        Pattern::Grid,
        Pattern::CenterCircle,
        Pattern::CenterLines,
        Pattern::Focus,
        Pattern::BlendGrid,
        Pattern::InvertColors,
        Pattern::BlackLevel(0),
        Pattern::WhiteLevel(255),
        Pattern::BlackColor,
        Pattern::WhiteColor,
    ];

    /// Whether the pattern is a ramp whose level the operator sets.
    pub fn has_level(&self) -> bool {
        matches!(self, Self::BlackLevel(_) | Self::WhiteLevel(_))
    }
}

/// The colours of the colour bars, from left to right.
const COLOR_BARS: [(u8, u8, u8); 7] = [
    (192, 192, 192),
    (192, 192, 0),
    (0, 192, 192),
    (0, 192, 0),
    (192, 0, 192),
    (192, 0, 0),
    (0, 0, 192),
];

/// Draw the pattern over a frame.
///
/// The pattern covers the whole picture: a test pattern that leaves the
/// slide showing round its edges is not a test pattern, it is a border.
pub fn draw(pattern: Pattern, frame: &mut Frame) {
    match pattern {
        // A clear pattern leaves the picture alone.
        Pattern::Clear => {}
        Pattern::ColorBars => color_bars(frame),
        Pattern::GrayScale => gray_scale(frame),
        Pattern::Grid => grid(frame),
        Pattern::CenterCircle => center_circle(frame),
        Pattern::CenterLines => center_lines(frame),
        Pattern::Focus => focus(frame),
        Pattern::BlendGrid => blend_grid(frame),
        Pattern::InvertColors => invert_colors(frame),
        Pattern::BlackLevel(level) => level_ramp(frame, level, true),
        Pattern::WhiteLevel(level) => level_ramp(frame, level, false),
        Pattern::BlackColor => fill(frame, (0, 0, 0)),
        Pattern::WhiteColor => fill(frame, (255, 255, 255)),
        Pattern::CustomColor(r, g, b) => fill(frame, (r, g, b)),
    }
}

/// Fill the whole frame with one colour.
fn fill(frame: &mut Frame, colour: (u8, u8, u8)) {
    for pixel in Arc::make_mut(&mut frame.data).as_chunks_mut::<4>().0 {
        *pixel = [colour.2, colour.1, colour.0, 255];
    }
}

/// The seven colour bars.
fn color_bars(frame: &mut Frame) {
    let bar_width = frame.width as f32 / COLOR_BARS.len() as f32;
    let data = Arc::make_mut(&mut frame.data);
    for (index, pixel) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32) % frame.width;
        let bar = ((x as f32 / bar_width) as usize).min(COLOR_BARS.len() - 1);
        let (r, g, b) = COLOR_BARS[bar];
        *pixel = [b, g, r, 255];
    }
}

/// A ramp of grey, from black on the left to white on the right.
fn gray_scale(frame: &mut Frame) {
    let data = Arc::make_mut(&mut frame.data);
    for (index, pixel) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32) % frame.width;
        let level = if frame.width <= 1 {
            0
        } else {
            (x * 255 / (frame.width - 1)) as u8
        };
        *pixel = [level, level, level, 255];
    }
}

/// A ramp that runs from black to a level, or from a level to white.
///
/// The level is the grey the ramp ends at: the operator sets it to see how
/// dark the picture really is.
fn level_ramp(frame: &mut Frame, level: u8, from_black: bool) {
    let data = Arc::make_mut(&mut frame.data);
    for (index, pixel) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32) % frame.width;
        let step = if frame.width <= 1 {
            0
        } else {
            (x * 255 / (frame.width - 1)) as u8
        };
        let value = if from_black { step } else { level };
        *pixel = [value, value, value, 255];
    }
}

/// A grid of lines over black.
///
/// The grid is a tenth of the screen apart, so an operator can see the
/// picture's shape without counting.
fn grid(frame: &mut Frame) {
    fill(frame, (0, 0, 0));
    let step_x = (frame.width / 10).max(1);
    let step_y = (frame.height / 10).max(1);
    for y in 0..frame.height {
        for x in 0..frame.width {
            if x % step_x == 0 || y % step_y == 0 {
                set(frame, x, y, (255, 255, 255));
            }
        }
    }
}

/// A circle in the middle of the screen.
fn center_circle(frame: &mut Frame) {
    fill(frame, (0, 0, 0));
    let centre_x = frame.width as f32 / 2.0;
    let centre_y = frame.height as f32 / 2.0;
    let radius = (frame.width.min(frame.height) as f32 * 0.4).max(1.0);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let dx = x as f32 - centre_x;
            let dy = y as f32 - centre_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if (distance - radius).abs() < 2.0 {
                set(frame, x, y, (255, 255, 255));
            }
        }
    }
}

/// The centre lines: one across the screen and one down it.
fn center_lines(frame: &mut Frame) {
    fill(frame, (0, 0, 0));
    let centre_x = frame.width / 2;
    let centre_y = frame.height / 2;
    for y in 0..frame.height {
        set(frame, centre_x, y, (255, 255, 255));
    }
    for x in 0..frame.width {
        set(frame, x, centre_y, (255, 255, 255));
    }
}

/// A small cross in the middle, with the corners marked.
fn focus(frame: &mut Frame) {
    fill(frame, (0, 0, 0));
    let centre_x = frame.width / 2;
    let centre_y = frame.height / 2;
    let arm = (frame.width.min(frame.height) / 10).max(2);
    for offset in 0..arm {
        if centre_x >= offset {
            set(frame, centre_x - offset, centre_y, (255, 255, 255));
        }
        if centre_x + offset < frame.width {
            set(frame, centre_x + offset, centre_y, (255, 255, 255));
        }
        if centre_y >= offset {
            set(frame, centre_x, centre_y - offset, (255, 255, 255));
        }
        if centre_y + offset < frame.height {
            set(frame, centre_x, centre_y + offset, (255, 255, 255));
        }
    }
}

/// A grid that fades towards the middle.
///
/// The pattern shows the brightness of the screen's edges against its
/// middle, which is how a projector is set up for a room with a bright
/// window in it.
fn blend_grid(frame: &mut Frame) {
    let centre_x = frame.width as f32 / 2.0;
    let centre_y = frame.height as f32 / 2.0;
    let far = (centre_x * centre_x + centre_y * centre_y).sqrt().max(1.0);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let dx = x as f32 - centre_x;
            let dy = y as f32 - centre_y;
            let distance = (dx * dx + dy * dy).sqrt();
            let level = (distance / far * 255.0) as u8;
            set(frame, x, y, (level, level, level));
        }
    }
}

/// The colours of the picture turned over.
fn invert_colors(frame: &mut Frame) {
    for pixel in Arc::make_mut(&mut frame.data).as_chunks_mut::<4>().0 {
        pixel[0] = 255 - pixel[0];
        pixel[1] = 255 - pixel[1];
        pixel[2] = 255 - pixel[2];
        pixel[3] = 255;
    }
}

/// Set one pixel of the frame.
fn set(frame: &mut Frame, x: u32, y: u32, colour: (u8, u8, u8)) {
    if x >= frame.width || y >= frame.height {
        return;
    }
    let index = ((y * frame.width + x) * 4) as usize;
    let data = Arc::make_mut(&mut frame.data);
    if index + 3 >= data.len() {
        return;
    }
    // The frame is BGRA and the colours are written as RGB.
    data[index] = colour.2;
    data[index + 1] = colour.1;
    data[index + 2] = colour.0;
    data[index + 3] = 255;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pattern covers the whole picture.
    #[test]
    fn the_pattern_covers_the_whole_frame() {
        let mut frame = Frame::new(64, 32);
        draw(Pattern::ColorBars, &mut frame);
        // Every pixel of the first bar is the first colour.
        let first = frame.pixel(1, 1).expect("the pixel is there");
        assert_eq!(first, (192, 192, 192, 255));
        // The last bar is the last colour.
        let last = frame.pixel(62, 30).expect("the pixel is there");
        assert_eq!(last, (0, 0, 192, 255));
    }

    /// A clear pattern leaves the picture alone.
    #[test]
    fn a_clear_pattern_leaves_the_picture_alone() {
        let mut frame = Frame::new(8, 8);
        frame.set_pixel(0, 0, 10, 20, 30, 255);
        draw(Pattern::Clear, &mut frame);
        assert_eq!(frame.pixel(0, 0), Some((10, 20, 30, 255)));
    }

    /// The colours are turned over, not the picture's shape.
    #[test]
    fn the_inverted_pattern_turns_the_colours_over() {
        let mut frame = Frame::new(4, 4);
        frame.set_pixel(1, 1, 0, 0, 0, 255);
        draw(Pattern::InvertColors, &mut frame);
        assert_eq!(frame.pixel(1, 1), Some((255, 255, 255, 255)));
    }
}
