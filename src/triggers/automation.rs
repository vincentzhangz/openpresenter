use super::Action;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroStep {
    pub delay_ms: u64,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub steps: Vec<MacroStep>,
    pub looping: bool,
}

impl Macro {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            steps: Vec::new(),
            looping: false,
        }
    }

    pub fn add_step(&mut self, delay_ms: u64, action: Action) {
        self.steps.push(MacroStep { delay_ms, action });
    }

    pub fn remove_step(&mut self, index: usize) {
        if index < self.steps.len() {
            self.steps.remove(index);
        }
    }

    pub fn spawn(&self, tx: tokio::sync::mpsc::Sender<Action>) -> tokio::task::JoinHandle<()> {
        let steps = self.steps.clone();
        let looping = self.looping;
        tokio::spawn(async move {
            loop {
                for step in &steps {
                    if step.delay_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(step.delay_ms)).await;
                    }
                    if tx.send(step.action.clone()).await.is_err() {
                        return;
                    }
                }
                if !looping {
                    break;
                }
            }
        })
    }
}

#[derive(Default)]
pub struct MacroManager {
    pub macros: Vec<Macro>,
    running: std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
}

impl MacroManager {
    pub fn add(&mut self, m: Macro) {
        self.macros.push(m);
    }

    pub fn remove(&mut self, id: &str) {
        self.stop(id);
        self.macros.retain(|m| m.id != id);
    }

    pub fn get(&self, id: &str) -> Option<&Macro> {
        self.macros.iter().find(|m| m.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Macro> {
        self.macros.iter_mut().find(|m| m.id == id)
    }

    pub fn run(&mut self, id: &str, tx: tokio::sync::mpsc::Sender<Action>) {
        self.stop(id);
        if let Some(m) = self.get(id) {
            let handle = m.spawn(tx);
            self.running.insert(id.to_string(), handle);
        }
    }

    pub fn stop(&mut self, id: &str) {
        if let Some(h) = self.running.remove(id) {
            h.abort();
        }
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::Action;

    #[test]
    fn macro_new_has_empty_steps_and_not_looping() {
        let m = Macro::new("Intro");
        assert_eq!(m.name, "Intro");
        assert!(m.steps.is_empty());
        assert!(!m.looping);
    }

    #[test]
    fn macro_add_step_appends() {
        let mut m = Macro::new("Test");
        m.add_step(500, Action::NextSlide);
        m.add_step(0, Action::ClearOutput);
        assert_eq!(m.steps.len(), 2);
        assert_eq!(m.steps[0].delay_ms, 500);
        assert_eq!(m.steps[1].delay_ms, 0);
    }

    #[test]
    fn macro_remove_step_by_index() {
        let mut m = Macro::new("Test");
        m.add_step(100, Action::NextSlide);
        m.add_step(200, Action::PrevSlide);
        m.remove_step(0);
        assert_eq!(m.steps.len(), 1);
        assert_eq!(m.steps[0].delay_ms, 200);
    }

    #[test]
    fn macro_remove_step_out_of_bounds_is_noop() {
        let mut m = Macro::new("Test");
        m.add_step(100, Action::NextSlide);
        m.remove_step(99);
        assert_eq!(m.steps.len(), 1);
    }

    #[test]
    fn macro_manager_add_get_remove() {
        let mut mgr = MacroManager::default();
        let m = Macro::new("Alpha");
        let id = m.id.clone();
        mgr.add(m);
        assert!(mgr.get(&id).is_some());
        assert_eq!(mgr.get(&id).unwrap().name, "Alpha");
        mgr.remove(&id);
        assert!(mgr.get(&id).is_none());
    }

    #[test]
    fn macro_manager_get_mut_allows_mutation() {
        let mut mgr = MacroManager::default();
        let m = Macro::new("Beta");
        let id = m.id.clone();
        mgr.add(m);
        mgr.get_mut(&id).unwrap().looping = true;
        assert!(mgr.get(&id).unwrap().looping);
    }

    #[test]
    fn macro_manager_is_not_running_by_default() {
        let mgr = MacroManager::default();
        assert!(!mgr.is_running("any-id"));
    }

    #[test]
    fn macro_manager_stop_unknown_id_is_noop() {
        let mut mgr = MacroManager::default();
        mgr.stop("does-not-exist");
    }

    #[test]
    fn macro_manager_remove_unknown_id_is_noop() {
        let mut mgr = MacroManager::default();
        mgr.remove("does-not-exist");
    }
}
