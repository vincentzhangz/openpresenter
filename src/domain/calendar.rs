//! The service calendar.
//!
//! A church's week is a list of services, and the list repeats: the same
//! service on the same evening, week after week, with the playlist that
//! plays at it. The calendar holds those services so the operator can see
//! what is coming and run it.

use std::fmt;

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A service on the calendar.
///
/// The event is the pattern, not one night of it: a service that repeats
/// every Sunday is one event, and the nights it happens on are its
/// occurrences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceEvent {
    pub id: String,
    /// The name the operator gave the service.
    pub name: String,
    /// What the service is: a note for the operator.
    #[serde(default)]
    pub description: String,
    /// When the service starts, as seconds since the epoch.
    pub starts_at: i64,
    /// How long the service runs, in minutes.
    #[serde(default = "default_duration")]
    pub duration_mins: u32,
    /// How often the service happens.
    #[serde(default)]
    pub repeat: Repeat,
    /// The playlist that plays at the service.
    #[serde(default)]
    pub playlist_id: Option<String>,
    /// Whether the service is recorded.
    #[serde(default)]
    pub capture: bool,
}

fn default_duration() -> u32 {
    60
}

impl Default for ServiceEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::from("Service"),
            description: String::new(),
            starts_at: 0,
            duration_mins: default_duration(),
            repeat: Repeat::Never,
            playlist_id: None,
            capture: false,
        }
    }
}

impl ServiceEvent {
    /// A service with the given name, starting at the given moment.
    pub fn new(name: impl Into<String>, starts_at: i64) -> Self {
        Self {
            name: name.into(),
            starts_at,
            ..Self::default()
        }
    }

    /// The time of day the service starts.
    ///
    /// A repeating service keeps its time: the service on Sunday evening
    /// starts at the same hour every Sunday.
    fn time_of_day(&self) -> (u32, u32, u32) {
        let moment = Utc
            .timestamp_opt(self.starts_at, 0)
            .single()
            .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
        (moment.hour(), moment.minute(), moment.second())
    }

    /// The day of the week the first service falls on.
    fn weekday(&self) -> Weekday {
        let moment = Utc
            .timestamp_opt(self.starts_at, 0)
            .single()
            .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
        Weekday::from(moment.weekday())
    }

    /// When the service next happens, at or after the given moment.
    ///
    /// A service that has already happened today happens today again only
    /// if it has not started yet: the operator is looking forwards.
    pub fn next_occurrence(&self, from: i64) -> Option<i64> {
        self.occurrences_between(from, from + SECONDS_PER_YEAR * 2)
            .into_iter()
            .next()
    }

    /// Every time the service happens between two moments.
    pub fn occurrences_between(&self, from: i64, to: i64) -> Vec<i64> {
        if to <= from {
            return Vec::new();
        }
        match self.repeat {
            Repeat::Never => {
                if self.starts_at >= from && self.starts_at < to {
                    vec![self.starts_at]
                } else {
                    Vec::new()
                }
            }
            Repeat::Weekly { ref days } => {
                if days.is_empty() {
                    return Vec::new();
                }
                let (hour, minute, second) = self.time_of_day();
                let mut out = Vec::new();
                // The walk starts on the day the first service fell on, so
                // the service keeps the day of the week it was made on.
                let mut day = self.weekday().days_from_monday();
                let mut cursor = from - (from.rem_euclid(SECONDS_PER_DAY));
                // A day before the moment the walk starts is not a
                // candidate, so the walk begins on the day itself.
                while cursor < to {
                    if cursor >= from {
                        let weekday = Weekday::from_days_from_monday(day);
                        if days.contains(&weekday) {
                            let candidate = cursor
                                + (hour as i64 * 3600)
                                + (minute as i64 * 60)
                                + second as i64;
                            if candidate >= from && candidate < to {
                                out.push(candidate);
                            }
                        }
                    }
                    cursor += SECONDS_PER_DAY;
                    day = (day + 1) % 7;
                }
                out
            }
        }
    }

    /// The moment the service ends.
    pub fn ends_at(&self, starts_at: i64) -> i64 {
        starts_at + i64::from(self.duration_mins) * 60
    }
}

/// How often a service happens.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Repeat {
    /// Once.
    #[default]
    Never,
    /// Every week, on these days.
    Weekly { days: Vec<Weekday> },
}

