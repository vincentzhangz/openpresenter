use openpresenter::domain::{
    Background, Color, Presentation, Slide, SlideContent, Song, TextStyle, Transition, Verse,
};
use openpresenter::import::{openlyrics, opp};
use tempfile::tempdir;

#[test]
fn openlyrics_file_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("song.xml");

    let mut song = Song::new("To God Be the Glory".to_string());
    song.artist = Some("Fanny Crosby".to_string());
    song.verses.push(Verse::new(
        "v1".to_string(),
        "To God be the glory\nGreat things He hath done".to_string(),
        0,
    ));
    song.verses.push(Verse::new(
        "c1".to_string(),
        "Praise the Lord, praise the Lord".to_string(),
        1,
    ));

    openlyrics::export_song(&song, &path).expect("export");
    assert!(path.exists());

    let imported = openlyrics::import_song(&path).expect("import");

    assert_eq!(imported.title, song.title);
    assert_eq!(imported.artist, song.artist);
    assert_eq!(imported.verses.len(), song.verses.len());
    assert_eq!(imported.verses[0].label, "v1");
    assert!(imported.verses[0].content.contains("To God be the glory"));
    assert_eq!(imported.verses[1].label, "c1");
}

#[test]
fn openlyrics_xml_string_roundtrip() {
    let mut song = Song::new("Blessed Assurance".to_string());
    song.bpm = Some(100);
    song.key_signature = Some("D".to_string());
    song.verses.push(Verse::new(
        "v1".to_string(),
        "Blessed assurance, Jesus is mine".to_string(),
        0,
    ));

    let xml = openlyrics::song_to_xml(&song);
    let restored = openlyrics::parse_openlyrics_xml(&xml).expect("parse");

    assert_eq!(restored.title, "Blessed Assurance");
    assert_eq!(restored.bpm, Some(100));
    assert_eq!(restored.verses.len(), 1);
    assert_eq!(restored.verses[0].label, "v1");
}

fn sample_presentation() -> Presentation {
    let mut p = Presentation::new("Integration Test Show".to_string());
    for i in 0..3 {
        p.slides.push(Slide {
            id: format!("slide-{i}"),
            content: SlideContent::Text {
                text: format!("Slide {i} content"),
                style: TextStyle::default(),
            },
            background: Background::Solid(Color::black()),
            transition: Transition::Cut,
            group: if i == 0 {
                Some("Intro".to_string())
            } else {
                None
            },
            notes: None,
            layers: Vec::new(),
            cues: Vec::new(),
        });
    }
    p
}

#[test]
fn opp_roundtrip_preserves_all_slides() {
    let dir = tempdir().unwrap();
    let opp_path = dir.path().join("show.opp");
    let media_dir = dir.path().join("media");
    std::fs::create_dir_all(&media_dir).unwrap();

    let original = sample_presentation();
    opp::export(&original, &opp_path).expect("export");

    let imported = opp::import(&opp_path, &media_dir).expect("import");

    assert_eq!(imported.name, original.name);
    assert_eq!(imported.slides.len(), 3);
    for (i, slide) in imported.slides.iter().enumerate() {
        match &slide.content {
            SlideContent::Text { text, .. } => {
                assert_eq!(text, &format!("Slide {i} content"));
            }
            _ => panic!("expected text slide at index {i}"),
        }
    }
}

#[test]
fn opp_roundtrip_preserves_group_labels() {
    let dir = tempdir().unwrap();
    let opp_path = dir.path().join("show.opp");
    let media_dir = dir.path().join("media");
    std::fs::create_dir_all(&media_dir).unwrap();

    let original = sample_presentation();
    opp::export(&original, &opp_path).expect("export");
    let imported = opp::import(&opp_path, &media_dir).expect("import");

    assert_eq!(imported.slides[0].group, Some("Intro".to_string()));
    assert!(imported.slides[1].group.is_none());
}
