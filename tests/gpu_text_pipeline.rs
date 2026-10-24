//! The GPU text path, end to end.
//!
//! The type is rendered into a texture of its own size and only that region
//! is read back, so the picture has to be the same as it would be if the
//! whole frame were rendered: the glyphs have to land where the slide says
//! they are.

use openpresenter::domain::{Color, TextAlignment, TextStyle};
use openpresenter::render::{GpuTextRenderer, create_gpu_text_renderer};

/// The text lands where the slide puts it.
#[test]
fn the_rendered_text_lands_where_the_slide_puts_it() {
    let Ok(mut renderer) = create_gpu_text_renderer() else {
        // No GPU in this environment: nothing to check.
        return;
    };
    let style = TextStyle {
        font_size: 72.0,
        color: Color::white(),
        ..Default::default()
    };

    let width = 1920u32;
    let height = 1080u32;
    let overlay = renderer
        .render_text("Hello", &style, &[], width, height)
        .expect("the text renders");

    // The glyphs are somewhere in the overlay.
    let lit: Vec<(u32, u32)> = overlay
        .pixels
        .chunks(4)
        .enumerate()
        .filter(|(_, px)| px[3] > 0)
        .map(|(idx, _)| {
            let px = idx as u32;
            (
                overlay.origin_x + px % overlay.width,
                overlay.origin_y + px / overlay.width,
            )
        })
        .collect();
    assert!(!lit.is_empty(), "the text is drawn");

    // The middle of the text is the middle of the frame: the anchor is the
    // slide's position.
    let min_x = lit.iter().map(|(x, _)| *x).min().unwrap();
    let max_x = lit.iter().map(|(x, _)| *x).max().unwrap();
    let min_y = lit.iter().map(|(_, y)| *y).min().unwrap();
    let max_y = lit.iter().map(|(_, y)| *y).max().unwrap();
    let mid_x = (min_x + max_x) / 2;
    let mid_y = (min_y + max_y) / 2;
    assert!(
        (mid_x as i32 - 960).abs() < 12,
        "the text is centred horizontally (got {mid_x})"
    );
    assert!(
        (mid_y as i32 - 540).abs() < 12,
        "the text is centred vertically (got {mid_y})"
    );
}

/// Only the text's own region is read back, not the whole frame.
///
/// A full-frame readback per text object copies megabytes for every line on
/// the slide, and that copying is what makes a 4K output stutter.
#[test]
fn only_the_texts_own_region_is_read_back() {
    let Ok(mut renderer) = create_gpu_text_renderer() else {
        return;
    };
    let style = TextStyle {
        font_size: 72.0,
        color: Color::white(),
        ..Default::default()
    };

    let width = 1920u32;
    let height = 1080u32;
    let overlay = renderer
        .render_text("Hello", &style, &[], width, height)
        .expect("the text renders");

    assert!(
        overlay.width < width / 4 && overlay.height < height / 4,
        "the region is a small part of the frame ({}×{} in {width}×{height})",
        overlay.width,
        overlay.height
    );
    assert_eq!(
        overlay.pixels.len() as u32,
        overlay.width * overlay.height * 4,
        "the overlay carries exactly its own pixels"
    );
}

/// The text is composited into the frame where it belongs.
#[test]
fn the_overlay_is_composited_at_its_own_origin() {
    let Ok(mut renderer) = create_gpu_text_renderer() else {
        return;
    };
    let style = TextStyle {
        font_size: 72.0,
        color: Color::white(),
        alignment: TextAlignment::Left,
        ..Default::default()
    };

    let width = 1920u32;
    let height = 1080u32;
    let overlay = renderer
        .render_text("Hello", &style, &[], width, height)
        .expect("the text renders");

    // A black frame, and the text drawn into it.
    let mut frame = vec![0u8; (width * height * 4) as usize];
    GpuTextRenderer::composite_bgra_at(&mut frame, &overlay, width);

    let lit: Vec<(u32, u32)> = frame
        .chunks(4)
        .enumerate()
        .filter(|(_, px)| px[3] > 0)
        .map(|(idx, _)| {
            let px = idx as u32;
            (px % width, px / width)
        })
        .collect();
    assert!(!lit.is_empty(), "the text is in the frame");

    // The text is left-aligned at the anchor, so it starts there and runs
    // to the right.
    let anchor_x = (style.position_x * width as f32) as u32;
    let min_x = lit.iter().map(|(x, _)| *x).min().unwrap();
    assert!(
        (min_x as i32 - anchor_x as i32).abs() < 12,
        "the text starts at the anchor (got {min_x}, anchor {anchor_x})"
    );
}

