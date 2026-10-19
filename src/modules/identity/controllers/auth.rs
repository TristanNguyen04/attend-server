//! `POST /api/auth/sign-in`.

use axum::{
    extract::{rejection::JsonRejection, State},
    Json,
};
use loco_rs::prelude::*;

use crate::{
    modules::identity::{
        dtos::{SignInRequest, SignInResponse},
        models::{accounts, credentials::Credentials, errors},
    },
    shared_lib::error::ApiError,
};

pub(super) fn routes() -> Routes {
    Routes::new()
        .prefix("api/auth")
        .add("/sign-in", post(sign_in))
}

/// A body that is not JSON of the expected shape (or is absent) is a `validation_failed`, not
/// the framework's own error body.
fn body_rejected(_rejection: JsonRejection) -> ApiError {
    errors::validation_failed(&["body"])
}

async fn sign_in(
    State(ctx): State<AppContext>,
    body: std::result::Result<Json<SignInRequest>, JsonRejection>,
) -> std::result::Result<Json<SignInResponse>, ApiError> {
    let Json(request) = body.map_err(body_rejected)?;
    let credentials = Credentials::try_from(request)?;
    let clock = super::clock(&ctx)?;
    let jwt = super::jwt_config(&ctx)?;

    let signed_in = accounts::sign_in(
        &ctx.db,
        clock.now(),
        &credentials,
        &jwt.secret,
        jwt.expiration,
    )
    .await?;

    Ok(Json(SignInResponse::bearer(
        signed_in.access_token,
        signed_in.expires_in,
    )))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::modules::identity::controllers::test_support;

    #[test]
    fn a_body_that_is_not_json_is_a_validation_failure() {
        for body in ["", "{", "[1]", "{\"email\": 5}"] {
            let rejection = Json::<SignInRequest>::from_bytes(body.as_bytes())
                .err()
                .unwrap_or_else(|| panic!("{body:?} is not a sign-in request"));
            let err = body_rejected(rejection);
            assert_eq!(err.body().code, "validation_failed", "{body:?}");
            assert_eq!(err.body().details, Some(json!({ "fields": ["body"] })));
        }
    }

    #[tokio::test]
    async fn a_valid_request_reaches_the_database() {
        // The dummy database has no `identity` schema, so the lookup fails: a database failure
        // is an `internal_error`, never a raw error body.
        let (ctx, _) = test_support::context().await;
        let request = SignInRequest {
            email: Some("alex@example.edu".to_string()),
            password: Some("Correct#123".to_string()),
        };
        let err = sign_in(State(ctx), Ok(Json(request))).await.err().unwrap();
        assert_eq!(err.body().code, "internal_error");
    }

    #[tokio::test]
    async fn a_blank_password_is_refused_before_any_lookup() {
        let (ctx, _) = test_support::context().await;
        let request = SignInRequest {
            email: Some("alex@example.edu".to_string()),
            password: Some(" ".to_string()),
        };
        let err = sign_in(State(ctx), Ok(Json(request))).await.err().unwrap();
        assert_eq!(err.body().code, "validation_failed");
    }
}
