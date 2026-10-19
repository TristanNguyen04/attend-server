//! The errors of the identity API, as `ApiError`s with their stable codes and messages.

use std::fmt::Display;

use serde_json::json;

use crate::shared_lib::error::ApiError;

/// Wrong password or unknown email. The two must look the same (FR3, FR4).
pub(crate) fn invalid_credentials() -> ApiError {
    ApiError::unauthorized("invalid_credentials", "Invalid email or password")
}

/// The account is `INVITED` or `SUSPENDED` (FR7, FR13).
pub(crate) fn account_not_active() -> ApiError {
    ApiError::forbidden("account_not_active", "This account is not active")
}

/// A protected endpoint was called without an access token (FR10).
pub(crate) fn unauthenticated() -> ApiError {
    ApiError::unauthorized("unauthenticated", "Sign in to use this endpoint")
}

/// The access token is malformed, altered or signed with another secret (FR11).
pub(crate) fn invalid_token() -> ApiError {
    ApiError::unauthorized("invalid_token", "The access token is not valid")
}

/// The access token is past its expiry (FR12).
pub(crate) fn token_expired() -> ApiError {
    ApiError::unauthorized("token_expired", "The access token has expired")
}

/// The request is missing or blanks `fields`, or is not the expected JSON (FR6).
/// `details` is `{ "fields": [...] }`.
pub(crate) fn validation_failed(fields: &[&str]) -> ApiError {
    ApiError::bad_request("validation_failed", "The request is not valid")
        .with_details(json!({ "fields": fields }))
}

/// Something went wrong on our side. The cause is logged, never sent to the client.
pub(crate) fn internal(cause: impl Display) -> ApiError {
    tracing::error!(%cause, "identity request failed");
    ApiError::internal("internal_error", "Something went wrong")
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::json;

    use super::*;

    fn code_and_status(err: &ApiError) -> (StatusCode, &str) {
        (err.status(), err.body().code.as_str())
    }

    #[test]
    fn each_error_has_its_status_and_code() {
        assert_eq!(
            code_and_status(&invalid_credentials()),
            (StatusCode::UNAUTHORIZED, "invalid_credentials")
        );
        assert_eq!(
            code_and_status(&account_not_active()),
            (StatusCode::FORBIDDEN, "account_not_active")
        );
        assert_eq!(
            code_and_status(&unauthenticated()),
            (StatusCode::UNAUTHORIZED, "unauthenticated")
        );
        assert_eq!(
            code_and_status(&invalid_token()),
            (StatusCode::UNAUTHORIZED, "invalid_token")
        );
        assert_eq!(
            code_and_status(&token_expired()),
            (StatusCode::UNAUTHORIZED, "token_expired")
        );
        assert_eq!(
            code_and_status(&validation_failed(&[])),
            (StatusCode::BAD_REQUEST, "validation_failed")
        );
        assert_eq!(
            code_and_status(&internal("boom")),
            (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
        );
    }

    #[test]
    fn invalid_credentials_has_the_exact_message() {
        assert_eq!(
            invalid_credentials().body().message,
            "Invalid email or password"
        );
    }

    #[test]
    fn validation_failed_names_the_fields() {
        assert_eq!(
            validation_failed(&["email", "password"]).body().details,
            Some(json!({ "fields": ["email", "password"] }))
        );
    }

    #[test]
    fn an_internal_error_does_not_leak_its_cause() {
        let err = internal("connection to db.internal:5432 refused");
        assert!(!err.body().message.contains("db.internal"));
        assert!(err.body().details.is_none());
    }
}
