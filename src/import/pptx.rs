//! Importing a PowerPoint presentation.
//!
//! A `.pptx` is a zip of XML: the slides are DrawingML, the pictures are
//! files in the package, and the sizes are in EMU — English Metric Units,
//! 914400 to the inch. Everything the importer reads is converted into the
//! model's own units: a position is a fraction of the slide, and a font size
//! is in points.
//!
//! The importer keeps what a slide really says: its text, its pictures and
//! where they sit. A deck can come in three ways — as editable objects, as
//! pictures of the slides, or as a whole presentation with the transitions
//! and the notes the file carried. The animation and the slide master are
//! not carried over in any of them: a presentation that plays in a different
//! program than the one it was built in is a presentation that surprises the
//! operator.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::domain::{ImageFit, Object, ObjectContent, ShapeType, Slide, TextStyle};

/// The presentation read from a `.pptx` file.
pub struct Pptx {
    /// The slides, in the order the file lists them.
    pub slides: Vec<Slide>,
    /// The name of the presentation, taken from the file.
    pub name: String,
}

/// Read a `.pptx` file.
///
/// The pictures are written next to the presentation's database so the
/// slides can point at them: a slide whose picture is somewhere else on the
/// machine is a slide that shows a broken picture on the venue's computer.
pub fn import(path: &Path, media_dir: &Path) -> Result<Pptx> {
    import_with(path, media_dir, false)
}

/// Read a `.pptx` file, with or without the show's own furniture.
///
/// The show's furniture is the transition the slide carries and the notes
/// under it: a deck that is imported without them is a deck that has to
/// have its transitions set again, slide by slide.
pub fn import_with(path: &Path, media_dir: &Path, with_show_details: bool) -> Result<Pptx> {
    let file =
        std::fs::File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("{} is not a PowerPoint file", path.display()))?;

    let slide_size = slide_size(&mut archive).unwrap_or((12_192_000, 6_858_000));

    // The pictures are taken out of the package first: the slides refer to
    // them by the name they have inside it.
    let media = extract_media(&mut archive, media_dir)?;

    // The colours the deck's theme defines, and the relationships of each
    // slide: a picture and a colour are both named by the file that holds
    // them.
    let theme = read_theme_colors(&mut archive);

    let mut slides = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().unwrap_or_default().to_string();
        if !name.starts_with("ppt/slides/slide") || !name.ends_with(".xml") {
            continue;
        }
        let Some(number) = slide_number(&name) else {
            continue;
        };
        names.push(format!("{number:08}\u{0}{name}"));
    }
    names.sort();

    for entry_name in names {
        let name = entry_name.split('\u{0}').nth(1).unwrap_or(&entry_name);
        let mut xml = String::new();
        {
            let mut entry = archive.by_name(name)?;
            entry.read_to_string(&mut xml)?;
        }
        // The slide's own relationships: the picture it holds and the
        // layout it inherits from.
        let rels = read_relationships(&mut archive, name);
        let mut slide = parse_slide(&xml, slide_size, &media, &rels, &theme)?;
        if let Some(background) = read_background(&mut archive, name, &rels, &media, &theme) {
            slide.background = background;
        }
        if with_show_details {
            // The transition the slide carries, and the notes under it.
            slide.transition = read_transition(&xml);
            slide.notes = read_notes(&mut archive, name);
        }
        slides.push(slide);
    }

    if slides.is_empty() {
        bail!("{} holds no slides", path.display());
    }

    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("Presentation"));

    Ok(Pptx { slides, name })
}

/// The slides of a presentation, as pictures.
///
/// A presentation that is imported as pictures keeps its picture: the
/// slides are drawn with the app's own renderer, so the type is the real
/// type and the layout is the layout the file had. It cannot be edited
/// afterwards, so it is the import for a deck that is shown rather than
/// made.
pub fn import_as_images(
    presentation: &crate::domain::Presentation,
    media_dir: &std::path::Path,
    width: u32,
    height: u32,
) -> anyhow::Result<crate::domain::Presentation> {
    std::fs::create_dir_all(media_dir)?;
    // The pipeline with the GPU text renderer draws the slides in the real
    // fonts, which is the whole point of importing a deck as pictures: the
    // picture is the slide, and a pixel font is not the slide.
    let mut pipeline = crate::render::RenderPipeline::new(width.max(16), height.max(16));
    let mut out = crate::domain::Presentation::new(presentation.name.clone());
    for (index, slide) in presentation.slides.iter().enumerate() {
        let frame = pipeline.render_slide(slide)?;
        let path = media_dir.join(format!("pptx-slide-{index}.jpg"));
        write_jpeg(&frame, &path)?;
        // The picture is the slide's background: the slide itself is empty,
        // so nothing on it can be moved by accident. The transition and the
        // notes still come across: a deck that is shown rather than made is
        // a deck that is played as it was written.
        let mut imported = crate::domain::Slide::new_text(String::new());
        imported.background = crate::domain::Background::Image(path.to_string_lossy().into_owned());
        imported.group = slide.group.clone();
        imported.transition = slide.transition;
        imported.notes = slide.notes.clone();
        out.slides.push(imported);
    }
    Ok(out)
}

