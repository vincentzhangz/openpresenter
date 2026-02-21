use crate::Result;
use crate::db::Database;
use crate::slides::{
    BibleImportFile, BibleTranslation, BibleVerse, LibraryAsset, Presentation, ServiceItem,
    ServicePlan, Slide, SlideTheme, Song, Verse,
};
use chrono::Utc;
use rusqlite::params;
use std::sync::Arc;
use uuid::Uuid;

#[inline]
fn ts_to_utc(ts: i64) -> chrono::DateTime<Utc> {
    chrono::TimeZone::timestamp_opt(&Utc, ts, 0)
        .single()
        .unwrap_or_default()
}

pub struct PresentationRepository {
    db: Arc<Database>,
}

impl PresentationRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_presentations(&self) -> Result<Vec<Presentation>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, created_at, updated_at \
                 FROM presentations ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Presentation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slides: Vec::new(),
                    created_at: ts_to_utc(row.get(2)?),
                    updated_at: ts_to_utc(row.get(3)?),
                })
            })?;
            let presentations = rows
                .filter_map(|r| r.map_err(|e| eprintln!("presentation row error: {e}")).ok())
                .collect();
            Ok(presentations)
        })
    }

    pub fn get_presentation(&self, id: &str) -> Result<Presentation> {
        self.db.with_conn(|conn| {
            let mut presentation = conn.query_row(
                "SELECT id, name, created_at, updated_at FROM presentations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Presentation {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        slides: Vec::new(),
                        created_at: ts_to_utc(row.get(2)?),
                        updated_at: ts_to_utc(row.get(3)?),
                    })
                },
            )?;
            let mut stmt = conn.prepare(
                "SELECT id, slide_data, order_index FROM slides \
                 WHERE presentation_id = ?1 ORDER BY order_index ASC",
            )?;
            presentation.slides = stmt
                .query_map(params![id], |row| {
                    let data: String = row.get(1)?;
                    serde_json::from_str::<Slide>(&data)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("slide row error: {e}")).ok())
                .collect();
            Ok(presentation)
        })
    }

    pub fn create_presentation(&self, name: &str) -> Result<Presentation> {
        let presentation = Presentation {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            slides: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO presentations (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    &presentation.id,
                    &presentation.name,
                    presentation.created_at.timestamp(),
                    presentation.updated_at.timestamp(),
                ],
            )?;
            Ok(())
        })?;
        Ok(presentation)
    }

    pub fn update_presentation(&self, id: &str, name: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE presentations SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, Utc::now().timestamp(), id],
            )?;
            Ok(())
        })
    }

    pub fn delete_presentation(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM presentations WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn add_slide(&self, presentation_id: &str, slide: &Slide, order_index: i32) -> Result<()> {
        let slide_data = serde_json::to_string(slide)?;
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO slides (id, presentation_id, slide_data, order_index) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![&slide.id, presentation_id, &slide_data, order_index],
            )?;
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 WHERE id = ?2",
                params![Utc::now().timestamp(), presentation_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn update_slide(&self, slide: &Slide) -> Result<()> {
        let slide_data = serde_json::to_string(slide)?;
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "UPDATE slides SET slide_data = ?1 WHERE id = ?2",
                params![&slide_data, &slide.id],
            )?;
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 \
                 WHERE id = (SELECT presentation_id FROM slides WHERE id = ?2)",
                params![Utc::now().timestamp(), &slide.id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn delete_slide(&self, slide_id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            let presentation_id: String = conn.query_row(
                "SELECT presentation_id FROM slides WHERE id = ?1",
                params![slide_id],
                |row| row.get(0),
            )?;
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM slides WHERE id = ?1", params![slide_id])?;
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 WHERE id = ?2",
                params![Utc::now().timestamp(), &presentation_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn reorder_slides(&self, presentation_id: &str, slide_ids: &[String]) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for (index, slide_id) in slide_ids.iter().enumerate() {
                tx.execute(
                    "UPDATE slides SET order_index = ?1 WHERE id = ?2",
                    params![index as i32, slide_id],
                )?;
            }
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 WHERE id = ?2",
                params![Utc::now().timestamp(), presentation_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn replace_presentation_slides(
        &self,
        presentation_id: &str,
        slides: &[Slide],
    ) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "DELETE FROM slides WHERE presentation_id = ?1",
                params![presentation_id],
            )?;
            for (i, slide) in slides.iter().enumerate() {
                let slide_data = serde_json::to_string(slide)?;
                tx.execute(
                    "INSERT INTO slides (id, presentation_id, slide_data, order_index) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![slide.id, presentation_id, slide_data, i as i32],
                )?;
            }
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 WHERE id = ?2",
                params![Utc::now().timestamp(), presentation_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }
}

pub struct LibraryRepository {
    db: Arc<Database>,
}

impl LibraryRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_assets(&self) -> Result<Vec<LibraryAsset>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, media_type, metadata, created_at \
                 FROM media ORDER BY created_at DESC",
            )?;
            let assets = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let path: String = row.get(1)?;
                    let media_type: String = row.get(2)?;
                    let metadata: Option<String> = row.get(3)?;
                    let ts: i64 = row.get(4)?;
                    let name = metadata
                        .as_deref()
                        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                        .and_then(|v| v["name"].as_str().map(|s| s.to_owned()))
                        .unwrap_or_else(|| {
                            std::path::Path::new(&path)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.clone())
                        });
                    Ok(LibraryAsset {
                        id,
                        name,
                        path,
                        media_type,
                        created_at: ts_to_utc(ts),
                    })
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("asset row error: {e}")).ok())
                .collect();
            Ok(assets)
        })
    }

    pub fn add_asset(&self, path: &str, media_type: &str) -> Result<LibraryAsset> {
        let id = Uuid::new_v4().to_string();
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_owned());
        let metadata = serde_json::json!({ "name": &name }).to_string();
        let now = Utc::now();
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO media (id, path, media_type, metadata, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![&id, path, media_type, &metadata, now.timestamp()],
            )?;
            Ok(())
        })?;
        Ok(LibraryAsset {
            id,
            name,
            path: path.to_owned(),
            media_type: media_type.to_owned(),
            created_at: now,
        })
    }

    pub fn delete_asset(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM media WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

pub struct ThemeRepository {
    db: Arc<Database>,
}

impl ThemeRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_themes(&self) -> Result<Vec<SlideTheme>> {
        self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT theme_data, created_at FROM themes ORDER BY created_at ASC")?;
            let themes = stmt
                .query_map([], |row| {
                    let theme_data: String = row.get(0)?;
                    let ts: i64 = row.get(1)?;
                    let mut theme: SlideTheme = serde_json::from_str(&theme_data)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                    theme.created_at = ts_to_utc(ts);
                    Ok(theme)
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("theme row error: {e}")).ok())
                .collect();
            Ok(themes)
        })
    }

    pub fn save_theme(&self, theme: &SlideTheme) -> Result<()> {
        let theme_data = serde_json::to_string(theme)?;
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO themes (id, name, theme_data, created_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    theme.id,
                    theme.name,
                    &theme_data,
                    theme.created_at.timestamp()
                ],
            )?;
            Ok(())
        })
    }

    pub fn delete_theme(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM themes WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

pub struct SongRepository {
    db: Arc<Database>,
}

impl SongRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_songs(&self) -> Result<Vec<Song>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, artist, copyright, ccli_number, key_signature, bpm, \
                 created_at, updated_at FROM songs ORDER BY updated_at DESC",
            )?;
            let songs = stmt
                .query_map([], |row| {
                    Ok(Song {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        artist: row.get(2)?,
                        copyright: row.get(3)?,
                        ccli_number: row.get(4)?,
                        key_signature: row.get(5)?,
                        bpm: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                        verses: Vec::new(),
                        created_at: ts_to_utc(row.get(7)?),
                        updated_at: ts_to_utc(row.get(8)?),
                    })
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("song row error: {e}")).ok())
                .collect();
            Ok(songs)
        })
    }

    pub fn search_songs(&self, query: &str) -> Result<Vec<Song>> {
        if query.trim().is_empty() {
            return self.list_songs();
        }
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT s.id, s.title, s.artist, s.copyright, s.ccli_number, \
                 s.key_signature, s.bpm, s.created_at, s.updated_at \
                 FROM songs s \
                 JOIN songs_fts ON songs_fts.rowid = s.rowid \
                 WHERE songs_fts MATCH ?1 ORDER BY rank",
            )?;
            let pattern = format!("{query}*");
            let songs = stmt
                .query_map(params![pattern], |row| {
                    Ok(Song {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        artist: row.get(2)?,
                        copyright: row.get(3)?,
                        ccli_number: row.get(4)?,
                        key_signature: row.get(5)?,
                        bpm: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                        verses: Vec::new(),
                        created_at: ts_to_utc(row.get(7)?),
                        updated_at: ts_to_utc(row.get(8)?),
                    })
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("song search row error: {e}")).ok())
                .collect();
            Ok(songs)
        })
    }

    pub fn get_song(&self, id: &str) -> Result<Song> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, artist, copyright, ccli_number, key_signature, bpm, \
                 created_at, updated_at FROM songs WHERE id = ?1",
            )?;
            let mut song: Song = stmt.query_row(params![id], |row| {
                Ok(Song {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    artist: row.get(2)?,
                    copyright: row.get(3)?,
                    ccli_number: row.get(4)?,
                    key_signature: row.get(5)?,
                    bpm: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                    verses: Vec::new(),
                    created_at: ts_to_utc(row.get(7)?),
                    updated_at: ts_to_utc(row.get(8)?),
                })
            })?;
            let mut vstmt = conn.prepare(
                "SELECT id, label, content, order_index FROM verses \
                 WHERE song_id = ?1 ORDER BY order_index",
            )?;
            song.verses = vstmt
                .query_map(params![song.id], |row| {
                    Ok(Verse {
                        id: row.get(0)?,
                        label: row.get(1)?,
                        content: row.get(2)?,
                        order_index: row.get::<_, i64>(3)? as usize,
                    })
                })?
                .filter_map(|r| r.map_err(|e| eprintln!("verse row error: {e}")).ok())
                .collect();
            Ok(song)
        })
    }

    pub fn save_song(&self, song: &Song) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO songs \
                 (id, title, artist, copyright, ccli_number, key_signature, bpm, \
                  created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    song.id,
                    song.title,
                    song.artist,
                    song.copyright,
                    song.ccli_number,
                    song.key_signature,
                    song.bpm.map(|v| v as i64),
                    song.created_at.timestamp(),
                    song.updated_at.timestamp(),
                ],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO songs_fts(rowid, title, artist) \
                 SELECT rowid, title, artist FROM songs WHERE id = ?1",
                params![song.id],
            )?;
            Ok(())
        })
    }

    pub fn delete_song(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM songs WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn replace_verses(&self, song_id: &str, verses: &[Verse]) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute("DELETE FROM verses WHERE song_id = ?1", params![song_id])?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO verses (id, song_id, label, content, order_index) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )?;
                for verse in verses {
                    stmt.execute(params![
                        verse.id,
                        song_id,
                        verse.label,
                        verse.content,
                        verse.order_index as i64,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }
}

pub struct ServiceRepository {
    db: Arc<Database>,
}

impl ServiceRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_plans(&self) -> Result<Vec<ServicePlan>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, created_at, updated_at \
                 FROM service_plans ORDER BY created_at DESC",
            )?;
            let plans = stmt
                .query_map([], |row| {
                    Ok(ServicePlan {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        items: Vec::new(),
                        created_at: ts_to_utc(row.get(2)?),
                        updated_at: ts_to_utc(row.get(3)?),
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(plans)
        })
    }

    pub fn get_plan(&self, id: &str) -> Result<ServicePlan> {
        self.db.with_conn(|conn| {
            let mut plan: ServicePlan = conn.query_row(
                "SELECT id, name, created_at, updated_at FROM service_plans WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ServicePlan {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        items: Vec::new(),
                        created_at: ts_to_utc(row.get(2)?),
                        updated_at: ts_to_utc(row.get(3)?),
                    })
                },
            )?;
            let mut stmt = conn.prepare(
                "SELECT item_data FROM service_items WHERE plan_id = ?1 ORDER BY position ASC",
            )?;
            let raw: Vec<String> = stmt
                .query_map(params![id], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            plan.items = raw
                .iter()
                .filter_map(|s| {
                    serde_json::from_str(s)
                        .map_err(|e| eprintln!("service item json error: {e}"))
                        .ok()
                })
                .collect();
            Ok(plan)
        })
    }

    pub fn save_plan(&self, plan: &ServicePlan) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO service_plans \
                 (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    plan.id,
                    plan.name,
                    plan.created_at.timestamp(),
                    plan.updated_at.timestamp()
                ],
            )?;
            Ok(())
        })
    }

    pub fn delete_plan(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM service_plans WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn save_items(&self, plan_id: &str, items: &[ServiceItem]) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "DELETE FROM service_items WHERE plan_id = ?1",
                params![plan_id],
            )?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO service_items (id, plan_id, item_data, position) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?;
                for (pos, item) in items.iter().enumerate() {
                    let data = serde_json::to_string(item).map_err(|e| anyhow::anyhow!("{e}"))?;
                    stmt.execute(params![
                        Uuid::new_v4().to_string(),
                        plan_id,
                        data,
                        pos as i64
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }
}

pub struct BibleRepository {
    db: Arc<Database>,
}

impl BibleRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_translations(&self) -> Result<Vec<BibleTranslation>> {
        self.db.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT id, name, abbr FROM bible_translations ORDER BY name")?;
            let rows = stmt.query_map([], |row| {
                Ok(BibleTranslation {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    abbr: row.get(2)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }

    pub fn import_translation(&self, file: BibleImportFile) -> Result<BibleTranslation> {
        let id = Uuid::new_v4().to_string();
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO bible_translations (id, name, abbr) VALUES (?1, ?2, ?3)",
                params![id, file.name, file.abbr],
            )?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO bible_verses \
                     (id, translation_id, book, chapter, verse, text) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )?;
                for rec in &file.verses {
                    stmt.execute(params![
                        Uuid::new_v4().to_string(),
                        id,
                        rec.book,
                        rec.chapter,
                        rec.verse,
                        rec.text,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })?;
        Ok(BibleTranslation {
            id,
            name: file.name,
            abbr: file.abbr,
        })
    }

    pub fn delete_translation(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM bible_translations WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn list_books(&self, translation_id: &str) -> Result<Vec<String>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT book FROM bible_verses \
                 WHERE translation_id = ?1 ORDER BY rowid",
            )?;
            let rows = stmt.query_map(params![translation_id], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()
                .map_err(Into::into)
        })
    }

    pub fn list_chapters(&self, translation_id: &str, book: &str) -> Result<Vec<i32>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT chapter FROM bible_verses \
                 WHERE translation_id = ?1 AND book = ?2 ORDER BY chapter",
            )?;
            let rows = stmt.query_map(params![translation_id, book], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<i32>>>()
                .map_err(Into::into)
        })
    }

    pub fn get_chapter_verses(
        &self,
        translation_id: &str,
        book: &str,
        chapter: i32,
    ) -> Result<Vec<BibleVerse>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, translation_id, book, chapter, verse, text \
                 FROM bible_verses \
                 WHERE translation_id = ?1 AND book = ?2 AND chapter = ?3 ORDER BY verse",
            )?;
            let rows = stmt.query_map(params![translation_id, book, chapter], |row| {
                Ok(BibleVerse {
                    id: row.get(0)?,
                    translation_id: row.get(1)?,
                    book: row.get(2)?,
                    chapter: row.get(3)?,
                    verse: row.get(4)?,
                    text: row.get(5)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }

    pub fn search_verses(&self, translation_id: &str, query: &str) -> Result<Vec<BibleVerse>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        self.db.with_conn(|conn| {
            let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
            let mut stmt = conn.prepare(
                "SELECT id, translation_id, book, chapter, verse, text \
                 FROM bible_verses \
                 WHERE translation_id = ?1 AND text LIKE ?2 ESCAPE '\\' \
                 ORDER BY book, chapter, verse LIMIT 200",
            )?;
            let rows = stmt.query_map(params![translation_id, pattern], |row| {
                Ok(BibleVerse {
                    id: row.get(0)?,
                    translation_id: row.get(1)?,
                    book: row.get(2)?,
                    chapter: row.get(3)?,
                    verse: row.get(4)?,
                    text: row.get(5)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }
}
