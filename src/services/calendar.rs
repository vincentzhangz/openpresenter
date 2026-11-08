//! Service calendar use-cases.
//!
//! The calendar is the church's week: the services that happen, and the
//! playlist that plays at each one. The events are the pattern — a service
//! that repeats every Sunday is one event — and the nights it happens on are
//! its occurrences.

use crate::Result;
use crate::db::CalendarRepository;
use crate::domain::calendar::{Repeat, ServiceEvent, Weekday};

#[derive(Clone)]
pub struct CalendarService {
    repo: CalendarRepository,
}

impl CalendarService {
    pub fn new(db: std::sync::Arc<crate::db::Database>) -> Self {
        Self {
            repo: CalendarRepository::new(db),
        }
    }

    /// Every service on the calendar, soonest first.
    pub fn list(&self) -> Result<Vec<ServiceEvent>> {
        self.repo.list_events()
    }

    /// Save a service.
    pub fn save(&self, event: &ServiceEvent) -> Result<()> {
        self.repo.save_event(event)
    }

    /// Take a service off the calendar.
    pub fn delete(&self, id: &str) -> Result<()> {
        self.repo.delete_event(id)
    }

    /// Every time a service happens between two moments, with the service
    /// that happens.
    ///
    /// The operator looks at what is coming, so the list is of occurrences
    /// rather than of patterns: a service that repeats every week is on the
    /// list once for each week it happens.
    pub fn occurrences(&self, from: i64, to: i64) -> Result<Vec<(ServiceEvent, i64)>> {
        let mut out = Vec::new();
        for event in self.list()? {
            for at in event.occurrences_between(from, to) {
                out.push((event.clone(), at));
            }
        }
        out.sort_by_key(|(_, at)| *at);
        Ok(out)
    }
}

/// The service the operator is building.
pub struct NewEvent {
    /// The name the operator gave the service.
    pub name: String,
    /// What the service is.
    pub description: String,
    /// When the service starts, as seconds since the epoch.
    pub starts_at: i64,
    /// How long the service runs, in minutes.
    pub duration_mins: u32,
    /// How often the service happens.
    pub repeat: Repeat,
    /// The playlist that plays at the service.
    pub playlist_id: Option<String>,
    /// Whether the service is recorded.
    pub capture: bool,
}

impl NewEvent {
    /// The service the operator is building, as an event.
    pub fn into_event(self) -> ServiceEvent {
        ServiceEvent {
            name: self.name,
            description: self.description,
            starts_at: self.starts_at,
            duration_mins: self.duration_mins,
            repeat: self.repeat,
            playlist_id: self.playlist_id,
            capture: self.capture,
            ..ServiceEvent::default()
        }
    }
}

/// The days of the week a service repeats on.
pub fn days_from(flags: [bool; 7]) -> Vec<Weekday> {
    Weekday::ALL
        .into_iter()
        .enumerate()
        .filter(|(index, _)| flags[*index])
        .map(|(_, day)| day)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc() -> CalendarService {
        let db = std::sync::Arc::new(crate::db::Database::in_memory().unwrap());
        CalendarService::new(db)
    }

    /// A service survives the round trip.
    ///
    /// The calendar is the church's week, so a service that is forgotten
    /// between services is a week of services that has to be typed again.
    #[test]
    fn a_service_survives_the_round_trip() {
        let s = svc();
        let mut event = ServiceEvent::new("Sunday Service", 1_700_000_000);
        event.duration_mins = 75;
        event.repeat = Repeat::Weekly {
            days: vec![Weekday::Sunday],
        };
        event.capture = true;
        s.save(&event).unwrap();

        let events = s.list().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "Sunday Service");
        assert_eq!(events[0].duration_mins, 75);
        assert!(events[0].capture);
        assert_eq!(events[0].repeat.label(), "Every Sun");
    }

    /// A repeating service is on the list once for each time it happens.
    #[test]
    fn a_repeating_service_is_listed_once_for_each_time_it_happens() {
        let s = svc();
        let mut event = ServiceEvent::new("Sunday Service", 1_700_000_000);
        event.repeat = Repeat::Weekly {
            days: vec![Weekday::Sunday],
        };
        s.save(&event).unwrap();

        let from = 1_700_000_000;
        let occurrences = s.occurrences(from, from + 28 * 86_400).unwrap();
        assert_eq!(occurrences.len(), 4);
        // The list is in the order the services happen.
        for pair in occurrences.windows(2) {
            assert!(pair[0].1 < pair[1].1);
        }
    }

    /// A service can be taken off the calendar.
    #[test]
    fn a_service_can_be_removed() {
        let s = svc();
        let event = ServiceEvent::new("Service", 1_700_000_000);
        s.save(&event).unwrap();
        s.delete(&event.id).unwrap();
        assert!(s.list().unwrap().is_empty());
    }
}