/// Write a frame out as a JPEG.
fn write_jpeg(frame: &crate::render::Frame, path: &std::path::Path) -> anyhow::Result<()> {
    let mut rgb = vec![0u8; (frame.width * frame.height * 3) as usize];
    for (index, pixel) in rgb.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let source = index * 4;
        // The frame is BGRA and the JPEG is RGB.
        pixel[0] = frame.data[source + 2];
        pixel[1] = frame.data[source + 1];
        pixel[2] = frame.data[source];
    }
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    encoder.encode(
        &rgb,
        frame.width,
        frame.height,
        image::ExtendedColorType::Rgb8,
    )?;
    std::fs::write(path, out)?;
    Ok(())
}

/// The transition a slide carries.
///
/// A PowerPoint slide names its transition and how fast it is; the speed is
/// one of three words, and each word is a length of time.
fn read_transition(xml: &str) -> crate::domain::Transition {
    let Some(start) = xml.find("<p:transition") else {
        return crate::domain::Transition::Cut;
    };
    let rest = &xml[start..];
    let end = rest.find("</p:transition>").unwrap_or(rest.len());
    let block = &rest[..end];

    // The speed is an attribute of the transition itself.
    let speed = if block.contains("spd=\"slow\"") {
        1200
    } else if block.contains("spd=\"fast\"") {
        350
    } else {
        700
    };
    let direction = if block.contains("dir=\"l\"") {
        Some("l")
    } else if block.contains("dir=\"r\"") {
        Some("r")
    } else if block.contains("dir=\"u\"") {
        Some("u")
    } else if block.contains("dir=\"d\"") {
        Some("d")
    } else {
        None
    };

    if block.contains("<p:fade") {
        return crate::domain::Transition::Fade { duration_ms: speed };
    }
    if block.contains("<p:dissolve") {
        return crate::domain::Transition::Dissolve { duration_ms: speed };
    }
    if block.contains("<p:push") {
        return crate::domain::Transition::Push {
            duration_ms: speed,
            direction: push_direction(direction),
        };
    }
    if block.contains("<p:wipe") {
        return crate::domain::Transition::Wipe {
            duration_ms: speed,
            angle_deg: sweep_angle(direction),
        };
    }
    if block.contains("<p:zoom") {
        return crate::domain::Transition::Zoom { duration_ms: speed };
    }
    if block.contains("<p:split") {
        return crate::domain::Transition::Flip { duration_ms: speed };
    }
    if block.contains("<p:cover") {
        return crate::domain::Transition::Slide { duration_ms: speed };
    }
    crate::domain::Transition::Cut
}

/// The direction a push goes in.
fn push_direction(direction: Option<&str>) -> u8 {
    match direction {
        Some("l") => 2,
        Some("r") => 0,
        Some("u") => 3,
        _ => 1,
    }
}

/// The angle a sweep goes at.
fn sweep_angle(direction: Option<&str>) -> i32 {
    match direction {
        Some("l") => 180,
        Some("r") => 0,
        Some("u") => 90,
        _ => 270,
    }
}

/// The notes under a slide.
///
/// The notes live in their own file, and the slide's relationships say which
/// one. A slide with no notes file has no notes.
fn read_notes<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    slide_name: &str,
) -> Option<String> {
    let slide_path = slide_name.strip_prefix("ppt/slides/")?;
    let rels_name = format!("ppt/slides/_rels/{slide_path}.rels");
    let mut rels = String::new();
    {
        let mut entry = archive.by_name(&rels_name).ok()?;
        entry.read_to_string(&mut rels).ok()?;
    }
    // The relationship that points at the notes.
    let target = rels
        .split("Relationship")
        .find(|part| part.contains("/notesSlide"))
        .and_then(|part| part.split("Target=\"").nth(1))
        .and_then(|part| part.split('"').next())?
        .to_string();
    let target = target.trim_start_matches("../");
    let notes_name = if target.starts_with("ppt/") {
        target.to_string()
    } else {
        format!("ppt/{target}")
    };
    let mut xml = String::new();
    {
        let mut entry = archive.by_name(&notes_name).ok()?;
        entry.read_to_string(&mut xml).ok()?;
    }
    // The notes are the text of the notes body, which is the shape whose
    // placeholder is of the notes type.
    let text = xml
        .split("<p:sp>")
        .find(|part| part.contains("type=\"body\""))
        .map(|part| {
            let mut out = String::new();
            for chunk in part.split("<a:t>").skip(1) {
                let value = chunk.split("</a:t>").next().unwrap_or("");
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(value);
            }
            out
        })
        .filter(|text| !text.trim().is_empty())?;
    Some(text)
}

/// The contents of a file in the package.
fn read_entry<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut xml = String::new();
    entry.read_to_string(&mut xml).ok()?;
    Some(xml)
}