/// The software renderer and the GPU renderer have to put the text in the
/// same place.
///
/// The software renderer is the fallback — the headless path and the tests
/// use it — so if the two disagree about where a line of text belongs, the
/// tests cannot see the regressions that the GPU path would show.
#[test]
fn the_two_renderers_agree_on_where_the_text_goes() {
    use openpresenter::domain::{Object, ObjectContent, Slide};
    use openpresenter::render::RenderPipeline;

    let Ok(mut gpu) = create_gpu_text_renderer() else {
        return;
    };

    let style = TextStyle {
        font_size: 72.0,
        color: Color::white(),
        ..Default::default()
    };

    // The GPU path.
    let overlay = gpu
        .render_text("Hello", &style, &[], 1920, 1080)
        .expect("the text renders");
    let gpu_lit: Vec<(u32, u32)> = overlay
        .pixels
        .chunks(4)
        .enumerate()
        .filter(|(_, px)| px[3] > 0)
        .map(|(idx, _)| {
            let px = idx as u32;
            (
                overlay.origin_x + px % overlay.width,
                overlay.origin_y + px / overlay.width,
            )
        })
        .collect();
    assert!(!gpu_lit.is_empty());

    // The software path, on a slide with the same text at the same place.
    let mut slide = Slide::new_text(String::new());
    slide.layers.clear();
    let mut object = Object::new_text("Hello".to_string());
    object.position_x = style.position_x;
    object.position_y = style.position_y;
    if let ObjectContent::Text {
        style: ref mut s, ..
    } = object.content
    {
        *s = style.clone();
    }
    slide.layers.push(object);

    let mut software = RenderPipeline::new_software(1920, 1080);
    let frame = software.render_slide(&slide).expect("the slide renders");
    let software_lit: Vec<(u32, u32)> = frame
        .data
        .chunks(4)
        .enumerate()
        .filter(|(_, px)| px[3] > 0)
        .map(|(idx, _)| {
            let px = idx as u32;
            (px % 1920, px / 1920)
        })
        .collect();
    assert!(
        !software_lit.is_empty(),
        "the software renderer draws the text"
    );

    // The two fonts are different, so the text is a different shape — but
    // it sits in the same place, and that is what the slide says.
    let gpu_mid = |axis: fn(&(u32, u32)) -> u32| {
        let min = gpu_lit.iter().map(axis).min().unwrap();
        let max = gpu_lit.iter().map(axis).max().unwrap();
        (min + max) / 2
    };
    let software_mid = |axis: fn(&(u32, u32)) -> u32| {
        let min = software_lit.iter().map(axis).min().unwrap();
        let max = software_lit.iter().map(axis).max().unwrap();
        (min + max) / 2
    };

    let anchor_x = (style.position_x * 1920.0) as u32;
    let anchor_y = (style.position_y * 1080.0) as u32;
    assert!(
        (gpu_mid(|p| p.0) as i32 - anchor_x as i32).abs() < 12,
        "the GPU text is centred on the anchor"
    );
    assert!(
        (software_mid(|p| p.0) as i32 - anchor_x as i32).abs() < 12,
        "the software text is centred on the anchor too"
    );
    assert!(
        (gpu_mid(|p| p.1) as i32 - anchor_y as i32).abs() < 20,
        "the GPU text is centred vertically on the anchor"
    );
    assert!(
        (software_mid(|p| p.1) as i32 - anchor_y as i32).abs() < 20,
        "the software text is centred vertically on the anchor too"
    );
}

