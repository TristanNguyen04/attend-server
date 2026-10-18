//! The typed shape of `courses.policies` (a jsonb column).
//!
//! `courses` stores and serves the value; `records` interprets it. The struct lives here so
//! both sides compile against one contract. Validation rules are specified per feature in
//! `specs/`, not invented here.

use serde::{Deserialize, Serialize};

/// Per-course attendance policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoursePolicies {
    pub late_after_minutes: u32,
    pub absent_after_minutes: u32,
    pub require_checkout: bool,
    /// Only meaningful when `require_checkout` is true.
    #[serde(default)]
    pub earliest_checkout_after_minutes: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_agreed_json_shape() {
        let json = serde_json::json!({
            "late_after_minutes": 10,
            "absent_after_minutes": 30,
            "require_checkout": true,
            "earliest_checkout_after_minutes": 45
        });
        let policies: CoursePolicies = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(policies.late_after_minutes, 10);
        assert_eq!(policies.earliest_checkout_after_minutes, Some(45));
        assert_eq!(serde_json::to_value(&policies).unwrap(), json);
    }

    #[test]
    fn earliest_checkout_is_optional() {
        let json = serde_json::json!({
            "late_after_minutes": 10,
            "absent_after_minutes": 30,
            "require_checkout": false
        });
        let policies: CoursePolicies = serde_json::from_value(json).unwrap();
        assert_eq!(policies.earliest_checkout_after_minutes, None);
    }
}
