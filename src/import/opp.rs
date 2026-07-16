use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::domain::{Background, ObjectContent, Presentation};

#[derive(Debug, Serialize, Deserialize)]
pub struct OppMeta {
    pub format_version: u32,
    pub created_at: String,
    pub app_version: String,
}

impl OppMeta {
    fn current() -> Self {
        Self {
            format_version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub fn export(presentation: &Presentation, dest_path: &Path) -> Result<usize> {
    let file = std::fs::File::create(dest_path)
        .with_context(|| format!("Cannot create {}", dest_path.display()))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let meta = serde_json::to_string_pretty(&OppMeta::current())?;
    zip.start_file("meta.json", options)?;
    zip.write_all(meta.as_bytes())?;

    let pres_json = serde_json::to_string_pretty(presentation)?;
    zip.start_file("presentation.json", options)?;
    zip.write_all(pres_json.as_bytes())?;

    let media_paths = collect_media_paths(presentation);
    let mut embedded = 0usize;

    for path in &media_paths {
        let p = Path::new(path);
        if !p.exists() {
            continue;
        }
        let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let entry_name = format!("media/{filename}");
        match std::fs::read(p) {
            Ok(bytes) => {
                zip.start_file(&entry_name, options)?;
                zip.write_all(&bytes)?;
                embedded += 1;
            }
            Err(e) => {
                eprintln!("[opp export] skipping {path}: {e}");
            }
        }
    }

    zip.finish()?;
    Ok(embedded)
}

pub fn import(src_path: &Path, media_dest_dir: &Path) -> Result<Presentation> {
    let file = std::fs::File::open(src_path)
        .with_context(|| format!("Cannot open {}", src_path.display()))?;

    let mut zip = zip::ZipArchive::new(file)?;

    let pres_json = {
        let mut entry = zip
            .by_name("presentation.json")
            .context("presentation.json not found in .opp archive")?;
        let mut s = String::new();
        entry.read_to_string(&mut s)?;
        s
    };
    let mut presentation: Presentation = serde_json::from_str(&pres_json)?;

    std::fs::create_dir_all(media_dest_dir)?;
    let mut extracted_map: std::collections::HashMap<String, PathBuf> =
        std::collections::HashMap::new();

    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    for name in &names {
        if !name.starts_with("media/") {
            continue;
        }
        let filename = name.trim_start_matches("media/");
        if filename.is_empty() {
            continue;
        }
        let dest = media_dest_dir.join(filename);
        let mut entry = zip.by_name(name)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        std::fs::write(&dest, &bytes)?;
        extracted_map.insert(filename.to_string(), dest);
    }

    rewrite_media_paths(&mut presentation, &extracted_map);

    Ok(presentation)
}

fn collect_media_paths(presentation: &Presentation) -> Vec<String> {
    let mut paths = Vec::new();
    for slide in &presentation.slides {
        match &slide.background {
            Background::Image(p) | Background::Video(p) => paths.push(p.clone()),
            Background::Solid(_) => {}
        }
        for layer in &slide.layers {
            match &layer.content {
                ObjectContent::Image { path, .. } => paths.push(path.clone()),
                ObjectContent::Video { path, .. } => paths.push(path.clone()),
                _ => {}
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn rewrite_media_paths(
    presentation: &mut Presentation,
    map: &std::collections::HashMap<String, PathBuf>,
) {
    for slide in &mut presentation.slides {
        match &mut slide.background {
            Background::Image(p) | Background::Video(p) => {
                let filename = std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(new_path) = map.get(&filename) {
                    *p = new_path.to_string_lossy().into_owned();
                }
            }
            Background::Solid(_) => {}
        }
        for layer in &mut slide.layers {
            match &mut layer.content {
                ObjectContent::Image { path, .. } | ObjectContent::Video { path, .. } => {
                    let filename = std::path::Path::new(&*path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    if let Some(new_path) = map.get(&filename) {
                        *path = new_path.to_string_lossy().into_owned();
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Background, Color, Presentation, Slide, SlideContent, TextStyle, Transition,
    };
    use std::io::Read;
    use tempfile::tempdir;

    fn minimal_presentation() -> Presentation {
        let mut p = Presentation::new("Test Presentation".to_string());
        p.slides.push(Slide {
            id: "slide-1".to_string(),
            content: SlideContent::Text {
                text: "Hello World".to_string(),
                style: TextStyle::default(),
            },
            background: Background::Solid(Color::black()),
            transition: Transition::Cut,
            group: None,
            notes: Some("Test note".to_string()),
            layers: Vec::new(),
            cues: Vec::new(),
        });
        p
    }

    #[test]
    fn export_creates_valid_zip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.opp");
        let pres = minimal_presentation();
        let _ = export(&pres, &path).unwrap();
        assert!(path.exists());
        let mut f = std::fs::File::open(&path).unwrap();
        let mut magic = [0u8; 4];
        f.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"PK\x03\x04");
    }

    #[test]
    fn roundtrip_preserves_presentation_name() {
        let dir = tempdir().unwrap();
        let opp_path = dir.path().join("test.opp");
        let media_dir = dir.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();

        let original = minimal_presentation();
        export(&original, &opp_path).unwrap();
        let imported = import(&opp_path, &media_dir).unwrap();

        assert_eq!(imported.name, original.name);
    }

    #[test]
    fn roundtrip_preserves_slide_count() {
        let dir = tempdir().unwrap();
        let opp_path = dir.path().join("test.opp");
        let media_dir = dir.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();

        let original = minimal_presentation();
        export(&original, &opp_path).unwrap();
        let imported = import(&opp_path, &media_dir).unwrap();

        assert_eq!(imported.slides.len(), original.slides.len());
    }

    #[test]
    fn roundtrip_preserves_slide_text() {
        let dir = tempdir().unwrap();
        let opp_path = dir.path().join("test.opp");
        let media_dir = dir.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();

        let original = minimal_presentation();
        export(&original, &opp_path).unwrap();
        let imported = import(&opp_path, &media_dir).unwrap();

        match &imported.slides[0].content {
            SlideContent::Text { text, .. } => assert_eq!(text, "Hello World"),
            _ => panic!("expected text slide"),
        }
    }
}
