use crate::Result;
use rusqlite::{Connection, params};

const LATEST_VERSION: u32 = 3;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    bootstrap_version_table(conn)?;
    let current = read_version(conn)?;
    for v in (current + 1)..=LATEST_VERSION {
        run_single(conn, v)?;
        write_version(conn, v)?;
    }
    Ok(())
}

fn bootstrap_version_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        [],
    )?;
    let row_count: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))?;
    if row_count == 0 {
        let baseline = detect_baseline_version(conn)?;
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![baseline],
        )?;
    }
    Ok(())
}

fn read_version(conn: &Connection) -> Result<u32> {
    let v: i64 = conn.query_row("SELECT version FROM schema_version", [], |r| r.get(0))?;
    Ok(v as u32)
}

fn write_version(conn: &Connection, v: u32) -> Result<()> {
    conn.execute("UPDATE schema_version SET version = ?1", params![v])?;
    Ok(())
}

fn detect_baseline_version(conn: &Connection) -> Result<u32> {
    if !table_exists(conn, "presentations")? {
        return Ok(0);
    }
    if table_exists(conn, "bible_translations")? {
        return Ok(3);
    }
    if table_exists(conn, "service_plans")? {
        return Ok(2);
    }
    Ok(1)
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name=?2",
        params![table, column],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

fn run_single(conn: &Connection, version: u32) -> Result<()> {
    match version {
        1 => migrate_v1(conn),
        2 => migrate_v2(conn),
        3 => migrate_v3(conn),
        v => Err(anyhow::anyhow!("unknown migration version {v}")),
    }
}

fn migrate_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS songs (
            id            TEXT PRIMARY KEY,
            title         TEXT NOT NULL,
            artist        TEXT,
            copyright     TEXT,
            ccli_number   TEXT,
            key_signature TEXT,
            bpm           INTEGER,
            created_at    INTEGER NOT NULL,
            updated_at    INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS verses (
            id          TEXT PRIMARY KEY,
            song_id     TEXT NOT NULL REFERENCES songs(id) ON DELETE CASCADE,
            label       TEXT NOT NULL,
            content     TEXT NOT NULL,
            order_index INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS presentations (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS slides (
            id              TEXT PRIMARY KEY,
            presentation_id TEXT NOT NULL REFERENCES presentations(id) ON DELETE CASCADE,
            slide_data      TEXT NOT NULL,
            order_index     INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS media (
            id         TEXT PRIMARY KEY,
            path       TEXT NOT NULL,
            media_type TEXT NOT NULL,
            thumbnail  BLOB,
            metadata   TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS themes (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            theme_data TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS songs_fts USING fts5(
            title, artist,
            content='songs', content_rowid='rowid'
        );
        CREATE INDEX IF NOT EXISTS idx_verses_song_id ON verses(song_id);
        CREATE INDEX IF NOT EXISTS idx_slides_presentation_id ON slides(presentation_id);",
    )?;
    for (col, sql_type) in &[
        ("ccli_number", "TEXT"),
        ("key_signature", "TEXT"),
        ("bpm", "INTEGER"),
    ] {
        if !column_exists(conn, "songs", col)? {
            conn.execute(
                &format!("ALTER TABLE songs ADD COLUMN {col} {sql_type}"),
                [],
            )?;
        }
    }
    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS service_plans (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS service_items (
            id        TEXT PRIMARY KEY,
            plan_id   TEXT NOT NULL REFERENCES service_plans(id) ON DELETE CASCADE,
            item_data TEXT NOT NULL,
            position  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_service_items_plan_id ON service_items(plan_id);",
    )?;
    Ok(())
}

fn migrate_v3(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS bible_translations (
            id   TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            abbr TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bible_verses (
            id             TEXT PRIMARY KEY,
            translation_id TEXT NOT NULL REFERENCES bible_translations(id) ON DELETE CASCADE,
            book           TEXT NOT NULL,
            chapter        INTEGER NOT NULL,
            verse          INTEGER NOT NULL,
            text           TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS bible_verses_fts USING fts5(
            text, book,
            content='bible_verses', content_rowid='rowid'
        );
        CREATE INDEX IF NOT EXISTS idx_bible_verses_translation ON bible_verses(translation_id);
        CREATE INDEX IF NOT EXISTS idx_bible_verses_ref ON bible_verses(translation_id, book, chapter);",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh() -> Connection {
        Connection::open_in_memory().expect("in-memory DB")
    }

    #[test]
    fn run_migrations_on_fresh_db_succeeds() {
        run_migrations(&fresh()).expect("migrations should succeed");
    }

    #[test]
    fn run_migrations_is_idempotent() {
        let conn = fresh();
        run_migrations(&conn).expect("first run");
        run_migrations(&conn).expect("second run should be a no-op");
    }

    #[test]
    fn core_tables_exist_after_migration() {
        let conn = fresh();
        run_migrations(&conn).expect("migrations");
        for table in &[
            "songs",
            "verses",
            "presentations",
            "slides",
            "themes",
            "media",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap_or_else(|e| panic!("table '{table}' missing: {e}"));
            assert_eq!(count, 0, "table '{table}' should be empty");
        }
    }

    #[test]
    fn service_plan_tables_exist_after_migration() {
        let conn = fresh();
        run_migrations(&conn).expect("migrations");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM service_plans", [], |r| r.get(0))
            .expect("service_plans table");
        assert_eq!(count, 0);
    }

    #[test]
    fn bible_tables_exist_after_migration() {
        let conn = fresh();
        run_migrations(&conn).expect("migrations");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM bible_translations", [], |r| r.get(0))
            .expect("bible_translations table");
        assert_eq!(count, 0);
    }

    #[test]
    fn schema_version_equals_latest_after_migration() {
        let conn = fresh();
        run_migrations(&conn).expect("migrations");
        let v: u32 = conn
            .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
            .expect("schema_version row");
        assert_eq!(v, LATEST_VERSION);
    }
}
