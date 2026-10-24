//! End-to-end checks for the slide output path.
//!
//! These exercise the path the audience actually sees: a domain slide goes
//! through the shared render pipeline (the same one the NDI sender and the
//! recorder use) and is then handed to GPUI as an image.

use openpresenter::domain::props::PropContent;
use openpresenter::domain::{
    Background, Color, Mask, Object, Prop, ShapeType, Slide, SlideContent,
};
use openpresenter::render::{Overlay, RenderPipeline};

/// A slide with a coloured rectangle on a black background.
fn slide_with_rectangle() -> Slide {
    let mut shape = Object::new_shape(ShapeType::Rectangle);
    if let openpresenter::domain::ObjectContent::Shape { fill, .. } = &mut shape.content {
        *fill = Color {
            r: 200,
            g: 40,
            b: 40,
            a: 255,
        };
    }
    shape.position_x = 0.5;
    shape.position_y = 0.5;
    shape.width = 0.5;
    shape.height = 0.5;

    Slide {
        id: "test-slide".to_string(),
        content: SlideContent::Text {
            text: "Test".to_string(),
            style: Default::default(),
        },
        background: Background::Solid(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }),
        transition: Default::default(),
        group: None,
        notes: None,
        layers: vec![shape],
        cues: Vec::new(),
    }
}

#[test]
fn the_pipeline_draws_the_shape_into_the_frame() {
    let mut pipeline = RenderPipeline::new_software(64, 64);
    let frame = pipeline
        .render_slide(&slide_with_rectangle())
        .expect("the slide renders");

    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 64);

    // The middle of the slide is inside the rectangle.
    let middle = frame.pixel(32, 32).expect("the middle pixel exists");
    assert_eq!(middle.0, 200, "red channel of the fill");
    assert_eq!(middle.1, 40, "green channel of the fill");
    assert_eq!(middle.2, 40, "blue channel of the fill");

    // The corner is outside the rectangle and stays on the background.
    let corner = frame.pixel(1, 1).expect("the corner pixel exists");
    assert_eq!(corner.0, 0);
    assert_eq!(corner.1, 0);
    assert_eq!(corner.2, 0);
}

#[test]
fn a_composed_frame_becomes_a_gpu_image() {
    let mut pipeline = RenderPipeline::new_software(64, 64);
    let frame = pipeline
        .render_slide(&slide_with_rectangle())
        .expect("the slide renders");

    // The bridge lives in the UI layer, so reach it through the crate's
    // public surface: the conversion is exercised by the unit test in
    // `ui::compositor`; here we assert the frame itself is convertible.
    let buffer = image::RgbaImage::from_raw(frame.width, frame.height, frame.data.as_ref().clone())
        .expect("the frame is a valid RGBA buffer");
    assert_eq!(buffer.dimensions(), (64, 64));

    // The render frame stores BGRA (GPUI's native order), so the red fill
    // shows up in the blue channel of the raw bytes.
    let raw = buffer.as_raw();
    let offset = (32 * 64 + 32) * 4;
    assert_eq!(&raw[offset..offset + 4], &[40, 40, 200, 255]);
}

#[test]
fn a_prop_is_composited_on_top_of_the_slide() {
    let mut pipeline = RenderPipeline::new_software(64, 64);
    let slide = slide_with_rectangle();

    // A green prop covers the top-left corner of the slide.
    let prop = Prop {
        id: "prop".to_string(),
        name: "Badge".to_string(),
        content: PropContent::Rectangle {
            color: [0.0, 1.0, 0.0, 1.0],
            corner_radius: 0.0,
        },
        x: 0.0,
        y: 0.0,
        width: 0.5,
        height: 0.5,
        visible: true,
    };
    let overlay = Overlay {
        props: vec![prop],
        mask: Mask::None,
        live_message: None,
        ..Overlay::default()
    };

    let frame = pipeline
        .render_slide_with_overlay(&slide, None, &overlay)
        .expect("the slide with an overlay renders");

    // The prop wins over the slide underneath it.
    let covered = frame.pixel(8, 8).expect("the pixel exists");
    assert_eq!(covered.1, 255, "the green prop is on top");
    assert_eq!(covered.0, 0);

    // Outside the prop the slide is unchanged. The slide's rectangle covers
    // the middle half of the frame, so (40, 40) is on the rectangle but
    // outside the prop that covers the top-left quarter.
    let outside = frame.pixel(40, 40).expect("the pixel exists");
    assert_eq!(outside.0, 200, "the slide's rectangle is still there");
}

