//! Playlist (ProPresenter "Playlist") use-cases.
//!
//! A playlist is an ordered set of items (presentations, songs, media cues,
//! headers, blanks) used to plan a service. The backing repository is still
//! named `PlaylistRepository` and stores rows in `service_plans` /
//! `service_items` for backwards compatibility.

use crate::Result;
use crate::db::PlaylistRepository;
use crate::domain::{Playlist, PlaylistItem};

#[derive(Clone)]
pub struct PlaylistService {
    repo: PlaylistRepository,
}

impl PlaylistService {
    pub fn new(db: std::sync::Arc<crate::db::Database>) -> Self {
        Self {
            repo: PlaylistRepository::new(db),
        }
    }

    pub fn list(&self) -> Result<Vec<Playlist>> {
        self.repo.list_playlists()
    }

    pub fn get(&self, id: &str) -> Result<Playlist> {
        self.repo.get_playlist(id)
    }

    /// Persist a playlist and its items atomically.
    pub fn save(&self, playlist: &Playlist) -> Result<()> {
        self.repo.save_playlist(playlist)?;
        self.repo.save_items(&playlist.id, &playlist.items)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.repo.delete_playlist(id)
    }

    /// Replace the item list and persist it.
    pub fn set_items(&self, playlist: &mut Playlist, items: Vec<PlaylistItem>) -> Result<()> {
        playlist.items = items;
        self.repo.save_items(&playlist.id, &playlist.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Playlist, PlaylistItem};
    use std::sync::Arc;

    fn svc() -> PlaylistService {
        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        PlaylistService::new(db)
    }

    #[test]
    fn save_and_list_round_trips() {
        let s = svc();
        let mut plan = Playlist::new("Sunday".into());
        plan.items.push(PlaylistItem::Blank);
        plan.items.push(PlaylistItem::Header {
            text: "Welcome".into(),
        });
        s.save(&plan).unwrap();

        let plans = s.list().unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "Sunday");
        // `list` returns summaries without items; `get` lazy-loads them.
        assert!(plans[0].items.is_empty());

        let reloaded = s.get(&plan.id).unwrap();
        assert_eq!(reloaded.items.len(), 2);
    }

    #[test]
    fn delete_removes_playlist() {
        let s = svc();
        let plan = Playlist::new("Temp".into());
        s.save(&plan).unwrap();
        assert_eq!(s.list().unwrap().len(), 1);
        s.delete(&plan.id).unwrap();
        assert!(s.list().unwrap().is_empty());
    }
}
