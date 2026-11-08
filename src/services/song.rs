//! Song use-cases.
//!
//! A song is its metadata **and** its verses: the lyrics live in their own
//! table, so saving a song means writing both. The service owns that
//! knowledge, so no caller can save a song and forget its lyrics — which is
//! what the import path used to do.

use crate::Result;
use crate::db::SongRepository;
use crate::domain::Song;
#[cfg(test)]
use crate::domain::Verse;

#[derive(Clone)]
pub struct SongService {
    repo: SongRepository,
}

impl SongService {
    pub fn new(db: std::sync::Arc<crate::db::Database>) -> Self {
        Self {
            repo: SongRepository::new(db),
        }
    }

    /// The songs in the library, newest first.
    pub fn list(&self) -> Result<Vec<Song>> {
        self.repo.list_songs()
    }

    /// The songs whose title or artist matches the query.
    pub fn search(&self, query: &str) -> Result<Vec<Song>> {
        self.repo.search_songs(query)
    }

    /// A song with its verses.
    pub fn get(&self, id: &str) -> Result<Song> {
        self.repo.get_song(id)
    }

    /// Persist a song and its verses.
    pub fn save(&self, song: &Song) -> Result<()> {
        self.repo.save_song_with_verses(song)
    }

    /// Delete a song and its verses.
    pub fn delete(&self, id: &str) -> Result<()> {
        self.repo.delete_song(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::sync::Arc;

    fn service() -> SongService {
        let db = Arc::new(Database::in_memory().expect("in-memory DB"));
        SongService::new(db)
    }

    /// A song is its lyrics: saving it has to keep them. The import path
    /// used to write only the metadata, and the lyrics were lost.
    #[test]
    fn saving_a_song_keeps_its_verses() {
        let service = service();
        let mut song = Song::new("Amazing Grace".to_string());
        song.verses.push(Verse {
            id: "verse-1".to_string(),
            label: "Verse 1".to_string(),
            content: "How sweet the sound".to_string(),
            order_index: 0,
        });

        service.save(&song).expect("the song is saved");

        let loaded = service.get(&song.id).expect("the song is read back");
        assert_eq!(loaded.verses.len(), 1, "the lyrics survive the save");
        assert_eq!(loaded.verses[0].content, "How sweet the sound");
    }

    /// Saving the same song again replaces the verses instead of doubling
    /// them.
    #[test]
    fn saving_a_song_twice_replaces_its_verses() {
        let service = service();
        let mut song = Song::new("Twice".to_string());
        song.verses.push(Verse {
            id: "verse-1".to_string(),
            label: String::new(),
            content: "First".to_string(),
            order_index: 0,
        });
        service.save(&song).expect("the first save");

        song.verses[0].content = "Second".to_string();
        service.save(&song).expect("the second save");

        let loaded = service.get(&song.id).expect("the song is read back");
        assert_eq!(loaded.verses.len(), 1, "the verses are replaced");
        assert_eq!(loaded.verses[0].content, "Second");
    }
}
