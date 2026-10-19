//! An injectable clock. Anything that depends on "now" (session state, check-in windows,
//! appeal deadlines) takes a [`Clock`] so tests can control time instead of sleeping.

use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, Mutex};

/// A source of the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The real clock, used in production.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that only moves when told to. Use it in tests.
#[derive(Debug, Clone)]
pub struct FixedClock {
    now: Arc<Mutex<DateTime<Utc>>>,
}

impl FixedClock {
    #[must_use]
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Arc::new(Mutex::new(now)),
        }
    }

    /// Jump to an exact instant.
    ///
    /// # Panics
    /// Panics if another thread panicked while holding the clock's lock.
    pub fn set(&self, now: DateTime<Utc>) {
        *self.now.lock().expect("clock lock poisoned") = now;
    }

    /// Move forward (or backward, with a negative duration).
    ///
    /// # Panics
    /// Panics if another thread panicked while holding the clock's lock.
    pub fn advance(&self, by: Duration) {
        let mut now = self.now.lock().expect("clock lock poisoned");
        *now += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, hour, minute, 0).unwrap()
    }

    #[test]
    fn system_clock_reads_the_real_time() {
        let before = Utc::now();
        let now = SystemClock.now();
        let after = Utc::now();
        assert!(before <= now && now <= after);
    }

    #[test]
    fn fixed_clock_returns_the_time_it_was_given() {
        let clock = FixedClock::new(at(10, 0));
        assert_eq!(clock.now(), at(10, 0));
    }

    #[test]
    fn fixed_clock_moves_only_when_told_to() {
        let clock = FixedClock::new(at(10, 0));
        clock.advance(Duration::minutes(15));
        assert_eq!(clock.now(), at(10, 15));
        clock.set(at(12, 0));
        assert_eq!(clock.now(), at(12, 0));
    }

    #[test]
    fn clones_share_the_same_time() {
        let clock = FixedClock::new(at(10, 0));
        let other = clock.clone();
        clock.advance(Duration::minutes(5));
        assert_eq!(other.now(), at(10, 5));
    }
}