#[test]
fn a_letterbox_mask_frames_the_output() {
    let mut pipeline = RenderPipeline::new_software(64, 64);
    let slide = slide_with_rectangle();
    let overlay = Overlay {
        props: Vec::new(),
        mask: Mask::Letterbox { bar_fraction: 0.25 },
        live_message: None,
        ..Overlay::default()
    };

    let frame = pipeline
        .render_slide_with_overlay(&slide, None, &overlay)
        .expect("the masked slide renders");

    // The top bar is black...
    let bar = frame.pixel(32, 4).expect("the pixel exists");
    assert_eq!(bar.0, 0);
    assert_eq!(bar.1, 0);
    assert_eq!(bar.2, 0);

    // ...and the middle of the slide is untouched.
    let middle = frame.pixel(32, 32).expect("the pixel exists");
    assert_eq!(middle.0, 200);
}

#[test]
fn a_screen_theme_replaces_the_slides_background() {
    use openpresenter::render::StyleOverride;

    let mut pipeline = RenderPipeline::new_software(64, 64);
    let slide = slide_with_rectangle();

    // Without an override the slide keeps its own background.
    let frame = pipeline
        .render_slide_with_overlay(&slide, None, &Overlay::default())
        .expect("the slide renders");
    let corner = frame.pixel(1, 1).expect("the corner pixel exists");
    assert_eq!(corner.0, 0, "the slide's own background");

    // A screen with a theme draws the slide on the theme's background.
    let style = StyleOverride {
        background: Some(Background::Solid(Color {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        })),
        ..StyleOverride::default()
    };
    let frame = pipeline
        .render_slide_styled(&slide, None, &Overlay::default(), &style)
        .expect("the styled slide renders");
    let corner = frame.pixel(1, 1).expect("the corner pixel exists");
    assert_eq!(corner.0, 10, "the theme's background is used");
    assert_eq!(corner.1, 20);
    assert_eq!(corner.2, 30);
}

