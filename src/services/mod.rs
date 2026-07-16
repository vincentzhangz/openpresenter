//! Business-logic layer.
//!
//! Services orchestrate repositories and domain types to perform use-cases.
//! They contain **no iced / UI code** and are unit-testable in isolation, which
//! is what makes the rest of the codebase scalable and maintainable: the UI
//! becomes a thin shell that calls into services and renders their results.
//!
//! See `AGENTS.md` for the full architecture and the migration path that moves
//! logic out of `src/ui/handlers` and into this module.

mod playlist;
mod presentation;

pub use playlist::PlaylistService;
pub use presentation::PresentationService;

use crate::db::Database;
use std::sync::Arc;

/// Aggregates every service behind a single handle so the UI holds one
/// dependency instead of a dozen repositories.
#[derive(Clone)]
pub struct Services {
    pub presentations: PresentationService,
    pub playlists: PlaylistService,
}

impl Services {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            presentations: PresentationService::new(db.clone()),
            playlists: PlaylistService::new(db.clone()),
        }
    }
}
