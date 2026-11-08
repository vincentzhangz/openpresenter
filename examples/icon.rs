//! Draw the app's icon.
//!
//! The icon is drawn rather than shipped as a picture, so it can be changed
//! with the app's colours and it never goes out of step with them.
//!
//! ```text
//! cargo run --example icon
//! ```

use image::{Rgb, RgbImage};

/// The size the icon is drawn at.
///
/// The icon is drawn big and scaled down: an icon that is drawn at its
/// final size has no room for the detail that survives the scaling.
const SIZE: u32 = 1024;

/// The app's background, the darkest of the interface.
const BACKGROUND: (u8, u8, u8) = (0x18, 0x18, 0x18);
/// The app's accent.
const ACCENT: (u8, u8, u8) = (0xF0, 0x37, 0x08);
/// The colour of the screen inside the icon.
const SCREEN: (u8, u8, u8) = (0xE2, 0xE2, 0xE2);

fn main() {
    let mut image = RgbImage::new(SIZE, SIZE);

    // The rounded square the icon sits on.
    let radius = SIZE as f32 * 0.2237;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
        if inside_rounded_square(cx, cy, SIZE as f32, radius) {
            *pixel = Rgb(BACKGROUND.into());
        }
    }

    // The screen: the slide the app shows on the projector.
    let screen = Rect {
        x: SIZE as f32 * 0.19,
        y: SIZE as f32 * 0.235,
        w: SIZE as f32 * 0.62,
        h: SIZE as f32 * 0.40,
    };
    fill_rect(&mut image, screen, SCREEN);

    // The accent bar across the top of the screen: the slide's own header,
    // and the app's accent.
    fill_rect(
        &mut image,
        Rect {
            x: screen.x,
            y: screen.y,
            w: screen.w,
            h: screen.h * 0.16,
        },
        ACCENT,
    );

    // The two lines of text on the slide.
    fill_rect(
        &mut image,
        Rect {
            x: screen.x + screen.w * 0.12,
            y: screen.y + screen.h * 0.42,
            w: screen.w * 0.56,
            h: screen.h * 0.10,
        },
        BACKGROUND,
    );
    fill_rect(
        &mut image,
        Rect {
            x: screen.x + screen.w * 0.12,
            y: screen.y + screen.h * 0.64,
            w: screen.w * 0.34,
            h: screen.h * 0.08,
        },
        BACKGROUND,
    );

    // The stand the screen sits on.
    fill_rect(
        &mut image,
        Rect {
            x: SIZE as f32 * 0.455,
            y: screen.y + screen.h,
            w: SIZE as f32 * 0.09,
            h: SIZE as f32 * 0.10,
        },
        SCREEN,
    );
    fill_rect(
        &mut image,
        Rect {
            x: SIZE as f32 * 0.30,
            y: screen.y + screen.h + SIZE as f32 * 0.10,
            w: SIZE as f32 * 0.40,
            h: SIZE as f32 * 0.05,
        },
        SCREEN,
    );

    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "packaging/AppIcon.png".to_string());
    if let Err(e) = image.save(&out) {
        eprintln!("could not write {out}: {e}");
        std::process::exit(1);
    }
    println!("wrote {out}");
}

/// A rectangle on the icon.
#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Fill a rectangle with a colour.
fn fill_rect(image: &mut RgbImage, rect: Rect, color: (u8, u8, u8)) {
    for y in rect.y as u32..(rect.y + rect.h) as u32 {
        for x in rect.x as u32..(rect.x + rect.w) as u32 {
            if x < image.width() && y < image.height() {
                image.put_pixel(x, y, Rgb(color.into()));
            }
        }
    }
}

/// Whether a point is inside a rounded square.
fn inside_rounded_square(x: f32, y: f32, size: f32, radius: f32) -> bool {
    let (cx, cy) = (x - size / 2.0, y - size / 2.0);
    let (half, half_y) = (size / 2.0 - radius, size / 2.0 - radius);
    let dx = (cx.abs() - half).max(0.0);
    let dy = (cy.abs() - half_y).max(0.0);
    dx * dx + dy * dy <= radius * radius
}