/// The value of an attribute in a fragment of XML.
fn attribute(part: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = part.find(&needle)? + needle.len();
    let rest = &part[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The relationships of a file in the package.
///
/// A picture in a slide is named by a relationship id — `rId2` — and the
/// relationship says which file in the package the picture really is. A
/// slide whose picture cannot be resolved is a slide with a broken
/// picture, so the relationship is read rather than guessed at.
fn read_relationships<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    file_name: &str,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some((dir, base)) = file_name.rsplit_once('/') else {
        return map;
    };
    let Some(xml) = read_entry(archive, &format!("{dir}/_rels/{base}.rels")) else {
        return map;
    };
    for part in xml.split("<Relationship") {
        // The root element splits with the first relationship, so a part
        // without an id is not a relationship.
        let (Some(id), Some(target)) = (attribute(part, "Id"), attribute(part, "Target")) else {
            continue;
        };
        map.insert(id, target);
    }
    map
}

/// A path inside the package, resolved against the folder of the file that
/// names it.
fn resolve_path(base_dir: &str, target: &str) -> String {
    let mut parts: Vec<&str> = base_dir.split('/').filter(|p| !p.is_empty()).collect();
    for part in target.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// The file a relationship points at, as a path in the package.
fn related_file(base_dir: &str, rels: &HashMap<String, String>, kind: &str) -> Option<String> {
    let target = rels
        .iter()
        .find(|(_, target)| target.contains(&format!("/{kind}")))
        .map(|(_, target)| target.clone())?;
    Some(resolve_path(base_dir, &target))
}

/// The picture a relationship points at.
fn media_for(
    rels: &HashMap<String, String>,
    media: &HashMap<String, PathBuf>,
    embed: &str,
) -> Option<PathBuf> {
    let target = rels.get(embed)?;
    let file_name = target.rsplit('/').next()?;
    media.get(file_name).cloned()
}

/// The relationship a picture is named by.
fn blip_embed(block: &str) -> Option<String> {
    let start = block.find("<a:blip")?;
    let rest = &block[start..];
    let end = rest.find('>')?;
    attribute(&rest[..end], "embed")
}

/// A colour written as six hex digits.
fn parse_hex_color(value: &str) -> Option<crate::domain::Color> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(crate::domain::Color { r, g, b, a: 255 })
}

/// The theme's own name for the colour a slide asks for.
///
/// A slide says `tx1` for the text colour and `bg1` for the background,
/// while the theme calls them `dk1` and `lt1`.
fn scheme_name(name: &str) -> &str {
    match name {
        "tx1" => "dk1",
        "bg1" => "lt1",
        "tx2" => "dk2",
        "bg2" => "lt2",
        other => other,
    }
}

/// The colours the theme of the deck defines.
///
/// A slide names its colours by the theme's names, and the theme says what
/// those names mean. A colour read without the theme is a colour the
/// operator did not choose.
fn read_theme_colors<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> HashMap<String, crate::domain::Color> {
    let mut colors = HashMap::new();
    let Some(xml) = read_entry(archive, "ppt/theme/theme1.xml") else {
        return colors;
    };
    let Some(start) = xml.find("<a:clrScheme") else {
        return colors;
    };
    let rest = &xml[start..];
    let end = rest.find("</a:clrScheme>").unwrap_or(rest.len());
    let scheme = &rest[..end];

    for name in [
        "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink",
    ] {
        let open = format!("<a:{name}>");
        let close = format!("</a:{name}>");
        let Some(begin) = scheme.find(&open) else {
            continue;
        };
        let begin = begin + open.len();
        let Some(finish) = scheme[begin..].find(&close) else {
            continue;
        };
        let inner = &scheme[begin..begin + finish];
        // A colour is either written out or named by the system, and the
        // system colour carries the colour it means in `lastClr`.
        let color = attribute(inner, "val")
            .and_then(|value| parse_hex_color(&value))
            .or_else(|| attribute(inner, "lastClr").and_then(|value| parse_hex_color(&value)));
        if let Some(color) = color {
            colors.insert(name.to_string(), color);
        }
    }
    colors
}

/// The colour of a fill.
///
/// A solid fill is one colour; a gradient starts from one, so its first
/// stop is the colour the fill is made of.
fn fill_color(
    block: &str,
    theme: &HashMap<String, crate::domain::Color>,
) -> Option<crate::domain::Color> {
    let (start, end) = if let Some(s) = block.find("<a:solidFill>") {
        let e = block[s..]
            .find("</a:solidFill>")
            .map(|i| s + i)
            .unwrap_or(block.len());
        (s, e)
    } else {
        let s = block.find("<a:gradFill")?;
        let e = block[s..]
            .find("</a:gradFill>")
            .map(|i| s + i)
            .unwrap_or(block.len());
        (s, e)
    };
    let inner = &block[start..end];
    let value = attribute(inner, "val")?;
    // A colour written out, or a colour the theme names.
    parse_hex_color(&value).or_else(|| theme.get(scheme_name(&value)).copied())
}

/// The background a file declares.
fn parse_background(
    xml: &str,
    rels: &HashMap<String, String>,
    media: &HashMap<String, PathBuf>,
    theme: &HashMap<String, crate::domain::Color>,
) -> Option<crate::domain::Background> {
    let start = xml.find("<p:bg>")? + "<p:bg>".len();
    let end = xml[start..].find("</p:bg>")? + start;
    let block = &xml[start..end];

    // A picture behind the slide.
    if block.contains("<a:blipFill")
        && let Some(embed) = blip_embed(block)
        && let Some(path) = media_for(rels, media, &embed)
    {
        return Some(crate::domain::Background::Image(
            path.to_string_lossy().into_owned(),
        ));
    }
    // A colour behind the slide.
    fill_color(block, theme).map(crate::domain::Background::Solid)
}

/// The background of a slide.
///
/// A slide carries its own background and usually inherits the one from its
/// layout, which in turn inherits the one from the master. The reference's
/// importer offers the choice of keeping the background or leaving it out,
/// so the background is read here.
fn read_background<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    slide_name: &str,
    rels: &HashMap<String, String>,
    media: &HashMap<String, PathBuf>,
    theme: &HashMap<String, crate::domain::Color>,
) -> Option<crate::domain::Background> {
    let dir = slide_name
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    // The slide's own background.
    if let Some(xml) = read_entry(archive, slide_name)
        && let Some(background) = parse_background(&xml, rels, media, theme)
    {
        return Some(background);
    }
    // The layout's background.
    let layout = related_file(dir, rels, "slideLayout")?;
    let layout_rels = read_relationships(archive, &layout);
    let layout_dir = layout.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    if let Some(xml) = read_entry(archive, &layout)
        && let Some(background) = parse_background(&xml, &layout_rels, media, theme)
    {
        return Some(background);
    }
    // The master's background.
    let master = related_file(layout_dir, &layout_rels, "slideMaster")?;
    let master_rels = read_relationships(archive, &master);
    let xml = read_entry(archive, &master)?;
    parse_background(&xml, &master_rels, media, theme)
}

