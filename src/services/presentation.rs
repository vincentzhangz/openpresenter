//! Presentation use-cases.
//!
//! Operations that mutate a [`Presentation`](crate::domain::Presentation) and
//! its slides. Every method reloads and returns the persisted presentation so
//! the caller (UI or test) always holds authoritative state.

use crate::Result;
use crate::db::PresentationRepository;
use crate::domain::{Presentation, Slide};

/// Move `slide_id` to `target_index` within `ids`, returning the new ordering.
///
/// Pure and side-effect free so it can be unit-tested without a database.
fn place_id_at(ids: &[String], slide_id: &str, target_index: usize) -> Vec<String> {
    let mut ids = ids.to_vec();
    if let Some(from) = ids.iter().position(|id| id == slide_id) {
        let moved = ids.remove(from);
        let target = target_index.min(ids.len());
        ids.insert(target, moved);
    }
    ids
}

#[derive(Clone)]
pub struct PresentationService {
    repo: PresentationRepository,
}

impl PresentationService {
    pub fn new(db: std::sync::Arc<crate::db::Database>) -> Self {
        Self {
            repo: PresentationRepository::new(db),
        }
    }

    pub fn repo(&self) -> &PresentationRepository {
        &self.repo
    }

    pub fn create(&self, name: &str) -> Result<Presentation> {
        self.repo.create_presentation(name)
    }

    pub fn list(&self) -> Result<Vec<Presentation>> {
        self.repo.list_presentations()
    }

    pub fn get(&self, id: &str) -> Result<Presentation> {
        self.repo.get_presentation(id)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        self.repo.update_presentation(id, name)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.repo.delete_presentation(id)
    }

    /// Append a new empty text slide and return the reloaded presentation.
    pub fn add_slide(&self, presentation_id: &str) -> Result<Presentation> {
        let pres = self.repo.get_presentation(presentation_id)?;
        let order = pres.slides.len() as i32;
        let slide = Slide::new_text(String::new());
        self.repo.add_slide(presentation_id, &slide, order)?;
        self.repo.get_presentation(presentation_id)
    }

    /// Insert a new slide directly after `after_index`, inheriting the source
    /// slide's group, then normalise the ordering. Returns the reloaded
    /// presentation and the index of the new slide.
    pub fn add_slide_after(
        &self,
        presentation_id: &str,
        after_index: usize,
    ) -> Result<(Presentation, usize)> {
        let pres = self.repo.get_presentation(presentation_id)?;
        let group = pres.slides.get(after_index).and_then(|s| s.group.clone());
        let mut slide = Slide::new_text(String::new());
        slide.group = group;
        let insert_order = (after_index + 1) as i32;
        self.repo.add_slide(presentation_id, &slide, insert_order)?;

        let updated = self.repo.get_presentation(presentation_id)?;
        let ids: Vec<String> = updated.slides.iter().map(|s| s.id.clone()).collect();
        let placed = place_id_at(&ids, &slide.id, after_index + 1);
        if placed != ids {
            let _ = self.repo.reorder_slides(presentation_id, &placed);
        }
        let reloaded = self.repo.get_presentation(presentation_id)?;
        let new_index = (after_index + 1).min(reloaded.slides.len().saturating_sub(1));
        Ok((reloaded, new_index))
    }

    /// Duplicate the slide at `index` immediately after it.
    pub fn duplicate_slide(
        &self,
        presentation_id: &str,
        index: usize,
    ) -> Result<(Presentation, usize)> {
        let pres = self.repo.get_presentation(presentation_id)?;
        let pres_id = pres.id.clone();
        if let Some(source) = pres.slides.get(index) {
            let mut dup = source.clone();
            dup.id = uuid::Uuid::new_v4().to_string();
            let order = (index + 1) as i32;
            self.repo.add_slide(&pres_id, &dup, order)?;

            let updated = self.repo.get_presentation(&pres_id)?;
            let ids: Vec<String> = updated.slides.iter().map(|s| s.id.clone()).collect();
            let placed = place_id_at(&ids, &dup.id, index + 1);
            if placed != ids {
                let _ = self.repo.reorder_slides(&pres_id, &placed);
            }
            let reloaded = self.repo.get_presentation(&pres_id)?;
            let new_index = (index + 1).min(reloaded.slides.len().saturating_sub(1));
            return Ok((reloaded, new_index));
        }
        self.repo.get_presentation(&pres_id).map(|p| (p, index))
    }

