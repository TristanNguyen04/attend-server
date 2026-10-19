//! Request and response shapes of the account endpoints.

use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::modules::identity::entities::users;

/// `GET /api/me` response body: the caller's own profile. Never a password or a hash.
#[derive(Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProfileResponse {
    #[ts(type = "string")]
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub status: String,
}

impl From<users::Model> for ProfileResponse {
    fn from(account: users::Model) -> Self {
        Self {
            id: account.id,
            email: account.email,
            full_name: account.full_name,
            status: account.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn the_profile_has_exactly_four_fields_and_never_the_hash() {
        let id = Uuid::new_v4();
        let now = Utc::now().fixed_offset();
        let account = users::Model {
            id,
            email: "alex@example.edu".to_string(),
            full_name: "Alex Tan".to_string(),
            password_hash: "$argon2id$secret".to_string(),
            status: "ACTIVE".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(
            serde_json::to_value(ProfileResponse::from(account)).unwrap(),
            json!({
                "id": id.to_string(),
                "email": "alex@example.edu",
                "full_name": "Alex Tan",
                "status": "ACTIVE",
            })
        );
    }
}
