use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::Path;

use crate::slides::{Song, Verse};

pub fn export_song(song: &Song, dest_path: &Path) -> Result<()> {
    let xml = song_to_xml(song);
    let mut file = std::fs::File::create(dest_path)
        .with_context(|| format!("Cannot create {}", dest_path.display()))?;
    file.write_all(xml.as_bytes())?;
    Ok(())
}

pub fn song_to_xml(song: &Song) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<song xmlns=\"http://openlyrics.info/namespace/2009/song\"\n");
    xml.push_str("      version=\"0.9\" createdIn=\"OpenPresenter\">\n");
    xml.push_str("  <properties>\n");

    xml.push_str("    <titles><title>");
    xml.push_str(&escape_xml(&song.title));
    xml.push_str("</title></titles>\n");

    if let Some(artist) = &song.artist {
        xml.push_str("    <authors><author>");
        xml.push_str(&escape_xml(artist));
        xml.push_str("</author></authors>\n");
    }

    if let Some(copyright) = &song.copyright {
        xml.push_str("    <copyright>");
        xml.push_str(&escape_xml(copyright));
        xml.push_str("</copyright>\n");
    }

    if let Some(ccli) = &song.ccli_number {
        xml.push_str("    <ccliNo>");
        xml.push_str(&escape_xml(ccli));
        xml.push_str("</ccliNo>\n");
    }

    if let Some(key) = &song.key_signature {
        xml.push_str("    <key>");
        xml.push_str(&escape_xml(key));
        xml.push_str("</key>\n");
    }

    if let Some(bpm) = song.bpm {
        xml.push_str(&format!("    <tempo unit=\"bpm\">{bpm}</tempo>\n"));
    }

    xml.push_str("  </properties>\n");
    xml.push_str("  <lyrics>\n");

    for verse in &song.verses {
        xml.push_str(&format!(
            "    <verse name=\"{}\">\n",
            escape_xml(&verse.label)
        ));
        for line in verse.content.split('\n') {
            xml.push_str("      <lines>");
            xml.push_str(&escape_xml(line));
            xml.push_str("</lines>\n");
        }
        xml.push_str("    </verse>\n");
    }

    xml.push_str("  </lyrics>\n");
    xml.push_str("</song>\n");
    xml
}

