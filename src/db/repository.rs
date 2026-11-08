use crate::Result;
use crate::db::Database;
use crate::domain::{
    BibleImportFile, BibleTranslation, BibleVerse, LibraryAsset, Playlist, PlaylistItem,
    Presentation, Slide, SlideTheme, Song, Verse,
};
use anyhow::anyhow;
use chrono::Utc;
use rusqlite::params;
use std::sync::Arc;
use uuid::Uuid;

/// How many verses a search shows at most.
///
/// A Bible holds tens of thousands of verses, so the list is cut short. The
/// caller is told that it was, so the operator is not left believing that
/// the list is everything.
pub const BIBLE_SEARCH_LIMIT: usize = 200;

#[inline]
/// The timeline of a presentation, read from the column it is written in.
///
/// A row that was written before the timeline existed has no timeline, and
/// a timeline that cannot be read is no timeline: a show that cannot be
/// read is still a show.
fn timeline_from_json(json: Option<&str>) -> crate::domain::Timeline {
    json.and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_default()
}

fn ts_to_utc(ts: i64) -> chrono::DateTime<Utc> {
    chrono::TimeZone::timestamp_opt(&Utc, ts, 0)
        .single()
        .unwrap_or_default()
}

#[derive(Clone)]
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
                "SELECT id, name, created_at, updated_at, timeline \
                 FROM presentations ORDER BY updated_at DESC, rowid DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Presentation {
                    timeline: timeline_from_json(row.get::<_, Option<String>>(4)?.as_deref()),
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slides: Vec::new(),
                    created_at: ts_to_utc(row.get(2)?),
                    updated_at: ts_to_utc(row.get(3)?),
                })
            })?;
            let mut presentations = rows
                .collect::<rusqlite::Result<Vec<Presentation>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;

            // Load the full slides for each presentation so list rows reflect
            // the real slide count (and contents) without a second fetch.
            let mut slide_stmt = conn.prepare(
                "SELECT slide_data FROM slides \
                 WHERE presentation_id = ?1 ORDER BY order_index ASC, rowid ASC",
            )?;
            for pres in &mut presentations {
                pres.slides = slide_stmt
                    .query_map(params![&pres.id], |row| {
                        let data: String = row.get(0)?;
                        serde_json::from_str::<Slide>(&data)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
            }
            Ok(presentations)
        })
    }

    pub fn get_presentation(&self, id: &str) -> Result<Presentation> {
        self.db.with_conn(|conn| {
            let mut presentation = conn.query_row(
                "SELECT id, name, created_at, updated_at, timeline FROM presentations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Presentation {
                        timeline: timeline_from_json(row.get::<_, Option<String>>(4)?.as_deref()),
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
                 WHERE presentation_id = ?1 ORDER BY order_index ASC, rowid ASC",
            )?;
            presentation.slides = stmt
                .query_map(params![id], |row| {
                    let data: String = row.get(1)?;
                    serde_json::from_str::<Slide>(&data)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
            Ok(presentation)
        })
    }

    pub fn create_presentation(&self, name: &str) -> Result<Presentation> {
        let presentation = Presentation {
            timeline: crate::domain::Timeline::default(),
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

    /// Save the presentation's timeline.
    ///
    /// The timeline is the show as a piece of time, and it is prepared once
    /// and used every week: a show whose timings are forgotten between
    /// services is a show that has to be rebuilt every week.
    pub fn update_timeline(&self, id: &str, timeline: &crate::domain::Timeline) -> Result<()> {
        let json = serde_json::to_string(timeline)?;
        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE presentations SET timeline = ?1, updated_at = ?2 WHERE id = ?3",
                params![json, Utc::now().timestamp(), id],
            )?;
            Ok(())
        })
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

    /// The order a new slide takes at the end of the presentation.
    ///
    /// The count of the slides is not enough: deleting a slide leaves a
    /// gap in the order, so the count would put the new slide where the
    /// deleted one was.
    pub fn next_slide_order(&self, presentation_id: &str) -> Result<i32> {
        self.db.with_conn(|conn| {
            let max: Option<i32> = conn.query_row(
                "SELECT MAX(order_index) FROM slides WHERE presentation_id = ?1",
                params![presentation_id],
                |row| row.get(0),
            )?;
            Ok(max.map_or(0, |max| max + 1))
        })
    }

    pub fn update_slide(&self, slide: &Slide) -> Result<()> {
        let slide_data = serde_json::to_string(slide)?;
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            // The row has to be there: an update that matches nothing
            // reports success, and the operator's work would be lost
            // without a word.
            let changed = tx.execute(
                "UPDATE slides SET slide_data = ?1 WHERE id = ?2",
                params![&slide_data, &slide.id],
            )?;
            if changed == 0 {
                anyhow::bail!("slide {} is not in the library", slide.id);
            }
            tx.execute(
                "UPDATE presentations SET updated_at = ?1 \
                 WHERE id = (SELECT presentation_id FROM slides WHERE id = ?2)",
                params![Utc::now().timestamp(), &slide.id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Create a presentation and its slides in one transaction.
    ///
    /// The presentation and its slides are one thing: a presentation whose
    /// slides failed to save is an empty entry in the rail, while the
    /// operator is told the slides are there.
    pub fn create_presentation_with_slides(
        &self,
        name: &str,
        slides: &[Slide],
    ) -> Result<Presentation> {
        let presentation = Presentation {
            timeline: crate::domain::Timeline::default(),
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            slides: slides.to_vec(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO presentations (id, name, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    &presentation.id,
                    &presentation.name,
                    presentation.created_at.timestamp(),
                    presentation.updated_at.timestamp(),
                ],
            )?;
            for (order, slide) in slides.iter().enumerate() {
                let slide_data = serde_json::to_string(slide)?;
                tx.execute(
                    "INSERT INTO slides (id, presentation_id, slide_data, order_index) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![&slide.id, &presentation.id, &slide_data, order as i64],
                )?;
            }
            tx.commit()?;
            Ok(())
        })?;
        Ok(presentation)
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
            // The whole presentation is renumbered, and only its own slides
            // are touched: a partial list would leave two slides sharing an
            // index, and the order between them would be undefined.
            tx.execute(
                "UPDATE slides SET order_index = -1 WHERE presentation_id = ?1",
                params![presentation_id],
            )?;
            for (index, slide_id) in slide_ids.iter().enumerate() {
                tx.execute(
                    "UPDATE slides SET order_index = ?1 WHERE id = ?2 AND presentation_id = ?3",
                    params![index as i32, slide_id, presentation_id],
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

#[derive(Clone)]
pub struct LibraryRepository {
    db: Arc<Database>,
}

impl LibraryRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Record that an asset was used now.
    ///
    /// The stamp is kept in **milliseconds** so two assets used within the
    /// same second still order correctly in the recently-used strip; with
    /// second resolution they would tie and the order would be arbitrary.
    pub fn touch_asset(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute(
                "UPDATE media SET last_used_at = ?2 WHERE id = ?1",
                params![id, Utc::now().timestamp_millis()],
            )?;
            Ok(())
        })
    }

    /// The assets that were used most recently, newest first.
    ///
    /// The `rowid` tiebreaker keeps the order stable for assets that were
    /// used in the same millisecond.
    pub fn recently_used_assets(&self, limit: usize) -> Result<Vec<LibraryAsset>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, media_type, metadata, created_at \
                 FROM media WHERE last_used_at > 0 \
                 ORDER BY last_used_at DESC, rowid DESC LIMIT ?1",
            )?;
            let assets = stmt
                .query_map(params![limit as i64], |row| {
                    let id: String = row.get(0)?;
                    let path: String = row.get(1)?;
                    let media_type: String = row.get(2)?;
                    let metadata: Option<String> = row.get(3)?;
                    let ts: i64 = row.get(4)?;
                    let name = metadata
                        .as_deref()
                        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
                        .and_then(|v| {
                            v.get("name")
                                .and_then(|n| n.as_str())
                                .map(|n| n.to_string())
                        })
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
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(assets)
        })
    }

    pub fn list_assets(&self) -> Result<Vec<LibraryAsset>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, path, media_type, metadata, created_at \
                 FROM media ORDER BY created_at DESC, rowid DESC",
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
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
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

#[derive(Clone)]
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
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
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

#[derive(Clone)]
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
                 created_at, updated_at FROM songs ORDER BY updated_at DESC, rowid DESC",
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
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
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
            // The term is quoted: an apostrophe or a hyphen would
            // otherwise be a syntax error in the match expression.
            let pattern = format!("\"{}\"*", query.replace('"', "\"\""));
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
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
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
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| anyhow!("row deserialization failed: {e}"))?;
            Ok(song)
        })
    }

    /// Persist a song **and its verses** in one transaction.
    ///
    /// The verses live in their own table, so a song is only really saved
    /// when both are written. Doing both in one transaction means a song
    /// whose metadata was written but whose lyrics were not cannot happen:
    /// the import path used to write only the metadata, and the lyrics were
    /// silently lost.
    pub fn save_song_with_verses(&self, song: &Song) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            // The search index is an external-content table, so it keeps the
            // tokens of the old title until the row is deleted from it: a
            // renamed song would still be found by its old name. The old
            // values are handed to the index before the row changes.
            // A song that is not in the table yet is simply new; a lookup
            // that fails for any other reason is a fault the operator has to
            // hear about: the index would keep the old title's words.
            let indexed: Option<(i64, String, Option<String>)> = {
                let mut stmt =
                    tx.prepare("SELECT rowid, title, artist FROM songs WHERE id = ?1")?;
                match stmt.query_row(params![song.id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                }) {
                    Ok(found) => Some(found),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(e.into()),
                }
            };
            if let Some((rowid, title, artist)) = indexed {
                tx.execute(
                    "INSERT INTO songs_fts(songs_fts, rowid, title, artist) \
                     VALUES('delete', ?1, ?2, ?3)",
                    params![rowid, title, artist],
                )?;
            }
            tx.execute(
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
            tx.execute(
                "INSERT INTO songs_fts(rowid, title, artist) \
                 SELECT rowid, title, artist FROM songs WHERE id = ?1",
                params![song.id],
            )?;
            tx.execute("DELETE FROM verses WHERE song_id = ?1", params![song.id])?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO verses (id, song_id, label, content, order_index) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )?;
                for verse in &song.verses {
                    stmt.execute(params![
                        verse.id,
                        song.id,
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

    /// Persist a song's metadata only, leaving its verses untouched.
    pub fn save_song(&self, song: &Song) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let indexed: Option<(i64, String, Option<String>)> = {
                let mut stmt =
                    tx.prepare("SELECT rowid, title, artist FROM songs WHERE id = ?1")?;
                match stmt.query_row(params![song.id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                }) {
                    Ok(found) => Some(found),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(e.into()),
                }
            };
            if let Some((rowid, title, artist)) = indexed {
                tx.execute(
                    "INSERT INTO songs_fts(songs_fts, rowid, title, artist) \
                     VALUES('delete', ?1, ?2, ?3)",
                    params![rowid, title, artist],
                )?;
            }
            tx.execute(
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
            tx.execute(
                "INSERT INTO songs_fts(rowid, title, artist) \
                 SELECT rowid, title, artist FROM songs WHERE id = ?1",
                params![song.id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    /// Replace every verse of a song.
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

    pub fn delete_song(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            // The index and the row are changed in one transaction: an index
            // that is emptied while the row survives would leave a song that
            // cannot be found by its own name.
            let tx = conn.unchecked_transaction()?;
            // The search index has to be told about the deletion first. The
            // index is an external-content table, so it keeps the tokens of
            // the row until the row is removed from it — and SQLite reuses
            // the rowid, so the next song would inherit the dead one's
            // search terms.
            let indexed: Option<(i64, String, Option<String>)> = {
                let mut stmt =
                    tx.prepare("SELECT rowid, title, artist FROM songs WHERE id = ?1")?;
                match stmt.query_row(params![id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                }) {
                    Ok(found) => Some(found),
                    Err(rusqlite::Error::QueryReturnedNoRows) => None,
                    Err(e) => return Err(e.into()),
                }
            };
            if let Some((rowid, title, artist)) = indexed {
                tx.execute(
                    "INSERT INTO songs_fts(songs_fts, rowid, title, artist) \
                     VALUES('delete', ?1, ?2, ?3)",
                    params![rowid, title, artist],
                )?;
            }
            tx.execute("DELETE FROM songs WHERE id = ?1", params![id])?;
            tx.commit()?;
            Ok(())
        })
    }
}

/// The service calendar.
///
/// The events are the church's week: the services that happen, and the
/// playlist that plays at each one.
#[derive(Clone)]
pub struct CalendarRepository {
    db: Arc<Database>,
}

impl CalendarRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Every service on the calendar, soonest first.
    pub fn list_events(&self) -> Result<Vec<crate::domain::calendar::ServiceEvent>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT event_data FROM service_events ORDER BY starts_at ASC, rowid ASC",
            )?;
            let events = stmt
                .query_map([], |row| {
                    let data: String = row.get(0)?;
                    Ok::<_, rusqlite::Error>(
                        serde_json::from_str::<crate::domain::calendar::ServiceEvent>(&data).ok(),
                    )
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            // An event that cannot be read is not an event the operator can
            // run, so it is left out rather than shown as a broken one.
            Ok(events.into_iter().flatten().collect())
        })
    }

    /// Save a service.
    pub fn save_event(&self, event: &crate::domain::calendar::ServiceEvent) -> Result<()> {
        let json = serde_json::to_string(event)?;
        self.db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO service_events (id, event_data, starts_at, created_at)                  VALUES (?1, ?2, ?3, ?4)                  ON CONFLICT(id) DO UPDATE SET event_data = ?2, starts_at = ?3",
                params![event.id, json, event.starts_at, Utc::now().timestamp()],
            )?;
            Ok(())
        })
    }

    /// Take a service off the calendar.
    pub fn delete_event(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM service_events WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

#[derive(Clone)]
pub struct PlaylistRepository {
    db: Arc<Database>,
}

impl PlaylistRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn list_playlists(&self) -> Result<Vec<Playlist>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT p.id, p.name, p.created_at, p.updated_at, \
                 (SELECT COUNT(*) FROM service_items i WHERE i.plan_id = p.id) \
                 FROM service_plans p ORDER BY p.created_at DESC, p.rowid DESC",
            )?;
            let plans = stmt
                .query_map([], |row| {
                    let count: i64 = row.get(4)?;
                    Ok(Playlist {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        // The list carries the count, not the items: the
                        // sidebar shows how many items a plan has, and
                        // loading every item of every plan would be a lot
                        // of work for a list.
                        items: Vec::new(),
                        item_count: count.max(0) as usize,
                        created_at: ts_to_utc(row.get(2)?),
                        updated_at: ts_to_utc(row.get(3)?),
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(plans)
        })
    }

    pub fn get_playlist(&self, id: &str) -> Result<Playlist> {
        self.db.with_conn(|conn| {
            let mut plan: Playlist = conn.query_row(
                "SELECT id, name, created_at, updated_at FROM service_plans WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Playlist {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        items: Vec::new(),
                        item_count: 0,
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
            // A malformed item must not silently vanish from a service plan.
            plan.items = raw
                .iter()
                .map(|s| {
                    serde_json::from_str(s).map_err(|e| anyhow!("playlist item json error: {e}"))
                })
                .collect::<Result<Vec<PlaylistItem>>>()?;
            plan.item_count = plan.items.len();
            Ok(plan)
        })
    }

    /// Persist a playlist's name.
    ///
    /// The row is updated rather than replaced: a replace would delete the
    /// row, and the cascade would take the items with it — the plan would be
    /// saved empty.
    pub fn save_playlist(&self, plan: &Playlist) -> Result<()> {
        self.db.with_conn(|conn| {
            let updated = conn.execute(
                "UPDATE service_plans SET name = ?2, updated_at = ?3 WHERE id = ?1",
                params![plan.id, plan.name, plan.updated_at.timestamp()],
            )?;
            if updated == 0 {
                conn.execute(
                    "INSERT INTO service_plans (id, name, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        plan.id,
                        plan.name,
                        plan.created_at.timestamp(),
                        plan.updated_at.timestamp()
                    ],
                )?;
            }
            Ok(())
        })
    }

    pub fn delete_playlist(&self, id: &str) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute("DELETE FROM service_plans WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    /// Persist a playlist and its items in one transaction.
    ///
    /// The plan row is updated rather than replaced: a replace would delete
    /// the row, and the cascade would take the items with it.
    pub fn save_playlist_with_items(&self, plan: &Playlist) -> Result<()> {
        self.db.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let updated = tx.execute(
                "UPDATE service_plans SET name = ?2, updated_at = ?3 WHERE id = ?1",
                params![plan.id, plan.name, plan.updated_at.timestamp()],
            )?;
            if updated == 0 {
                tx.execute(
                    "INSERT INTO service_plans (id, name, created_at, updated_at)                      VALUES (?1, ?2, ?3, ?4)",
                    params![
                        plan.id,
                        plan.name,
                        plan.created_at.timestamp(),
                        plan.updated_at.timestamp()
                    ],
                )?;
            }
            tx.execute(
                "DELETE FROM service_items WHERE plan_id = ?1",
                params![plan.id],
            )?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO service_items (id, plan_id, item_data, position)                      VALUES (?1, ?2, ?3, ?4)",
                )?;
                for (pos, item) in plan.items.iter().enumerate() {
                    let data = serde_json::to_string(item).map_err(|e| anyhow::anyhow!("{e}"))?;
                    stmt.execute(params![
                        Uuid::new_v4().to_string(),
                        plan.id,
                        data,
                        pos as i64
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn save_items(&self, plan_id: &str, items: &[PlaylistItem]) -> Result<()> {
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

#[derive(Clone)]
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
            // The search index is filled from the verses that were just
            // written: a translation whose verses are not indexed cannot be
            // searched.
            tx.execute(
                "INSERT INTO bible_verses_fts(bible_verses_fts) VALUES('rebuild')",
                [],
            )?;
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
            let tx = conn.unchecked_transaction()?;
            // The search index is an external-content index: it holds the
            // words of the verses, not the verses themselves. The cascade
            // removes the rows, but the index has to be told — a deleted
            // translation that still answers a search is scripture the
            // operator thought they had removed.
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO bible_verses_fts(bible_verses_fts, rowid, text) \
                     SELECT 'delete', rowid, text FROM bible_verses WHERE translation_id = ?1",
                )?;
                stmt.execute(params![id])?;
            }
            tx.execute("DELETE FROM bible_translations WHERE id = ?1", params![id])?;
            tx.commit()?;
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
            // The verses are found through the search index: a Bible holds
            // tens of thousands of them, and scanning every one of them on
            // every keystroke is a lag the operator feels.
            //
            // The term is quoted: an apostrophe or a hyphen would otherwise
            // be a syntax error in the match expression.
            let pattern = format!("\"{}\"*", query.replace('"', "\"\""));
            // One row more than the limit is fetched, so the caller can say
            // that the list is cut short: a search that quietly drops the
            // rest of the matches tells the operator a lie.
            let mut stmt = conn.prepare(
                "SELECT v.id, v.translation_id, v.book, v.chapter, v.verse, v.text \
                 FROM bible_verses v \
                 JOIN bible_verses_fts ON bible_verses_fts.rowid = v.rowid \
                 WHERE bible_verses_fts MATCH ?1 AND v.translation_id = ?2 \
                 ORDER BY v.book, v.chapter, v.verse LIMIT ?3",
            )?;
            let rows = stmt.query_map(
                params![pattern, translation_id, (BIBLE_SEARCH_LIMIT + 1) as i64],
                |row| {
                    Ok(BibleVerse {
                        id: row.get(0)?,
                        translation_id: row.get(1)?,
                        book: row.get(2)?,
                        chapter: row.get(3)?,
                        verse: row.get(4)?,
                        text: row.get(5)?,
                    })
                },
            )?;
            // One row more than the limit is returned: the extra row says
            // that the list is cut short, so the operator can be told.
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(Into::into)
        })
    }
}