impl Repeat {
    /// The name the repeat is shown under.
    pub fn label(&self) -> String {
        match self {
            Self::Never => String::from("Does not repeat"),
            Self::Weekly { days } => {
                if days.is_empty() {
                    return String::from("Every week");
                }
                let names: Vec<&str> = days.iter().map(|day| day.short_name()).collect();
                format!("Every {}", names.join(", "))
            }
        }
    }
}

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    /// Every day, from Monday.
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    /// The day's full name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }

    /// The day's short name.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
            Self::Sunday => "Sun",
        }
    }

    /// The day from the day the given moment falls on.
    pub fn from(weekday: chrono::Weekday) -> Self {
        match weekday {
            chrono::Weekday::Mon => Self::Monday,
            chrono::Weekday::Tue => Self::Tuesday,
            chrono::Weekday::Wed => Self::Wednesday,
            chrono::Weekday::Thu => Self::Thursday,
            chrono::Weekday::Fri => Self::Friday,
            chrono::Weekday::Sat => Self::Saturday,
            chrono::Weekday::Sun => Self::Sunday,
        }
    }

    /// The day from the number of days from Monday.
    pub fn from_days_from_monday(days: usize) -> Self {
        Self::ALL[days % 7]
    }

    /// The number of days from Monday.
    pub fn days_from_monday(&self) -> usize {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// The seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// The seconds in a year.
///
/// The year is used for the walk: a service that repeats every week is
/// looked for a year ahead, which is long enough for the next occurrence
/// and short enough to finish.
const SECONDS_PER_YEAR: i64 = 365 * SECONDS_PER_DAY;

/// The moment a service happens, in the operator's own time.
pub fn local_time(epoch: i64) -> DateTime<Local> {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .unwrap_or_else(|| Local.timestamp_opt(0, 0).unwrap())
}

/// The moment now.
pub fn now() -> i64 {
    Utc::now().timestamp()
}

/// How long is left until the service, in words.
///
/// "in 3 days" reads better than a date when the operator is looking at
/// what is coming.
pub fn time_until(epoch: i64) -> String {
    let away = epoch - now();
    if away <= 0 {
        return String::from("now");
    }
    let days = away / SECONDS_PER_DAY;
    if days >= 7 {
        let weeks = days / 7;
        return if weeks == 1 {
            String::from("in 1 week")
        } else {
            format!("in {weeks} weeks")
        };
    }
    if days >= 1 {
        return if days == 1 {
            String::from("tomorrow")
        } else {
            format!("in {days} days")
        };
    }
    let hours = away / 3600;
    if hours >= 1 {
        return if hours == 1 {
            String::from("in 1 hour")
        } else {
            format!("in {hours} hours")
        };
    }
    let minutes = (away % 3600) / 60;
    if minutes == 0 {
        String::from("in under a minute")
    } else if minutes == 1 {
        String::from("in 1 minute")
    } else {
        format!("in {minutes} minutes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A service that happens once happens once.
    #[test]
    fn a_service_that_does_not_repeat_happens_once() {
        let event = ServiceEvent::new("Service", 1_700_000_000);
        assert_eq!(
            event.occurrences_between(1_699_000_000, 1_701_000_000),
            vec![1_700_000_000]
        );
        assert!(
            event
                .occurrences_between(1_700_000_001, 1_701_000_000)
                .is_empty()
        );
    }

    /// A service that repeats every Sunday happens every Sunday.
    #[test]
    fn a_service_that_repeats_happens_every_week() {
        let mut event = ServiceEvent::new("Service", 1_700_000_000);
        event.repeat = Repeat::Weekly {
            days: vec![Weekday::Sunday],
        };
        let from = 1_700_000_000;
        let occurrences = event.occurrences_between(from, from + 28 * SECONDS_PER_DAY);
        assert_eq!(occurrences.len(), 4);
        // Every occurrence is a week apart.
        for pair in occurrences.windows(2) {
            assert_eq!(pair[1] - pair[0], 7 * SECONDS_PER_DAY);
        }
    }

    /// A service keeps the time of day it was made at.
    #[test]
    fn a_repeating_service_keeps_its_time() {
        let mut event = ServiceEvent::new("Service", 1_700_000_000);
        event.repeat = Repeat::Weekly {
            days: vec![Weekday::Sunday],
        };
        let first = 1_700_000_000;
        // Two Sundays apart: a fortnight holds two of them.
        let occurrences = event.occurrences_between(first, first + 14 * SECONDS_PER_DAY);
        let second = occurrences[1];
        let time_of_day =
            |epoch: i64| Utc.timestamp_opt(epoch, 0).unwrap().timestamp() % SECONDS_PER_DAY;
        assert_eq!(time_of_day(first), time_of_day(second));
    }

    /// The next service is the one that is coming.
    #[test]
    fn the_next_service_is_the_one_that_is_coming() {
        let mut event = ServiceEvent::new("Service", 1_700_000_000);
        event.repeat = Repeat::Weekly {
            days: vec![Weekday::Sunday],
        };
        let next = event
            .next_occurrence(1_700_000_000)
            .expect("there is a next one");
        assert!(next >= 1_700_000_000);
        assert!(next - 1_700_000_000 <= 7 * SECONDS_PER_DAY);
    }
}
