//! Writing a presentation out as a PDF.
//!
//! A PDF is a small text file with the pages' pictures inside it, so the
//! export is: draw each slide, encode the picture, and write the file. The
//! slides are drawn with the same pipeline the projector uses, so a printed
//! slide is the slide the audience saw.
//!
//! The pictures are JPEG, because a PDF holds a JPEG as it is: an image that
//! is encoded once is encoded once.

use crate::render::Frame;

/// A page of the document, with the picture that is on it.
struct Page {
    /// The picture, as JPEG.
    jpeg: Vec<u8>,
    /// The picture's width in pixels.
    width: u32,
    /// The picture's height in pixels.
    height: u32,
}

/// Write the slides out as a PDF.
///
/// The pages are the slides, in order, each as large as the slide is. The
/// size is in points — seventy-two to the inch — so a 16:9 slide is a wide
/// page and the printed order is the size the operator expects.
pub fn write(slides: &[Frame], path: &std::path::Path) -> anyhow::Result<()> {
    let mut pages = Vec::with_capacity(slides.len());
    for frame in slides {
        pages.push(Page {
            jpeg: encode_jpeg(frame)?,
            width: frame.width,
            height: frame.height,
        });
    }
    let bytes = build(&pages);
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Draw the presentation and write it out as a PDF.
///
/// The slides are drawn with the same pipeline the projector uses, so the
/// document is the show the audience saw.
pub fn render_presentation(
    presentation: &crate::domain::Presentation,
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let mut pipeline = crate::render::RenderPipeline::new(width.max(16), height.max(16));
    let mut frames = Vec::with_capacity(presentation.slides.len());
    for slide in &presentation.slides {
        frames.push(pipeline.render_slide(slide)?);
    }
    write(&frames, path)
}

/// Encode a frame as a JPEG.
fn encode_jpeg(frame: &Frame) -> anyhow::Result<Vec<u8>> {
    let mut rgb = vec![0u8; (frame.width * frame.height * 3) as usize];
    for (index, pixel) in rgb.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let source = index * 4;
        // The frame is BGRA and the JPEG is RGB.
        pixel[0] = frame.data[source + 2];
        pixel[1] = frame.data[source + 1];
        pixel[2] = frame.data[source];
    }
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92);
    encoder.encode(
        &rgb,
        frame.width,
        frame.height,
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(out)
}

/// The size of a page, in points.
///
/// The page keeps the slide's shape: a slide that is wide is printed wide,
/// so the operator's order matches the projector.
fn page_size(page: &Page) -> (u32, u32) {
    const HEIGHT_POINTS: u32 = 540;
    let width = (page.width as u64 * HEIGHT_POINTS as u64 / page.height.max(1) as u64) as u32;
    (width.max(1), HEIGHT_POINTS)
}

/// Build the PDF.
fn build(pages: &[Page]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut offsets: Vec<usize> = Vec::new();

    // The header.
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    // The catalogue and the page tree.
    let object = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(out.len());
        out.extend_from_slice(body);
    };

    let kids: Vec<String> = (0..pages.len())
        .map(|index| format!("{} 0 R", 3 + index * 3))
        .collect();
    object(
        &mut out,
        &mut offsets,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );
    object(
        &mut out,
        &mut offsets,
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.join(" "),
            pages.len()
        )
        .as_bytes(),
    );

    for (index, page) in pages.iter().enumerate() {
        let page_object = 3 + index * 3;
        let image_object = page_object + 1;
        let content_object = page_object + 2;
        let (width, height) = page_size(page);

        // The page.
        object(
            &mut out,
            &mut offsets,
            format!(
                "{page_object} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
                 /Resources << /XObject << /Im0 {image_object} 0 R >> /ProcSet [/PDF /ImageC] >> \
                 /Contents {content_object} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        // The picture.
        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{image_object} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} \
                 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
                page.width,
                page.height,
                page.jpeg.len()
            )
            .as_bytes(),
        );
        out.extend_from_slice(&page.jpeg);
        out.extend_from_slice(b"\nendstream\nendobj\n");

        // The content that draws the picture over the whole page.
        let content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q\n");
        offsets.push(out.len());
        out.extend_from_slice(
            format!(
                "{content_object} 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
                content.len()
            )
            .as_bytes(),
        );
    }

    // The cross-reference table.
    let start_xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start_xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The PDF has the parts a reader looks for.
    #[test]
    fn the_pdf_has_a_catalogue_and_the_pages() {
        let mut frame = Frame::new(64, 36);
        frame.set_pixel(0, 0, 255, 0, 0, 255);
        let jpeg = encode_jpeg(&frame).expect("the picture is encoded");
        let bytes = build(&[Page {
            jpeg,
            width: 64,
            height: 36,
        }]);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/Type /Pages"));
        assert!(text.contains("/Type /Page"));
        assert!(text.contains("xref"));
        assert!(text.trim_end().ends_with("%%EOF"));
    }

    /// The page keeps the slide's shape.
    #[test]
    fn the_page_keeps_the_slides_shape() {
        let page = Page {
            jpeg: Vec::new(),
            width: 1920,
            height: 1080,
        };
        let (width, height) = page_size(&page);
        assert_eq!(height, 540);
        // Sixteen by nine is a wide page.
        assert!(width > height);
        assert_eq!(width, 960);
    }
}

#[cfg(test)]
mod round_trip_tests {
    use super::*;

    /// A written PDF is a PDF the system can read.
    ///
    /// The document is handed to the system to print, so a file that the
    /// system cannot open is a file the operator cannot print.
    #[test]
    fn a_written_pdf_is_read_by_the_system() {
        let mut slide = crate::domain::Slide::new_text(String::from("A slide"));
        slide.background = crate::domain::Background::Solid(crate::domain::Color {
            r: 20,
            g: 120,
            b: 200,
            a: 255,
        });
        let mut pipeline = crate::render::RenderPipeline::new_software(320, 180);
        let frame = pipeline.render_slide(&slide).expect("the slide is drawn");

        let path = std::env::temp_dir().join("openpresenter-pdf-test.pdf");
        write(&[frame], &path).expect("the pdf is written");
        let bytes = std::fs::read(&path).expect("the pdf is there");
        assert!(bytes.len() > 1000, "the pdf holds the picture");
        assert!(bytes.starts_with(b"%PDF-1.4"));

        // The system reads it: the file is a PDF and not a picture with a
        // PDF's name.
        let out = std::process::Command::new("mdls")
            .args([
                "-name",
                "kMDItemContentType",
                path.to_string_lossy().as_ref(),
            ])
            .output();
        if let Ok(out) = out {
            let text = String::from_utf8_lossy(&out.stdout);
            assert!(
                text.contains("com.adobe.pdf"),
                "the system reads the document as a PDF (got {text})"
            );
        }
        let _ = std::fs::remove_file(&path);
    }
}
