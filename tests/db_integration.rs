use openpresenter::db::{Database, PresentationRepository, SongRepository};
use openpresenter::slides::{
    Background, Color, Slide, SlideContent, Song, TextStyle, Transition, Verse,
};
use std::sync::Arc;

fn in_memory_db() -> Arc<Database> {
    Arc::new(Database::in_memory().expect("in-memory DB"))
}

#[test]
fn create_and_list_presentation() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    repo.create_presentation("Test Show").expect("create");
    let list = repo.list_presentations().expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "Test Show");
}

#[test]
fn get_presentation_by_id() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    let pres = repo.create_presentation("My Slides").expect("create");
    let loaded = repo.get_presentation(&pres.id).expect("get");
    assert_eq!(loaded.id, pres.id);
    assert_eq!(loaded.name, "My Slides");
}

#[test]
fn add_slides_to_presentation() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    let pres = repo.create_presentation("With Slides").expect("create");

    let slide1 = Slide {
        id: "slide-1".to_string(),
        content: SlideContent::Text {
            text: "Verse 1".to_string(),
            style: TextStyle::default(),
        },
        background: Background::Solid(Color::black()),
        transition: Transition::Cut,
        group_label: None,
        notes: None,
        layers: Vec::new(),
    };
    let slide2 = Slide {
        id: "slide-2".to_string(),
        content: SlideContent::Text {
            text: "Verse 2".to_string(),
            style: TextStyle::default(),
        },
        background: Background::Solid(Color::black()),
        transition: Transition::Cut,
        group_label: Some("Group A".to_string()),
        notes: None,
        layers: Vec::new(),
    };

    repo.add_slide(&pres.id, &slide1, 0).expect("add slide1");
    repo.add_slide(&pres.id, &slide2, 1).expect("add slide2");

    let loaded = repo.get_presentation(&pres.id).expect("get");
    assert_eq!(loaded.slides.len(), 2);
    match &loaded.slides[0].content {
        SlideContent::Text { text, .. } => assert_eq!(text, "Verse 1"),
        _ => panic!("unexpected content type"),
    }
    assert_eq!(loaded.slides[1].group_label, Some("Group A".to_string()));
}

#[test]
fn delete_presentation() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    let pres = repo.create_presentation("To Delete").expect("create");
    repo.delete_presentation(&pres.id).expect("delete");
    assert!(repo.list_presentations().expect("list").is_empty());
}

#[test]
fn update_presentation_name() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    let pres = repo.create_presentation("Old Name").expect("create");
    repo.update_presentation(&pres.id, "New Name")
        .expect("update");
    let loaded = repo.get_presentation(&pres.id).expect("get");
    assert_eq!(loaded.name, "New Name");
}

#[test]
fn replace_slides_replaces_all_existing_slides() {
    let db = in_memory_db();
    let repo = PresentationRepository::new(Arc::clone(&db));
    let pres = repo.create_presentation("Replace Test").expect("create");

    let orig = Slide {
        id: "orig-1".to_string(),
        content: SlideContent::Text {
            text: "Original".to_string(),
            style: TextStyle::default(),
        },
        background: Background::Solid(Color::black()),
        transition: Transition::Cut,
        group_label: None,
        notes: None,
        layers: Vec::new(),
    };
    repo.add_slide(&pres.id, &orig, 0).expect("add");

    let new_slides = vec![
        Slide {
            id: "new-1".to_string(),
            content: SlideContent::Text {
                text: "New A".to_string(),
                style: TextStyle::default(),
            },
            background: Background::Solid(Color::black()),
            transition: Transition::Cut,
            group_label: None,
            notes: None,
            layers: Vec::new(),
        },
        Slide {
            id: "new-2".to_string(),
            content: SlideContent::Text {
                text: "New B".to_string(),
                style: TextStyle::default(),
            },
            background: Background::Solid(Color::black()),
            transition: Transition::Cut,
            group_label: None,
            notes: None,
            layers: Vec::new(),
        },
    ];
    repo.replace_presentation_slides(&pres.id, &new_slides)
        .expect("replace");

    let loaded = repo.get_presentation(&pres.id).expect("get");
    assert_eq!(loaded.slides.len(), 2);
    match &loaded.slides[0].content {
        SlideContent::Text { text, .. } => assert_eq!(text, "New A"),
        _ => panic!("unexpected content"),
    }
}

#[test]
fn save_and_list_song() {
    let db = in_memory_db();
    let repo = SongRepository::new(Arc::clone(&db));
    let mut song = Song::new("Amazing Grace".to_string());
    song.artist = Some("John Newton".to_string());
    repo.save_song(&song).expect("save");
    let list = repo.list_songs().expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Amazing Grace");
    assert_eq!(list[0].artist, Some("John Newton".to_string()));
}

#[test]
fn get_song_with_verses() {
    let db = in_memory_db();
    let repo = SongRepository::new(Arc::clone(&db));
    let mut song = Song::new("Rock of Ages".to_string());
    song.verses.push(Verse::new(
        "v1".to_string(),
        "Rock of ages cleft for me".to_string(),
        0,
    ));
    song.verses.push(Verse::new(
        "v2".to_string(),
        "Not the labour of my hands".to_string(),
        1,
    ));
    let id = song.id.clone();
    repo.save_song(&song).expect("save");
    repo.replace_verses(&id, &song.verses)
        .expect("replace_verses");
    let loaded = repo.get_song(&id).expect("get");
    assert_eq!(loaded.verses.len(), 2);
    assert_eq!(loaded.verses[0].label, "v1");
    assert!(loaded.verses[0].content.contains("Rock of ages"));
}

#[test]
fn delete_song() {
    let db = in_memory_db();
    let repo = SongRepository::new(Arc::clone(&db));
    let song = Song::new("Delete Me".to_string());
    let id = song.id.clone();
    repo.save_song(&song).expect("save");
    repo.delete_song(&id).expect("delete");
    assert!(repo.list_songs().expect("list").is_empty());
}
