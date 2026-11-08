//! Bible use-cases.
//!
//! The Bible texts are read and imported through this service, so the
//! panel never talks to the repository directly: the rules about what a
//! search finds and what a chapter contains live here.

use crate::Result;
use crate::db::BibleRepository;
use crate::db::repository::BIBLE_SEARCH_LIMIT;
use crate::domain::{BibleImportFile, BibleTranslation, BibleVerse};

/// The result of a Bible search.
#[derive(Debug, Clone, Default)]
pub struct BibleSearch {
    /// The verses that matched.
    pub verses: Vec<BibleVerse>,
    /// Whether more verses matched than are shown.
    pub truncated: bool,
}

#[derive(Clone)]
pub struct BibleService {
    repo: BibleRepository,
}

impl BibleService {
    pub fn new(db: std::sync::Arc<crate::db::Database>) -> Self {
        Self {
            repo: BibleRepository::new(db),
        }
    }

    /// The translations that are installed.
    pub fn list_translations(&self) -> Result<Vec<BibleTranslation>> {
        self.repo.list_translations()
    }

    /// The books of a translation.
    pub fn list_books(&self, translation_id: &str) -> Result<Vec<String>> {
        self.repo.list_books(translation_id)
    }

    /// The chapters of a book.
    pub fn list_chapters(&self, translation_id: &str, book: &str) -> Result<Vec<i32>> {
        self.repo.list_chapters(translation_id, book)
    }

    /// The verses of a chapter.
    pub fn chapter_verses(
        &self,
        translation_id: &str,
        book: &str,
        chapter: i32,
    ) -> Result<Vec<BibleVerse>> {
        self.repo.get_chapter_verses(translation_id, book, chapter)
    }

    /// The verses that match a search term.
    /// Search a translation for verses.
    ///
    /// The list is cut short at [`BIBLE_SEARCH_LIMIT`], and the result says
    /// so: a search that quietly drops the rest of the matches tells the
    /// operator a lie about what is there.
    pub fn search_verses(&self, translation_id: &str, query: &str) -> Result<BibleSearch> {
        let mut verses = self.repo.search_verses(translation_id, query)?;
        let truncated = verses.len() > BIBLE_SEARCH_LIMIT;
        verses.truncate(BIBLE_SEARCH_LIMIT);
        Ok(BibleSearch { verses, truncated })
    }

    /// Install a translation.
    pub fn import_translation(&self, file: BibleImportFile) -> Result<BibleTranslation> {
        self.repo.import_translation(file)
    }

    /// Remove a translation.
    pub fn delete_translation(&self, id: &str) -> Result<()> {
        self.repo.delete_translation(id)
    }
}
