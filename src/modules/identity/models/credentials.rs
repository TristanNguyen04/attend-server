//! Validated sign-in input.

use super::errors;
use crate::{modules::identity::dtos::SignInRequest, shared_lib::error::ApiError};

/// An email and a password that are both present and not blank.
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
        let err = Credentials::try_from(request)
            .err()
            .expect("validation fails");
        assert_eq!(err.body().code, "validation_failed");
        err.body().details.clone().expect("details")
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
