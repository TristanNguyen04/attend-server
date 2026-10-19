//! Request and response types of this module's HTTP API.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::entities::users;

/// `POST /api/auth/sign-in` request body.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SignInRequest {
    /// Missing, empty and blank are all rejected with `validation_failed`.
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

/// `POST /api/auth/sign-in` response body.
#[derive(Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SignInResponse {
    pub access_token: String,
    /// Always `"Bearer"`.
    pub token_type: String,
    /// Seconds until the access token expires.
    #[ts(type = "number")]
    pub expires_in: u64,
}

impl SignInResponse {
    /// A response for a freshly issued access token.
    #[must_use]
    pub fn bearer(access_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

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

    use super::{ProfileResponse, SignInResponse};
    use crate::modules::identity::entities::users;

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

    #[test]
    fn the_response_has_exactly_the_documented_fields() {
        let response = SignInResponse::bearer("abc".to_string(), 86_400);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({ "access_token": "abc", "token_type": "Bearer", "expires_in": 86_400 })
        );
    }
}