/// The number in a slide's file name.
///
/// The slides are numbered, but the file names are not padded, so the
/// slides have to be sorted by number rather than by name.
fn slide_number(name: &str) -> Option<u32> {
    let stem = name.strip_prefix("ppt/slides/slide")?;
    let stem = stem.strip_suffix(".xml")?;
    stem.parse().ok()
}

/// The size of the slides, in EMU.
fn slide_size<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>) -> Option<(u64, u64)> {
    let mut xml = String::new();
    {
        let mut entry = archive.by_name("ppt/presentation.xml").ok()?;
        entry.read_to_string(&mut xml).ok()?;
    }
    let reader = Reader::from_str(&xml);
    let mut reader = reader;
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == "sldSz" {
                    let mut cx = None;
                    let mut cy = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            "cx" => {
                                cx = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .and_then(|v| v.parse().ok())
                            }
                            "cy" => {
                                cy = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .and_then(|v| v.parse().ok())
                            }
                            _ => {}
                        }
                    }
                    if let (Some(cx), Some(cy)) = (cx, cy) {
                        return Some((cx, cy));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

/// The pictures in the package, written out and indexed by their name.
fn extract_media<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    media_dir: &Path,
) -> Result<HashMap<String, PathBuf>> {
    let mut media = HashMap::new();
    let mut names: Vec<String> = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().unwrap_or_default().to_string();
        if name.starts_with("ppt/media/") && !name.ends_with('/') {
            names.push(name);
        }
    }
    if names.is_empty() {
        return Ok(media);
    }
    std::fs::create_dir_all(media_dir)
        .with_context(|| format!("could not make the media folder {}", media_dir.display()))?;
    for name in names {
        let file_name = name.rsplit('/').next().unwrap_or(&name).to_string();
        let target = media_dir.join(&file_name);
        let mut bytes = Vec::new();
        {
            let mut entry = archive.by_name(&name)?;
            entry.read_to_end(&mut bytes)?;
        }
        std::fs::write(&target, &bytes)
            .with_context(|| format!("could not write {}", target.display()))?;
        media.insert(file_name, target);
    }
    Ok(media)
}

/// The style a run of text is drawn in.
///
/// The size is in hundredths of a point, and the weight and the slant are
/// flags: the run's own size wins over the shape's.
fn read_run_style(event: &quick_xml::events::BytesStart<'_>, mut run: TextStyle) -> TextStyle {
    for attr in event.attributes().flatten() {
        match attr.key.as_ref() {
            "sz" => {
                if let Some(points) = attr
                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
                {
                    // The size is in hundredths of a point.
                    run.font_size = TextStyle::clamp_font_size((points / 100.0) as f32);
                }
            }
            "b" => {
                if let Ok(value) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    run.bold = matches!(value.as_ref(), "1" | "true");
                }
            }
            "i" => {
                if let Ok(value) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    run.italic = matches!(value.as_ref(), "1" | "true");
                }
            }
            "u" => {
                if let Ok(value) = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
                    run.underline = !matches!(value.as_ref(), "none");
                }
            }
            _ => {}
        }
    }
    run
}

