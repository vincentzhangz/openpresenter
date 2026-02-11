pub const CREATE_SONGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS songs (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT,
    copyright TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)
"#;

pub const CREATE_VERSES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS verses (
    id TEXT PRIMARY KEY,
    song_id TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    content TEXT NOT NULL,
    order_index INTEGER NOT NULL
)
"#;

pub const CREATE_PRESENTATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS presentations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
)
"#;

pub const CREATE_SLIDES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS slides (
    id TEXT PRIMARY KEY,
    presentation_id TEXT NOT NULL REFERENCES presentations(id) ON DELETE CASCADE,
    slide_data TEXT NOT NULL,
    order_index INTEGER NOT NULL
)
"#;

pub const CREATE_MEDIA_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS media (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    media_type TEXT NOT NULL,
    thumbnail BLOB,
    metadata TEXT,
    created_at INTEGER NOT NULL
)
"#;

pub const CREATE_SONGS_FTS: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS songs_fts USING fts5(
    title,
    artist,
    content='songs',
    content_rowid='rowid'
)
"#;