pub fn import_song(src_path: &Path) -> Result<Song> {
    let mut file = std::fs::File::open(src_path)
        .with_context(|| format!("Cannot open {}", src_path.display()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    parse_openlyrics_xml(&content)
}

pub fn parse_openlyrics_xml(xml: &str) -> Result<Song> {
    let mut song = Song::new(String::new());

    song.title = extract_first_tag(xml, "title").unwrap_or_default();
    song.artist = extract_first_tag(xml, "author");
    song.copyright = extract_first_tag(xml, "copyright");
    song.ccli_number = extract_first_tag(xml, "ccliNo");
    song.key_signature = extract_first_tag(xml, "key");
    song.bpm = extract_first_tag(xml, "tempo").and_then(|s| s.parse::<u32>().ok());

    if song.title.is_empty() {
        song.title = "Imported Song".to_string();
    }

    for (order_index, verse_block) in split_tag_blocks(xml, "verse").into_iter().enumerate() {
        let label =
            attr_value(&verse_block, "name").unwrap_or_else(|| format!("v{}", order_index + 1));
        let lines: Vec<String> = split_tag_blocks(&verse_block, "lines")
            .into_iter()
            .map(|b| unescape_xml(&strip_outer_tag(&b, "lines")))
            .collect();
        let content = lines.join("\n");
        song.verses.push(Verse::new(label, content, order_index));
    }

    let now = chrono::Utc::now();
    song.created_at = now;
    song.updated_at = now;
    song.id = uuid::Uuid::new_v4().to_string();

    Ok(song)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn extract_first_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let open_attr = format!("<{tag} ");
    let close = format!("</{tag}>");

    let _start = xml.find(&open).or_else(|| {
        xml.find(&open_attr)
            .and_then(|pos| xml[pos..].find('>').map(|end| pos + end + 1))
    });

    let inner_start = if let Some(pos) = xml.find(&open) {
        pos + open.len()
    } else if let Some(pos) = xml.find(&open_attr) {
        pos + xml[pos..].find('>')? + 1
    } else {
        return None;
    };

    let inner_end = xml[inner_start..].find(&close)?;
    let text = unescape_xml(xml[inner_start..inner_start + inner_end].trim());
    if text.is_empty() { None } else { Some(text) }
}

fn split_tag_blocks(xml: &str, tag: &str) -> Vec<String> {
    let open1 = format!("<{tag}>");
    let open2 = format!("<{tag} ");
    let close = format!("</{tag}>");
    let mut blocks = Vec::new();
    let mut remaining = xml;
    loop {
        let pos1 = remaining.find(&open1);
        let pos2 = remaining.find(&open2);
        let start = match (pos1, pos2) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let start = match start {
            Some(s) => s,
            None => break,
        };
        let after_start = match remaining[start..].find('>') {
            Some(p) => start + p + 1,
            None => break,
        };
        let close_pos = match remaining[after_start..].find(&close) {
            Some(p) => after_start + p,
            None => break,
        };
        blocks.push(remaining[start..close_pos + close.len()].to_string());
        remaining = &remaining[close_pos + close.len()..];
    }
    blocks
}

fn strip_outer_tag(block: &str, tag: &str) -> String {
    let close = format!("</{tag}>");
    let start = block.find('>').map(|p| p + 1).unwrap_or(0);
    let end = block.rfind(&close).unwrap_or(block.len());
    block[start..end].to_string()
}

fn attr_value(block: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let start = block.find(&needle)? + needle.len();
    let end = block[start..].find('"')?;
    Some(block[start..start + end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_song() {
        let mut song = Song::new("Amazing Grace".to_string());
        song.artist = Some("John Newton".to_string());
        song.copyright = Some("Public Domain".to_string());
        song.ccli_number = Some("12345".to_string());
        song.key_signature = Some("G".to_string());
        song.bpm = Some(80);
        song.verses.push(Verse::new(
            "v1".to_string(),
            "Amazing grace how sweet the sound\nThat saved a wretch like me".to_string(),
            0,
        ));
        song.verses.push(Verse::new(
            "v2".to_string(),
            "Twas grace that taught my heart to fear\nAnd grace my fears relieved".to_string(),
            1,
        ));

        let xml = song_to_xml(&song);
        let imported = parse_openlyrics_xml(&xml).unwrap();

        assert_eq!(imported.title, song.title);
        assert_eq!(imported.artist, song.artist);
        assert_eq!(imported.ccli_number, song.ccli_number);
        assert_eq!(imported.bpm, song.bpm);
        assert_eq!(imported.verses.len(), song.verses.len());
        assert_eq!(imported.verses[0].label, "v1");
        assert!(imported.verses[0].content.contains("Amazing grace"));
    }

    #[test]
    fn roundtrip_minimal_song() {
        let song = Song::new("Minimal".to_string());
        let xml = song_to_xml(&song);
        let imported = parse_openlyrics_xml(&xml).unwrap();
        assert_eq!(imported.title, "Minimal");
        assert!(imported.verses.is_empty());
    }

    #[test]
    fn roundtrip_song_with_special_characters() {
        let mut song = Song::new("Hallelujah & Amen".to_string());
        song.verses.push(Verse::new(
            "c1".to_string(),
            "Praise <God> & His \"mercy\"".to_string(),
            0,
        ));
        let xml = song_to_xml(&song);
        let imported = parse_openlyrics_xml(&xml).unwrap();
        assert_eq!(imported.title, "Hallelujah & Amen");
        assert!(!imported.verses[0].content.is_empty());
    }

    #[test]
    fn roundtrip_preserves_verse_order() {
        let mut song = Song::new("Order Test".to_string());
        for i in 0..5 {
            song.verses.push(Verse::new(
                format!("v{}", i + 1),
                format!("Verse {i} content"),
                i,
            ));
        }
        let xml = song_to_xml(&song);
        let imported = parse_openlyrics_xml(&xml).unwrap();
        assert_eq!(imported.verses.len(), 5);
        for (i, v) in imported.verses.iter().enumerate() {
            assert_eq!(v.label, format!("v{}", i + 1));
        }
    }

    #[test]
    fn roundtrip_optional_fields_absent_when_none() {
        let song = Song::new("No Meta".to_string());
        let xml = song_to_xml(&song);
        let imported = parse_openlyrics_xml(&xml).unwrap();
        assert!(imported.artist.is_none());
        assert!(imported.bpm.is_none());
        assert!(imported.ccli_number.is_none());
    }
}
