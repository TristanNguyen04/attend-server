//! Request and response shapes of the sign-in endpoint, and the validation of the request.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{modules::identity::errors, shared_lib::error::ApiError};

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

/// An email and a password that are both present and not blank.
#[derive(Debug)]
pub(crate) struct Credentials {
    pub email: String,
    pub password: String,
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

impl TryFrom<SignInRequest> for Credentials {
    type Error = ApiError;

    /// # Errors
    /// `validation_failed`, naming every field that is missing, empty or only whitespace (FR6).
    fn try_from(request: SignInRequest) -> Result<Self, ApiError> {
        let offending: Vec<&str> = [
            ("email", request.email.as_deref()),
            ("password", request.password.as_deref()),
        ]
        .into_iter()
        .filter(|(_, value)| is_blank(*value))
        .map(|(name, _)| name)
        .collect();
        if !offending.is_empty() {
            return Err(errors::validation_failed(&offending));
        }
        Ok(Self {
            email: request.email.unwrap_or_default(),
            password: request.password.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request(email: Option<&str>, password: Option<&str>) -> SignInRequest {
        SignInRequest {
            email: email.map(str::to_string),
            password: password.map(str::to_string),
        }
    }

    fn offending(request: SignInRequest) -> serde_json::Value {
        let err = Credentials::try_from(request).unwrap_err();
        assert_eq!(err.body().code, "validation_failed");
        err.body().details.clone().expect("details")
    }

    #[test]
    fn the_response_has_exactly_the_documented_fields() {
        let response = SignInResponse::bearer("abc".to_string(), 86_400);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({ "access_token": "abc", "token_type": "Bearer", "expires_in": 86_400 })
        );
    }

    #[test]
    fn accepts_an_email_and_a_password() {
        let credentials = Credentials::try_from(request(Some("a@b.c"), Some(" p "))).unwrap();
        assert_eq!(credentials.email, "a@b.c");
        assert_eq!(credentials.password, " p ", "the password is not trimmed");
    }

    #[test]
    fn a_missing_field_is_named() {
        assert_eq!(
            offending(request(None, Some("p"))),
            json!({ "fields": ["email"] })
        );
        assert_eq!(
            offending(request(Some("a@b.c"), None)),
            json!({ "fields": ["password"] })
        );
    }

    #[test]
    fn an_empty_or_whitespace_field_is_named() {
        assert_eq!(
            offending(request(Some(""), Some("p"))),
            json!({ "fields": ["email"] })
        );
        assert_eq!(
            offending(request(Some(" \t"), Some("p"))),
            json!({ "fields": ["email"] })
        );
        assert_eq!(
            offending(request(Some("a@b.c"), Some(""))),
            json!({ "fields": ["password"] })
        );
        assert_eq!(
            offending(request(Some("a@b.c"), Some("  "))),
            json!({ "fields": ["password"] })
        );
    }

    #[test]
    fn every_offending_field_is_named() {
        assert_eq!(
            offending(request(None, Some(" "))),
            json!({ "fields": ["email", "password"] })
        );
    }
}