    pub fn delete_slide(&self, presentation_id: &str, slide_id: &str) -> Result<Presentation> {
        self.repo.delete_slide(slide_id)?;
        self.repo.get_presentation(presentation_id)
    }

    /// Persist an edited slide, returning the reloaded presentation.
    pub fn update_slide(&self, presentation_id: &str, slide: &Slide) -> Result<Presentation> {
        self.repo.update_slide(slide)?;
        self.repo.get_presentation(presentation_id)
    }

    /// Persist a new ordering for the slides of `presentation_id`.
    pub fn reorder_slides(&self, presentation_id: &str, slide_ids: &[String]) -> Result<()> {
        self.repo.reorder_slides(presentation_id, slide_ids)
    }

    /// Replace every slide of `presentation_id` with `slides`.
    pub fn replace_slides(&self, presentation_id: &str, slides: &[Slide]) -> Result<Presentation> {
        self.repo
            .replace_presentation_slides(presentation_id, slides)?;
        self.repo.get_presentation(presentation_id)
    }

    /// Reorder a slide to a new position, returning the reloaded presentation.
    pub fn move_slide(
        &self,
        presentation_id: &str,
        slide_id: &str,
        target_index: usize,
    ) -> Result<Presentation> {
        let pres = self.repo.get_presentation(presentation_id)?;
        let ids: Vec<String> = pres.slides.iter().map(|s| s.id.clone()).collect();
        let placed = place_id_at(&ids, slide_id, target_index);
        self.repo.reorder_slides(presentation_id, &placed)?;
        self.repo.get_presentation(presentation_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn svc() -> PresentationService {
        let db = Arc::new(crate::db::Database::in_memory().unwrap());
        PresentationService::new(db)
    }

    #[test]
    fn place_id_at_moves_item() {
        let ids = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        assert_eq!(place_id_at(&ids, "a", 2), vec!["b", "c", "a", "d"]);
    }

    #[test]
    fn place_id_at_clamps_target() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(place_id_at(&ids, "a", 99), vec!["b", "c", "a"]);
    }

    #[test]
    fn create_and_add_slides() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        assert_eq!(pres.slides.len(), 1);
        let (pres, idx) = s.add_slide_after(&pres.id, 0).unwrap();
        assert_eq!(pres.slides.len(), 2);
        assert_eq!(idx, 1);
        // New slide inherits the group of the one it was inserted after.
        assert_eq!(pres.slides[1].group, pres.slides[0].group);
    }

    #[test]
    fn duplicate_inserts_after_source() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let (pres, idx) = s.duplicate_slide(&pres.id, 0).unwrap();
        assert_eq!(pres.slides.len(), 2);
        assert_eq!(idx, 1);
        assert_ne!(pres.slides[0].id, pres.slides[1].id);
    }

    #[test]
    fn delete_removes_slide() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let id = pres.slides[0].id.clone();
        let pres = s.delete_slide(&pres.id, &id).unwrap();
        assert!(pres.slides.is_empty());
    }

    #[test]
    fn move_slide_reorders() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let first_id = pres.slides[0].id.clone();
        let pres = s.move_slide(&pres.id, &first_id, 2).unwrap();
        assert_eq!(pres.slides[2].id, first_id);
    }

    #[test]
    fn update_slide_persists_changes() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let mut slide = pres.slides[0].clone();
        slide.notes = Some("hello".into());
        let pres = s.update_slide(&pres.id, &slide).unwrap();
        assert_eq!(pres.slides[0].notes.as_deref(), Some("hello"));
    }

    #[test]
    fn reorder_slides_reorders() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let ids: Vec<String> = pres.slides.iter().map(|sl| sl.id.clone()).collect();
        let reordered = vec![ids[1].clone(), ids[0].clone()];
        s.reorder_slides(&pres.id, &reordered).unwrap();
        let pres = s.get(&pres.id).unwrap();
        assert_eq!(pres.slides[0].id, reordered[0]);
    }

    #[test]
    fn replace_slides_swaps_content() {
        let s = svc();
        let pres = s.create("Service").unwrap();
        let pres = s.add_slide(&pres.id).unwrap();
        let mut slides = pres.slides.clone();
        slides[0].notes = Some("note".into());
        let pres = s.replace_slides(&pres.id, &slides).unwrap();
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(pres.slides[0].notes.as_deref(), Some("note"));
    }
}