/// The real output path: a slide rendered with the GPU, the way the
/// compositor draws it for the projector.
///
/// The shapes are rasterised on the CPU and the type is rendered on the GPU,
/// so this is the composition the audience actually sees.
#[test]
fn the_output_pipeline_draws_the_text_with_the_gpu() {
    use openpresenter::domain::{Object, ObjectContent, Slide};
    use openpresenter::render::RenderPipeline;

    let mut slide = Slide::new_text(String::new());
    slide.layers.clear();
    let mut object = Object::new_text("Hello".to_string());
    object.position_x = 0.5;
    object.position_y = 0.5;
    if let ObjectContent::Text {
        style: ref mut s, ..
    } = object.content
    {
        s.font_size = 96.0;
        s.color = Color::white();
    }
    slide.layers.push(object);

    let mut pipeline = RenderPipeline::new(1920, 1080);
    let frame = pipeline.render_slide(&slide).expect("the slide renders");

    let lit: Vec<(u32, u32)> = frame
        .data
        .chunks(4)
        .enumerate()
        .filter(|(_, px)| px[3] > 0)
        .map(|(idx, _)| {
            let px = idx as u32;
            (px % 1920, px / 1920)
        })
        .collect();
    assert!(!lit.is_empty(), "the text is drawn on the output");

    let min_x = lit.iter().map(|(x, _)| *x).min().unwrap();
    let max_x = lit.iter().map(|(x, _)| *x).max().unwrap();
    let min_y = lit.iter().map(|(_, y)| *y).min().unwrap();
    let max_y = lit.iter().map(|(_, y)| *y).max().unwrap();
    let mid_x = (min_x + max_x) / 2;
    let mid_y = (min_y + max_y) / 2;
    assert!(
        (mid_x as i32 - 960).abs() < 16,
        "the text is centred on the slide (got {mid_x})"
    );
    assert!(
        (mid_y as i32 - 540).abs() < 24,
        "the text is centred vertically (got {mid_y})"
    );
}

/// The text's effects reach the output: the shadow is the glyphs shifted
/// and turned black, the stroke is the glyphs dilated.
#[test]
fn the_text_effects_reach_the_output() {
    use openpresenter::domain::{Background, Object, ObjectContent, Slide};
    use openpresenter::render::RenderPipeline;

    // The background is a mid grey, because the shadow is black: on a black
    // background it would be invisible, and the test would prove nothing.
    const BG: (u8, u8, u8) = (128, 128, 128);

    let build = |shadow: bool, stroke: f32| {
        let mut slide = Slide::new_text(String::new());
        slide.layers.clear();
        slide.background = Background::Solid(Color {
            r: BG.0,
            g: BG.1,
            b: BG.2,
            a: 255,
        });
        let mut object = Object::new_text("Hello".to_string());
        object.position_x = 0.5;
        object.position_y = 0.5;
        if let ObjectContent::Text {
            style: ref mut s, ..
        } = object.content
        {
            s.font_size = 96.0;
            s.color = Color::white();
            s.shadow = shadow;
            s.text_stroke_width = stroke;
            s.text_stroke_color = Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            };
        }
        slide.layers.push(object);
        slide
    };

    // The pixels that are not the background: the text and its effects.
    let count_of = |slide: &openpresenter::domain::Slide| {
        let mut pipeline = RenderPipeline::new(1920, 1080);
        let frame = pipeline.render_slide(slide).expect("the slide renders");
        frame
            .data
            .chunks(4)
            .filter(|px| (px[0], px[1], px[2]) != (BG.0, BG.1, BG.2))
            .count()
    };

    let plain = count_of(&build(false, 0.0));
    let shadowed = count_of(&build(true, 0.0));
    let stroked = count_of(&build(false, 4.0));

    assert!(
        shadowed > plain,
        "the shadow adds pixels (plain {plain}, shadowed {shadowed})"
    );
    assert!(
        stroked > plain,
        "the stroke adds pixels (plain {plain}, stroked {stroked})"
    );
}
