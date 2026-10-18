//! Event contracts shared between modules.
//!
//! This holds the envelope every event travels in. The outbox, the in-process bus and the
//! dispatcher are not built yet: they are the first infrastructure task, to be specified
//! and implemented with the first feature that publishes an event.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The wrapper around every event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    /// Unique per event. Consumers use it to ignore duplicates.
    pub id: Uuid,
    /// The event name, for example `ScanAccepted`.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Schema version of `data`. Only additive changes are allowed within a version.
    pub version: u32,
    pub occurred_at: DateTime<Utc>,
    /// Ties the event back to the request that caused it.
    pub correlation_id: String,
    /// The module that published the event.
    pub producer: String,
    pub data: T,
}

impl<T> Envelope<T> {
    #[must_use]
    pub fn new(
        event_type: impl Into<String>,
        version: u32,
        occurred_at: DateTime<Utc>,
        correlation_id: impl Into<String>,
        producer: impl Into<String>,
        data: T,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            version,
            occurred_at,
            correlation_id: correlation_id.into(),
            producer: producer.into(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn serialises_the_event_type_under_the_key_type() {
        let at = Utc.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let envelope = Envelope::new(
            "ScanAccepted",
            1,
            at,
            "req-1",
            "capture",
            serde_json::json!({}),
        );
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(json["type"], "ScanAccepted");
        assert_eq!(json["version"], 1);
        assert_eq!(json["producer"], "capture");
    }

    #[test]
    fn every_event_gets_its_own_id() {
        let at = Utc.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let a = Envelope::new("X", 1, at, "c", "p", ());
        let b = Envelope::new("X", 1, at, "c", "p", ());
        assert_ne!(a.id, b.id);
    }
}