/// The colour a colour element names.
///
/// A colour is written out as six hex digits, or named by the theme, or
/// taken from the system — and the system colour carries the colour it
/// means in `lastClr`.
fn element_color(
    e: &quick_xml::events::BytesStart<'_>,
    theme: &HashMap<String, crate::domain::Color>,
) -> Option<crate::domain::Color> {
    let mut value = None;
    let mut last = None;
    for attr in e.attributes().flatten() {
        let text = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .ok()
            .map(|v| v.into_owned());
        match attr.key.as_ref() {
            "val" => value = text,
            "lastClr" => last = text,
            _ => {}
        }
    }
    value
        .as_ref()
        .and_then(|v| parse_hex_color(v))
        .or_else(|| {
            value
                .as_ref()
                .and_then(|v| theme.get(scheme_name(v)).copied())
        })
        .or_else(|| last.as_ref().and_then(|v| parse_hex_color(v)))
}

/// One shape read from a slide.
struct Shape {
    /// The shape's left edge, as a fraction of the slide's width.
    x: f64,
    /// The shape's top edge, as a fraction of the slide's height.
    y: f64,
    /// The shape's width, as a fraction of the slide's width.
    width: f64,
    /// The shape's height, as a fraction of the slide's height.
    height: f64,
    /// The text the shape carries.
    text: String,
    /// The style the text is drawn in.
    style: TextStyle,
    /// The picture the shape shows, if it is a picture.
    image: Option<PathBuf>,
    /// The video the shape plays, if it is a video.
    video: Option<PathBuf>,
}

/// Read one slide out of its XML.
fn parse_slide(
    xml: &str,
    slide_size: (u64, u64),
    media: &HashMap<String, PathBuf>,
    rels: &HashMap<String, String>,
    theme: &HashMap<String, crate::domain::Color>,
) -> Result<Slide> {
    let (slide_width, slide_height) = slide_size;
    let shapes = parse_shapes(xml, slide_width, slide_height, media, rels, theme)?;

    let mut slide = Slide::new_text(String::new());
    slide.layers.clear();
    for shape in shapes {
        let mut object = match (&shape.video, &shape.image) {
            // A clip plays, and the picture beside it is the frame the
            // deck shows before it starts.
            (Some(video), image) => Object::new_video(
                video.to_string_lossy().into_owned(),
                image
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
            ),
            (None, Some(path)) => {
                Object::new_image(path.to_string_lossy().into_owned(), ImageFit::Fill)
            }
            (None, None) => {
                let mut object = Object::new_text(shape.text.clone());
                if let ObjectContent::Text { style, .. } = &mut object.content {
                    *style = shape.style.clone();
                }
                object
            }
        };
        object.position_x = shape.x as f32;
        object.position_y = shape.y as f32;
        object.width = shape.width as f32;
        object.height = shape.height as f32;
        slide.layers.push(object);
    }
    Ok(slide)
}

