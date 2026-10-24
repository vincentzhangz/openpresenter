use crate::{Result, db::Database};

pub mod log;

pub struct App {
    db: Database,
}

impl App {
    pub fn new() -> Result<Self> {
        let db = Database::new()?;
        Ok(Self { db })
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn into_db(self) -> Database {
        self.db
    }
}
