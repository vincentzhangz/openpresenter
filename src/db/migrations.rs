use super::schema::*;
use crate::Result;
use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute(CREATE_SONGS_TABLE, [])?;
    conn.execute(CREATE_VERSES_TABLE, [])?;
    conn.execute(CREATE_PRESENTATIONS_TABLE, [])?;
    conn.execute(CREATE_SLIDES_TABLE, [])?;
    conn.execute(CREATE_MEDIA_TABLE, [])?;

    conn.execute(CREATE_SONGS_FTS, [])?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_verses_song_id ON verses(song_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_slides_presentation_id ON slides(presentation_id)",
        [],
    )?;

    Ok(())
}