#[test]
fn a_custom_mask_hides_the_output_it_covers() {
    use openpresenter::domain::Mask;

    // A white mask covers the whole frame, so the slide is hidden.
    let dir = std::env::temp_dir().join("openpresenter-mask-test");
    std::fs::create_dir_all(&dir).expect("the temp directory exists");
    let path = dir.join("mask.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 255, 255, 255]))
        .save(&path)
        .expect("the mask is written");

    let mut pipeline = RenderPipeline::new_software(16, 16);
    let slide = slide_with_rectangle();
    let overlay = Overlay {
        mask: Mask::Custom {
            path: path.to_string_lossy().into_owned(),
        },
        ..Overlay::default()
    };

    let frame = pipeline
        .render_slide_with_overlay(&slide, None, &overlay)
        .expect("the masked slide renders");

    // The slide's red is gone: the white mask hides everything.
    let middle = frame.pixel(8, 8).expect("a pixel");
    assert_eq!(middle.0, 0, "the mask hides the slide");
    assert_eq!(middle.1, 0);
    assert_eq!(middle.2, 0);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_video_object_stays_in_its_own_box() {
    use openpresenter::media::decoder::RgbaFrame;

    // A video object in the top-left quarter of the slide.
    let mut video = Object::new_shape(ShapeType::Rectangle);
    video.content = openpresenter::domain::ObjectContent::Video {
        path: String::from("clip.mp4"),
        thumbnail: None,
    };
    video.position_x = 0.25;
    video.position_y = 0.25;
    video.width = 0.5;
    video.height = 0.5;

    let mut slide = slide_with_rectangle();
    slide.layers.push(video);

    // A fully red video frame. The frame is RGBA and the picture is BGRA, so
    // a green frame would hide a swapped red and blue: the red is what tells
    // the two apart.
    let frame_data = vec![0u8; 16 * 16 * 4];
    let mut data = frame_data;
    for px in data.as_chunks_mut::<4>().0 {
        px[0] = 255;
        px[1] = 0;
        px[2] = 0;
        px[3] = 255;
    }
    let video_frame = RgbaFrame {
        data,
        width: 16,
        height: 16,
        pts_secs: 0.0,
    };

    let mut pipeline = RenderPipeline::new_software(64, 64);
    let composed = pipeline
        .render_slide_with_video(&slide, Some(&video_frame))
        .expect("the slide with a video renders");

    // Inside the video's box the red shows, and it is still red: a swapped
    // red and blue would show a blue clip.
    let inside = composed.pixel(16, 16).expect("a pixel");
    assert_eq!(inside.0, 255, "the video fills its own box");
    assert_eq!(inside.2, 0, "the clip keeps its colours");

    // Outside the video's box the slide's own rectangle is still there: the
    // rectangle covers the middle half of the frame, the video only its
    // top-left quarter.
    let outside = composed.pixel(40, 40).expect("a pixel");
    assert_eq!(outside.1, 40, "the video does not cover the whole slide");
    assert_eq!(outside.0, 200, "the slide's own colour is untouched");
    assert_eq!(outside.2, 40, "the slide's own colour is untouched");
}

#[test]
fn an_image_object_is_drawn_into_its_box() {
    use openpresenter::domain::{ImageFit, Object};

    // A green image.
    let dir = std::env::temp_dir().join("openpresenter-image-test");
    std::fs::create_dir_all(&dir).expect("the temp directory exists");
    let path = dir.join("image.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]))
        .save(&path)
        .expect("the image is written");

    // A slide whose only object is the image, covering half the frame.
    let mut image_object = Object::new_image(path.to_string_lossy().into_owned(), ImageFit::Fill);
    image_object.width = 0.5;
    image_object.height = 0.5;
    image_object.position_x = 0.25;
    image_object.position_y = 0.25;

    let mut slide = slide_with_rectangle();
    slide.layers.push(image_object);

    let mut pipeline = RenderPipeline::new_software(64, 64);
    let frame = pipeline
        .render_slide(&slide)
        .expect("the slide with an image renders");

    // Inside the image's box the green shows.
    let inside = frame.pixel(16, 16).expect("a pixel");
    assert_eq!(inside.1, 255, "the image is drawn in its own box");

    // Outside it the slide's own rectangle is still there: the rectangle
    // covers the middle half of the frame, the image only its top-left
    // quarter.
    let outside = frame.pixel(40, 40).expect("a pixel");
    assert_eq!(outside.1, 40, "the image does not cover the whole slide");

    let _ = std::fs::remove_dir_all(&dir);
}

/// The image fit has to reach the output. `Fit` keeps the whole image
/// inside the box, so a wide image in a square box leaves the top and the
/// bottom of the box empty; `Stretch` fills the box instead. The editor
/// draws the same way, so the operator sees what the audience sees.
#[test]
fn the_image_fit_reaches_the_output() {
    use openpresenter::domain::{ImageFit, Object};

    // A wide image: twice as wide as it is high.
    let dir = std::env::temp_dir().join("openpresenter-image-fit-test");
    std::fs::create_dir_all(&dir).expect("the temp directory exists");
    let path = dir.join("wide.png");
    image::RgbaImage::from_pixel(32, 16, image::Rgba([0, 255, 0, 255]))
        .save(&path)
        .expect("the image is written");

    let render = |fit: ImageFit| {
        let mut object = Object::new_image(path.to_string_lossy().into_owned(), fit);
        // A box that is as wide as the frame and as tall as the frame, so
        // the letterboxing of a wide image is visible.
        object.width = 1.0;
        object.height = 1.0;
        object.position_x = 0.5;
        object.position_y = 0.5;

        let mut slide = slide_with_rectangle();
        slide.layers.clear();
        slide.layers.push(object);
        let mut pipeline = RenderPipeline::new_software(64, 64);
        pipeline
            .render_slide(&slide)
            .expect("the slide with an image renders")
    };

    // Stretched: the image fills the box, so the top row is the image.
    let stretched = render(ImageFit::Stretch);
    let top = stretched.pixel(32, 2).expect("a pixel");
    assert_eq!(top.1, 255, "a stretched image fills its box");

    // Fitted: the image keeps its shape and sits in the middle of the box,
    // so the top row is the slide's background instead.
    let fitted = render(ImageFit::Fit);
    let top = fitted.pixel(32, 2).expect("a pixel");
    assert_ne!(top.1, 255, "a fitted image is letterboxed in its box");
    let middle = fitted.pixel(32, 32).expect("a pixel");
    assert_eq!(middle.1, 255, "the image is in the middle of its box");

    let _ = std::fs::remove_dir_all(&dir);
}

/// A centred image keeps its shape.
///
/// The editor draws a centred image at its own size, scaled down when it is
/// too big for its box. A projector that squeezed it into the box would show
/// a distorted picture that the editor never showed.
#[test]
fn a_centred_image_keeps_its_shape() {
    use openpresenter::domain::{ImageFit, Object};

    // A wide image: four times as wide as it is high.
    let dir = std::env::temp_dir().join("openpresenter-image-centre-test");
    std::fs::create_dir_all(&dir).expect("the temp directory exists");
    let path = dir.join("wide.png");
    image::RgbaImage::from_pixel(64, 16, image::Rgba([0, 255, 0, 255]))
        .save(&path)
        .expect("the image is written");

    let mut object = Object::new_image(path.to_string_lossy().into_owned(), ImageFit::Center);
    // A tall, narrow box: the image does not fit, so it is scaled down.
    object.width = 0.25;
    object.height = 1.0;
    object.position_x = 0.5;
    object.position_y = 0.5;

    let mut slide = slide_with_rectangle();
    slide.layers.clear();
    slide.layers.push(object);
    let mut pipeline = RenderPipeline::new_software(64, 64);
    let frame = pipeline
        .render_slide(&slide)
        .expect("the slide with an image renders");

    // The image is 64×16 in a 16×64 box, so it is scaled down to 16×4 and
    // centred. A squashed image would be 16×16 — twice as tall as it should
    // be — and would reach further up and down the box.
    let mut rows_with_image: Vec<u32> = Vec::new();
    for y in 0..64 {
        let pixel = frame.pixel(32, y).expect("a pixel");
        if pixel.1 == 255 {
            rows_with_image.push(y);
        }
    }
    assert_eq!(
        rows_with_image.len(),
        4,
        "the image keeps its shape: four rows of a four-to-one picture, not {rows_with_image:?}"
    );
    assert_eq!(
        rows_with_image.first().copied(),
        Some(30),
        "the image sits in the middle of its box"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The slide's background image covers the frame, the way the editor draws
/// it: the picture keeps its shape and the overflow is cropped. Stretching it
/// to the frame would show a distorted picture on the projector that the
/// editor never showed.
#[test]
fn the_background_image_covers_the_frame_without_distorting_it() {
    use openpresenter::domain::{Background, SlideContent};

    let dir = std::env::temp_dir().join("openpresenter-background-test");
    std::fs::create_dir_all(&dir).expect("the temp directory exists");
    let path = dir.join("wide.png");

    // A wide image with a red stripe down its left edge. Covering a square
    // frame with it crops the sides away, so the stripe is not on the
    // frame; stretching it to the frame would put the stripe there.
    let mut image = image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 200, 0, 255]));
    for y in 0..32 {
        for x in 0..8 {
            image.put_pixel(x, y, image::Rgba([220, 0, 0, 255]));
        }
    }
    image.save(&path).expect("the image is written");

    let mut slide = slide_with_rectangle();
    slide.content = SlideContent::Text {
        text: String::new(),
        style: Default::default(),
    };
    slide.layers.clear();
    slide.background = Background::Image(path.to_string_lossy().into_owned());

    let mut pipeline = RenderPipeline::new_software(100, 100);
    let frame = pipeline.render_slide(&slide).expect("the slide renders");

    // The image covers the whole frame, so every corner is the image.
    for (x, y) in [(0u32, 0u32), (99, 0), (0, 99), (99, 99), (50, 50)] {
        let pixel = frame.pixel(x, y).expect("a pixel");
        assert_eq!(pixel.1, 200, "the background covers the frame at ({x},{y})");
    }

    // And the stripe was cropped away, not squashed onto the frame.
    let left = frame.pixel(0, 50).expect("a pixel");
    assert_eq!(
        left.0, 0,
        "the image is cropped, not stretched: the left stripe must not show"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A rounded rectangle keeps its corners on the output.
///
/// The editor draws the corner radius, so a square on the projector would
/// show a slide the editor never showed.
/// A shape that hangs over the edge of the slide is drawn where it is.
///
/// The box keeps its place and the part that hangs over is clipped: a shape
/// that was pushed back onto the slide would be drawn somewhere else than
/// the model says, on the projector as well as in the editor.
#[test]
fn a_shape_at_the_edge_keeps_its_place() {
    use openpresenter::domain::{Object, ObjectContent, ShapeType};

    let mut shape = Object::new_shape(ShapeType::Rectangle);
    if let ObjectContent::Shape { fill, .. } = &mut shape.content {
        *fill = Color {
            r: 200,
            g: 40,
            b: 40,
            a: 255,
        };
    }
    // The shape's centre is at 15% of the slide, and it is 90% wide, so a
    // third of it hangs over the left edge.
    shape.position_x = 0.15;
    shape.position_y = 0.5;
    shape.width = 0.9;
    shape.height = 0.2;

    let mut slide = slide_with_rectangle();
    slide.layers.clear();
    slide.layers.push(shape);

    let mut pipeline = RenderPipeline::new_software(100, 100);
    let frame = pipeline
        .render_slide(&slide)
        .expect("the slide with the shape renders");

    // The shape's right edge is at 15% + 45% = 60% of the slide.
    let inside = frame.pixel(59, 50).expect("a pixel");
    assert_eq!(inside.0, 200, "the shape reaches its own right edge");
    let outside = frame.pixel(61, 50).expect("a pixel");
    assert_eq!(
        outside.0, 0,
        "the shape does not reach past the edge the model gives it"
    );
}

#[test]
fn a_rounded_rectangle_keeps_its_corners() {
    use openpresenter::domain::{Object, ObjectContent, ShapeType};

    let mut slide = slide_with_rectangle();
    slide.layers.clear();

    let mut shape = Object::new_shape(ShapeType::Rectangle);
    shape.width = 0.5;
    shape.height = 0.5;
    shape.position_x = 0.5;
    shape.position_y = 0.5;
    if let ObjectContent::Shape {
        ref mut corner_radius,
        ref mut fill,
        ..
    } = shape.content
    {
        *corner_radius = 20.0;
        *fill = openpresenter::domain::Color {
            r: 0,
            g: 200,
            b: 0,
            a: 255,
        };
    }
    slide.layers.push(shape);

    let mut pipeline = RenderPipeline::new_software(100, 100);
    let frame = pipeline.render_slide(&slide).expect("the slide renders");

    // The middle of the shape is the shape's colour.
    let middle = frame.pixel(50, 50).expect("a pixel");
    assert_eq!(middle.1, 200, "the shape is filled");

    // The corner is outside the rounded shape, so the slide's own
    // background shows there.
    let corner = frame.pixel(26, 26).expect("a pixel");
    assert_ne!(
        corner.1, 200,
        "the corner of a rounded rectangle is not part of the shape"
    );

    // A point inside the shape, away from the corner, is filled.
    let inside = frame.pixel(40, 50).expect("a pixel");
    assert_eq!(inside.1, 200, "the shape is filled away from the corner");
}
