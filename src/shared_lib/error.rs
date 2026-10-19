//! The API error envelope: every error response has the body
//! `{ "code": "...", "message": "...", "details": ... }`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The JSON body of an error response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiErrorBody {
    /// A stable, machine-readable identifier, e.g. `session_not_open`.
    pub code: String,
    /// A human-readable explanation.
    pub message: String,
    /// Optional structured context (for example which field was invalid).
    #[ts(type = "unknown")]
    pub details: Option<serde_json::Value>,
}

/// An error together with the HTTP status it is returned with.
#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl ApiError {
    #[must_use]
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    #[must_use]
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    #[must_use]
    pub fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    #[must_use]
    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    #[must_use]
    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    #[must_use]
    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    #[must_use]
    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    /// Attach structured context.
    #[must_use]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.body.details = Some(details);
        self
    }

    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    #[must_use]
    pub fn body(&self) -> &ApiErrorBody {
        &self.body
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_envelope_with_status_code_and_message() {
        let err = ApiError::conflict("session_open", "A session is open");
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.body().code, "session_open");
        assert_eq!(err.body().message, "A session is open");
        assert!(err.body().details.is_none());
    }

    #[test]
    fn each_constructor_picks_its_http_status() {
        assert_eq!(
            ApiError::bad_request("c", "m").status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::forbidden("c", "m").status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ApiError::not_found("c", "m").status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(ApiError::conflict("c", "m").status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn the_response_carries_the_status_and_the_json_envelope() {
        let response = ApiError::forbidden("not_allowed", "No access").into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "not_allowed", "message": "No access", "details": null })
        );
    }

    #[test]
    fn unauthorized_is_a_401_with_the_given_code() {
        let err = ApiError::unauthorized("unauthenticated", "Sign in first");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.body().code, "unauthenticated");
        assert_eq!(err.body().message, "Sign in first");
    }

    #[test]
    fn internal_is_a_500_with_the_given_code() {
        let err = ApiError::internal("internal_error", "Something went wrong");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.body().code, "internal_error");
    }

    #[test]
    fn details_are_optional_context() {
        let err = ApiError::bad_request("invalid_field", "Bad value")
            .with_details(serde_json::json!({ "field": "starts_at" }));
        assert_eq!(
            err.body().details,
            Some(serde_json::json!({ "field": "starts_at" }))
        );
    }

    #[test]
    fn serialises_to_the_documented_json_shape() {
        let err = ApiError::not_found("not_found", "No such session");
        let json = serde_json::to_value(err.body()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "not_found", "message": "No such session", "details": null })
        );
    }
}
