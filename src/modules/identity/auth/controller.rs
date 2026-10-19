//! `POST /api/auth/sign-in`.

use axum::{
    extract::{rejection::JsonRejection, State},
    Json,
};
use loco_rs::prelude::*;

use super::{
    dto::{Credentials, SignInRequest, SignInResponse},
    service,
};
use crate::{modules::identity::errors, shared_lib::error::ApiError};

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

    let signed_in = service::sign_in(
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
    use crate::modules::identity::{
        auth::token,
        test_support::{context, insert_account, remove_clock, start, PASSWORD, SECRET},
    };

    fn request(email: Option<&str>, password: Option<&str>) -> SignInRequest {
        SignInRequest {
            email: email.map(str::to_string),
            password: password.map(str::to_string),
        }
    }

    async fn sign_in_code(ctx: &AppContext, request: SignInRequest) -> String {
        let err = sign_in(State(ctx.clone()), Ok(Json(request)))
            .await
            .unwrap_err();
        err.body().code.clone()
    }

    #[test]
    fn a_body_that_is_not_json_is_a_validation_failure() {
        for body in ["", "{", "[1]", "{\"email\": 5}"] {
            let rejection = Json::<SignInRequest>::from_bytes(body.as_bytes()).unwrap_err();
            let err = body_rejected(rejection);
            assert_eq!(err.body().code, "validation_failed", "{body:?}");
            assert_eq!(err.body().details, Some(json!({ "fields": ["body"] })));
        }
    }

    #[tokio::test]
    async fn a_rejected_body_is_refused_before_anything_else() {
        let (ctx, _) = context().await;
        let rejection = Json::<SignInRequest>::from_bytes(b"{").unwrap_err();
        let err = sign_in(State(ctx), Err(rejection)).await.unwrap_err();
        assert_eq!(err.body().code, "validation_failed");
    }

    #[tokio::test]
    async fn a_blank_password_is_refused_before_any_lookup() {
        let (ctx, _) = context().await;
        let code = sign_in_code(&ctx, request(Some("alex@example.edu"), Some(" "))).await;
        assert_eq!(code, "validation_failed");
    }

    #[tokio::test]
    async fn signs_in_and_returns_a_bearer_token() {
        let (ctx, _) = context().await;
        let alex = insert_account(&ctx.db, "alex@example.edu", "ACTIVE").await;

        let Json(response) = sign_in(
            State(ctx),
            Ok(Json(request(Some("alex@example.edu"), Some(PASSWORD)))),
        )
        .await
        .unwrap();

        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 86_400);
        let claims = token::verify(&response.access_token, SECRET, start()).unwrap();
        assert_eq!(claims.sub, alex.id);
    }

    #[tokio::test]
    async fn a_wrong_password_is_invalid_credentials() {
        let (ctx, _) = context().await;
        insert_account(&ctx.db, "alex@example.edu", "ACTIVE").await;
        let code = sign_in_code(&ctx, request(Some("alex@example.edu"), Some("Wrong#999"))).await;
        assert_eq!(code, "invalid_credentials");
    }

    #[tokio::test]
    async fn a_missing_clock_or_jwt_configuration_is_an_internal_error() {
        let valid = || request(Some("alex@example.edu"), Some(PASSWORD));

        let (ctx, _) = context().await;
        remove_clock(&ctx);
        assert_eq!(sign_in_code(&ctx, valid()).await, "internal_error");

        let (mut ctx, _) = context().await;
        ctx.config.auth = None;
        assert_eq!(sign_in_code(&ctx, valid()).await, "internal_error");
    }
}
