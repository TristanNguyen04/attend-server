//! The authentication extractor: a handler that takes [`Authenticated`] only runs for a request
//! that carries a valid, unexpired access token of an `ACTIVE` account, and receives that
//! account.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};
use loco_rs::app::AppContext;

use super::token;
use crate::{
    modules::identity::{account, entities::users, errors},
    shared_lib::error::ApiError,
};

/// The account a request is made as.
pub(crate) struct Authenticated(pub(crate) users::Model);

/// The token of an `Authorization: Bearer <token>` header.
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

impl FromRequestParts<AppContext> for Authenticated {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, ctx: &AppContext) -> Result<Self, ApiError> {
        let token = bearer_token(&parts.headers).ok_or_else(errors::unauthenticated)?;
        let jwt = super::jwt_config(ctx)?;
        let claims = token::verify(token, &jwt.secret, super::clock(ctx)?.now())?;
        let found = account::find_by_id(&ctx.db, claims.sub).await?;
        Ok(Self(account::require_active(found)?))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, Request};
    use chrono::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::modules::identity::test_support::{
        context, insert_account, remove_clock, start, SECRET,
    };

    fn headers(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn the_token_is_what_follows_bearer() {
        assert_eq!(
            bearer_token(&headers("Bearer abc.def.ghi")),
            Some("abc.def.ghi")
        );
    }

    #[test]
    fn there_is_no_token_without_a_bearer_header() {
        assert_eq!(bearer_token(&HeaderMap::new()), None);
        assert_eq!(bearer_token(&headers("Basic abc")), None);
        let mut not_text = HeaderMap::new();
        not_text.insert(
            AUTHORIZATION,
            HeaderValue::from_bytes(b"Bearer \xff").unwrap(),
        );
        assert_eq!(bearer_token(&not_text), None);
    }

    /// Runs the extractor for a request with this `Authorization` header (`None`: no header).
    async fn extract(
        ctx: &AppContext,
        authorization: Option<String>,
    ) -> Result<Authenticated, ApiError> {
        let mut request = Request::builder().uri("/api/me");
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        let (mut parts, ()) = request.body(()).unwrap().into_parts();
        Authenticated::from_request_parts(&mut parts, ctx).await
    }

    async fn rejection(ctx: &AppContext, authorization: Option<String>) -> (u16, String) {
        let err = extract(ctx, authorization).await.err().unwrap();
        (err.status().as_u16(), err.body().code.clone())
    }

    fn bearer(subject: Uuid, lifetime_secs: u64) -> String {
        let token = token::issue(subject, start(), lifetime_secs, SECRET).unwrap();
        format!("Bearer {token}")
    }

    #[tokio::test]
    async fn yields_the_account_of_a_valid_token() {
        let (ctx, _) = context().await;
        let alex = insert_account(&ctx.db, "alex@example.edu", "ACTIVE").await;
        let Authenticated(account) = extract(&ctx, Some(bearer(alex.id, 100)))
            .await
            .ok()
            .unwrap();
        assert_eq!(account.id, alex.id);
    }

    #[tokio::test]
    async fn no_header_is_unauthenticated() {
        let (ctx, _) = context().await;
        assert_eq!(rejection(&ctx, None).await, (401, "unauthenticated".into()));
    }

    #[tokio::test]
    async fn a_malformed_token_is_invalid() {
        let (ctx, _) = context().await;
        let header = Some("Bearer not-a-jwt".to_string());
        assert_eq!(rejection(&ctx, header).await, (401, "invalid_token".into()));
    }

    #[tokio::test]
    async fn an_expired_token_is_rejected_by_the_clock() {
        let (ctx, clock) = context().await;
        let alex = insert_account(&ctx.db, "alex@example.edu", "ACTIVE").await;
        let header = Some(bearer(alex.id, 100));
        clock.advance(Duration::seconds(100));
        assert_eq!(rejection(&ctx, header).await, (401, "token_expired".into()));
    }

    #[tokio::test]
    async fn a_token_one_second_before_expiry_still_works() {
        let (ctx, clock) = context().await;
        let alex = insert_account(&ctx.db, "alex@example.edu", "ACTIVE").await;
        let header = Some(bearer(alex.id, 100));
        clock.advance(Duration::seconds(99));
        assert!(extract(&ctx, header).await.is_ok());
    }

    #[tokio::test]
    async fn a_token_of_an_account_that_is_not_active_is_refused() {
        let (ctx, _) = context().await;
        let alex = insert_account(&ctx.db, "alex@example.edu", "SUSPENDED").await;
        let result = rejection(&ctx, Some(bearer(alex.id, 100))).await;
        assert_eq!(result, (403, "account_not_active".into()));
    }

    #[tokio::test]
    async fn a_token_of_a_missing_account_is_invalid() {
        let (ctx, _) = context().await;
        let result = rejection(&ctx, Some(bearer(Uuid::new_v4(), 100))).await;
        assert_eq!(result, (401, "invalid_token".into()));
    }

    #[tokio::test]
    async fn missing_configuration_is_an_internal_error() {
        let (mut ctx, _) = context().await;
        let header = Some(bearer(Uuid::new_v4(), 100));
        ctx.config.auth = None;
        assert_eq!(
            rejection(&ctx, header.clone()).await,
            (500, "internal_error".into())
        );

        let (ctx, _) = context().await;
        remove_clock(&ctx);
        assert_eq!(
            rejection(&ctx, header).await,
            (500, "internal_error".into())
        );
    }
}