/// Read the shapes out of a slide's XML.
fn parse_shapes(
    xml: &str,
    slide_width: u64,
    slide_height: u64,
    media: &HashMap<String, PathBuf>,
    rels: &HashMap<String, String>,
    theme: &HashMap<String, crate::domain::Color>,
) -> Result<Vec<Shape>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut shapes: Vec<Shape> = Vec::new();
    // The shape being read.
    let mut current: Option<Shape> = None;
    // The place of the shape on the slide, in EMU.
    let mut off = (0i64, 0i64);
    let mut ext = (0i64, 0i64);
    // The text of the shape, and the style of the first run that carries one.
    let mut text = String::new();
    let mut style: Option<TextStyle> = None;
    // The relationship the picture points at.
    let mut embed: Option<String> = None;
    // The relationship the video points at.
    let mut video_embed: Option<String> = None;
    // Inside a text run.
    let mut in_run = false;
    let mut run_text = String::new();
    let mut run_style: Option<TextStyle> = None;
    // Inside the properties of a run, and inside a fill within them: the
    // colour of the text is written in a fill, not in an attribute.
    let mut in_run_props = false;
    let mut in_fill = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.local_name().as_ref() {
                    "sp" | "pic" | "graphicFrame" => {
                        current = Some(Shape {
                            x: 0.0,
                            y: 0.0,
                            width: 0.0,
                            height: 0.0,
                            text: String::new(),
                            style: TextStyle::default(),
                            image: None,
                            video: None,
                        });
                        off = (0, 0);
                        ext = (0, 0);
                        text.clear();
                        style = None;
                        embed = None;
                        video_embed = None;
                    }
                    "off" => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                "x" => {
                                    off.0 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                "y" => {
                                    off.1 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                _ => {}
                            }
                        }
                    }
                    "ext" => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                "cx" => {
                                    ext.0 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                "cy" => {
                                    ext.1 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                _ => {}
                            }
                        }
                    }
                    "r" => {
                        in_run = true;
                        run_text.clear();
                        run_style = None;
                    }
                    "rPr" | "defRPr" => {
                        // The run's own size and weight, when it has them.
                        run_style = Some(read_run_style(&e, run_style.clone().unwrap_or_default()));
                        // The colour of the text is written in a fill
                        // inside these properties.
                        in_run_props = true;
                    }
                    "solidFill" => {
                        in_fill = true;
                    }
                    "blip" => {
                        for attr in e.attributes().flatten() {
                            // The relationship is namespaced, so the key ends
                            // with "embed".
                            if attr.key.as_ref().ends_with("embed") {
                                embed = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .map(|v| v.into_owned());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                match e.local_name().as_ref() {
                    "off" => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                "x" => {
                                    off.0 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                "y" => {
                                    off.1 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                _ => {}
                            }
                        }
                    }
                    "ext" => {
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                "cx" => {
                                    ext.0 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                "cy" => {
                                    ext.1 = attr
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .ok()
                                        .and_then(|v| v.parse().ok())
                                        .unwrap_or(0)
                                }
                                _ => {}
                            }
                        }
                    }
                    "blip" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref().ends_with("embed") {
                                embed = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .map(|v| v.into_owned());
                            }
                        }
                    }
                    "rPr" | "defRPr" => {
                        // The run's own size and weight, when it has them.
                        run_style = Some(read_run_style(&e, run_style.clone().unwrap_or_default()));
                    }
                    "srgbClr" | "schemeClr" | "sysClr" => {
                        // The colour of the text, written out or named by
                        // the theme.
                        if in_fill
                            && in_run_props
                            && let Some(color) = element_color(&e, theme)
                        {
                            let mut style = run_style.clone().unwrap_or_default();
                            style.color = color;
                            run_style = Some(style);
                        }
                    }
                    "videoFile" | "media" => {
                        // The clip a shape plays: the file it names is in
                        // the package, like a picture.
                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            if key.ends_with("link") || key.ends_with("embed") {
                                video_embed = attr
                                    .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .ok()
                                    .map(|v| v.into_owned());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if in_run {
                    let value = t.into_inner();
                    run_text.push_str(&value);
                }
            }
            Ok(Event::End(e)) => {
                match e.local_name().as_ref() {
                    "solidFill" => {
                        in_fill = false;
                    }
                    "rPr" | "defRPr" => {
                        in_run_props = false;
                    }
                    "r" => {
                        in_run = false;
                        if !run_text.is_empty() {
                            if style.is_none() {
                                style = run_style.clone();
                            }
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(&run_text);
                        }
                        run_text.clear();
                    }
                    "sp" | "pic" | "graphicFrame" => {
                        if let Some(mut shape) = current.take() {
                            let width = if slide_width > 0 {
                                ext.0 as f64 / slide_width as f64
                            } else {
                                0.0
                            };
                            let height = if slide_height > 0 {
                                ext.1 as f64 / slide_height as f64
                            } else {
                                0.0
                            };
                            shape.x = if slide_width > 0 {
                                off.0 as f64 / slide_width as f64
                            } else {
                                0.0
                            };
                            shape.y = if slide_height > 0 {
                                off.1 as f64 / slide_height as f64
                            } else {
                                0.0
                            };
                            shape.width = width;
                            shape.height = height;
                            if let Some(style) = style.clone() {
                                shape.style = style;
                            }
                            // The text the shape carries is the text that
                            // was read out of its runs.
                            shape.text = text.clone();
                            // The picture is found through the slide's own
                            // relationships: the embed name says which file
                            // in the package the picture is.
                            if let Some(embed) = embed.take() {
                                let path = media_for(rels, media, &embed).or_else(|| {
                                    // A deck that names the file itself is
                                    // read too, so a picture is never lost
                                    // to a relationship that is not there.
                                    media
                                        .iter()
                                        .find(|(name, _)| name.contains(&embed))
                                        .map(|(_, path)| path.clone())
                                });
                                shape.image = path;
                            }
                            // A clip is named by its own relationship, and
                            // the picture beside it is the frame the deck
                            // shows while the clip is not playing.
                            if let Some(embed) = video_embed.take() {
                                shape.video = media_for(rels, media, &embed);
                            }
                            if shape.video.is_some()
                                || shape.image.is_some()
                                || !shape.text.trim().is_empty()
                            {
                                shapes.push(shape);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => bail!("the slide's XML could not be read: {e}"),
            _ => {}
        }
        buf.clear();
    }
    Ok(shapes)
}

/// The shape a slide is built from when it holds nothing else.
pub fn blank_slide() -> Slide {
    Slide::new_text(String::new())
}

/// The shape kinds a slide can hold, for the caller that builds the menu.
pub fn shape_kinds() -> [ShapeType; 1] {
    [ShapeType::Rectangle]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slide's shapes are read with their places and their text.
    #[test]
    fn a_slide_is_read_with_its_shapes() {
        let xml = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:sp>
      <p:spPr><a:xfrm><a:off x="914400" y="457200"/>
        <a:ext cx="4572000" cy="1143000"/></a:xfrm></p:spPr>
      <p:txBody>
        <a:p><a:r><a:rPr lang="en-US" sz="4400" b="1"/><a:t>Welcome</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
        let shapes = parse_shapes(
            xml,
            12_192_000,
            6_858_000,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("the slide is read");
        assert_eq!(shapes.len(), 1);
        let shape = &shapes[0];
        assert_eq!(shape.text, "Welcome");
        assert!(shape.style.bold);
        // The size is 4400 hundredths of a point, so 44 points.
        assert!((shape.style.font_size - 44.0).abs() < 0.01);
        // The place is a fraction of the slide.
        assert!((shape.x - 0.075).abs() < 0.001);
        assert!((shape.width - 0.375).abs() < 0.001);
    }

    /// A shape with no text and no picture is not a shape: it is the
    /// background, and the background is the slide's own.
    #[test]
    fn an_empty_shape_is_left_out() {
        let xml = r#"<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:sp><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr></p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
        let shapes = parse_shapes(
            xml,
            12_192_000,
            6_858_000,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("the slide is read");
        assert!(shapes.is_empty());
    }

    /// A slide carries the transition the file gave it, and the speed of a
    /// transition is a length of time.
    #[test]
    fn a_transition_is_read_with_its_speed() {
        let xml = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:transition spd="slow"><p:wipe dir="l"/></p:transition>
</p:sld>"#;
        assert_eq!(
            read_transition(xml),
            crate::domain::Transition::Wipe {
                duration_ms: 1200,
                angle_deg: 180,
            }
        );

        let xml = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:transition spd="fast"><p:fade/></p:transition>
</p:sld>"#;
        assert_eq!(
            read_transition(xml),
            crate::domain::Transition::Fade { duration_ms: 350 }
        );
    }

    /// A slide with no transition cuts: that is what a slide with no
    /// transition means.
    #[test]
    fn a_slide_with_no_transition_cuts() {
        let xml = r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree/></p:cSld>
</p:sld>"#;
        assert_eq!(read_transition(xml), crate::domain::Transition::Cut);
    }

    /// A deck written to a file, so the importer can be driven the way the
    /// operator drives it.
    fn a_deck(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a temp dir");
        let path = dir.path().join("deck.pptx");
        let file = std::fs::File::create(&path).expect("the file");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, body) in files {
            zip.start_file(*name, options).expect("the entry");
            std::io::Write::write_all(&mut zip, body.as_bytes()).expect("the body");
        }
        zip.finish().expect("the archive");
        (dir, path)
    }

    /// The slide of the little deck the tests share.
    const SLIDE: &str = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:sp>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="4572000" cy="1143000"/></a:xfrm></p:spPr>
      <p:txBody><a:p><a:r><a:t>Welcome</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
  <p:transition spd="slow"><p:wipe dir="r"/></p:transition>
</p:sld>"#;

    /// The notes page of the little deck the tests share.
    const NOTES: &str = r#"<?xml version="1.0"?>
<p:notes xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
         xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:sp>
      <p:nvSpPr><p:nvPr><p:ph type="body"/></p:nvPr></p:nvSpPr>
      <p:txBody><a:p><a:r><a:t>Sing slowly.</a:t></a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:notes>"#;

    /// The deck the tests share: one slide, a transition and a notes page.
    fn the_deck() -> (tempfile::TempDir, PathBuf) {
        a_deck(&[
            ("ppt/slides/slide1.xml", SLIDE),
            (
                "ppt/slides/_rels/slide1.xml.rels",
                r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide"
                Target="../notesSlides/notesSlide1.xml"/>
</Relationships>"#,
            ),
            ("ppt/notesSlides/notesSlide1.xml", NOTES),
        ])
    }

    /// A deck imported as a presentation keeps the transition and the notes
    /// the file carried.
    #[test]
    fn a_deck_imported_as_a_presentation_keeps_its_show_details() {
        let (_dir, path) = the_deck();
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), true).expect("the deck is read");
        assert_eq!(deck.slides.len(), 1);
        assert_eq!(
            deck.slides[0].transition,
            crate::domain::Transition::Wipe {
                duration_ms: 1200,
                angle_deg: 0,
            }
        );
        assert_eq!(deck.slides[0].notes.as_deref(), Some("Sing slowly."));
        // The slide's own text is still there.
        assert_eq!(deck.slides[0].layers.len(), 1);
    }

    /// A deck imported as editable objects comes in without the show's
    /// furniture: the operator is going to change it.
    #[test]
    fn a_deck_imported_as_objects_leaves_the_show_details_out() {
        let (_dir, path) = the_deck();
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(deck.slides[0].transition, crate::domain::Transition::Cut);
        assert!(deck.slides[0].notes.is_none());
        assert_eq!(deck.slides[0].layers.len(), 1);
    }

    /// A slide that holds a picture: the picture is named by a
    /// relationship, and the relationship says which file it is.
    const SLIDE_WITH_PICTURE: &str = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:pic>
      <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="4572000" cy="1143000"/></a:xfrm></p:spPr>
    </p:pic>
  </p:spTree></p:cSld>
</p:sld>"#;

    /// The relationships of the slide that holds a picture.
    const RELS_WITH_PICTURE: &str = r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout"
                Target="../slideLayouts/slideLayout1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                Target="../media/image1.png"/>
</Relationships>"#;

    /// A picture in a deck comes in as a picture.
    ///
    /// The picture is named by a relationship — `rId2` — and the
    /// relationship says which file in the package it is. A deck whose
    /// pictures are dropped is a deck whose slides are empty.
    #[test]
    fn a_picture_in_the_deck_comes_in_as_a_picture() {
        let (_dir, path) = a_deck(&[
            ("ppt/slides/slide1.xml", SLIDE_WITH_PICTURE),
            ("ppt/slides/_rels/slide1.xml.rels", RELS_WITH_PICTURE),
            ("ppt/media/image1.png", "the picture's bytes"),
        ]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(deck.slides[0].layers.len(), 1);
        let layer = &deck.slides[0].layers[0];
        assert!(
            matches!(&layer.content, ObjectContent::Image { path, .. } if path.ends_with("image1.png")),
            "the picture came in as a picture, not as {layer:?}"
        );
    }

    /// A slide keeps the background the deck gave it.
    #[test]
    fn a_slide_keeps_the_background_the_deck_gave_it() {
        let slide = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="1F4E79"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree/>
  </p:cSld>
</p:sld>"#;
        let (_dir, path) = a_deck(&[("ppt/slides/slide1.xml", slide)]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(
            deck.slides[0].background,
            crate::domain::Background::Solid(crate::domain::Color {
                r: 0x1F,
                g: 0x4E,
                b: 0x79,
                a: 255
            })
        );
    }

    /// A slide with no background of its own inherits the one from its
    /// layout, which is where a deck's background really lives.
    #[test]
    fn a_slide_inherits_the_background_of_its_layout() {
        let layout = r#"<?xml version="1.0"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree/>
  </p:cSld>
</p:sldLayout>"#;
        let (_dir, path) = a_deck(&[
            ("ppt/slides/slide1.xml", SLIDE),
            ("ppt/slides/_rels/slide1.xml.rels", RELS_WITH_PICTURE),
            ("ppt/slideLayouts/slideLayout1.xml", layout),
        ]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(
            deck.slides[0].background,
            crate::domain::Background::Solid(crate::domain::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            })
        );
    }

    /// A colour the theme names is the colour the theme gives it.
    #[test]
    fn a_colour_the_theme_names_is_the_colour_the_theme_gives_it() {
        let theme = r#"<?xml version="1.0"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Test">
  <a:themeElements>
    <a:clrScheme name="Test">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;
        let slide = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg><p:bgPr><a:solidFill><a:schemeClr val="dk2"/></a:solidFill></p:bgPr></p:bg>
    <p:spTree/>
  </p:cSld>
</p:sld>"#;
        let (_dir, path) = a_deck(&[
            ("ppt/slides/slide1.xml", slide),
            ("ppt/theme/theme1.xml", theme),
        ]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(
            deck.slides[0].background,
            crate::domain::Background::Solid(crate::domain::Color {
                r: 0x44,
                g: 0x54,
                b: 0x6A,
                a: 255
            })
        );
    }

    /// The colour of the text is the colour the deck wrote in it.
    #[test]
    fn the_text_keeps_the_colour_the_deck_wrote_in_it() {
        let slide = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:sp>
      <p:txBody><a:p><a:r>
        <a:rPr lang="en-US" sz="4400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:rPr>
        <a:t>Welcome</a:t>
      </a:r></a:p></p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;
        let (_dir, path) = a_deck(&[("ppt/slides/slide1.xml", slide)]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(deck.slides[0].layers.len(), 1);
        let ObjectContent::Text { style, .. } = &deck.slides[0].layers[0].content else {
            panic!("the shape carries text");
        };
        assert_eq!(
            style.color,
            crate::domain::Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }
        );
    }

    /// A clip in a deck comes in as a clip.
    ///
    /// The clip is named by its own relationship, and the picture beside it
    /// is the frame the deck shows before the clip starts.
    #[test]
    fn a_clip_in_the_deck_comes_in_as_a_clip() {
        let slide = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree>
    <p:pic>
      <p:nvPicPr><p:nvPr><a:videoFile r:link="rId3"/></p:nvPr></p:nvPicPr>
      <p:blipFill><a:blip r:embed="rId2"/></p:blipFill>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="4572000" cy="1143000"/></a:xfrm></p:spPr>
    </p:pic>
  </p:spTree></p:cSld>
</p:sld>"#;
        let rels = r#"<?xml version="1.0"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                Target="../media/image1.png"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/video"
                Target="../media/media1.mp4"/>
</Relationships>"#;
        let (_dir, path) = a_deck(&[
            ("ppt/slides/slide1.xml", slide),
            ("ppt/slides/_rels/slide1.xml.rels", rels),
            ("ppt/media/image1.png", "the poster frame"),
            ("ppt/media/media1.mp4", "the clip's bytes"),
        ]);
        let media = tempfile::tempdir().expect("a media dir");
        let deck = import_with(&path, media.path(), false).expect("the deck is read");
        assert_eq!(deck.slides[0].layers.len(), 1);
        let ObjectContent::Video { path, thumbnail } = &deck.slides[0].layers[0].content else {
            panic!(
                "the shape carries a clip, not {:?}",
                deck.slides[0].layers[0].content
            );
        };
        assert!(path.ends_with("media1.mp4"), "the clip came in as a clip");
        // The frame the deck shows before the clip starts comes with it.
        assert!(
            thumbnail
                .as_ref()
                .is_some_and(|path| path.ends_with("image1.png")),
            "the poster frame came with the clip"
        );
    }
}
