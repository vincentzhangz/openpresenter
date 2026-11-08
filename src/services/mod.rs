//! Business-logic layer.
//!
//! Services orchestrate repositories and domain types to perform use-cases.
//! They contain **no iced / UI code** and are unit-testable in isolation, which
//! keeps the domain logic decoupled and maintainable: the UI calls into services
//! and renders authoritative application state.

mod bible;
mod calendar;
mod playlist;
mod presentation;
mod song;

pub use bible::BibleService;
pub use calendar::CalendarService;
pub use playlist::PlaylistService;
pub use presentation::PresentationService;
pub use song::SongService;

use crate::db::Database;
use std::sync::Arc;

/// Aggregates every service behind a single handle so the UI holds one
/// dependency instead of a dozen repositories.
#[derive(Clone)]
pub struct Services {
    pub presentations: PresentationService,
    pub playlists: PlaylistService,
    pub bible: BibleService,
    pub songs: SongService,
    pub calendar: CalendarService,
}

impl Services {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            presentations: PresentationService::new(db.clone()),
            playlists: PlaylistService::new(db.clone()),
            bible: BibleService::new(db.clone()),
            songs: SongService::new(db.clone()),
            calendar: CalendarService::new(db),
        }
    }
}
