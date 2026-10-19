//! The authentication extractor: a handler that takes [`Authenticated`] only runs for a request
//! that carries a valid, unexpired access token of an `ACTIVE` account, and receives that
//! account.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderMap},
};
use loco_rs::app::AppContext;

use crate::{
    modules::identity::{
        entities::users,
        models::{accounts, errors, tokens},
    },
    shared_lib::error::ApiError,
};

/// The account a request is made as.
pub(super) struct Authenticated(pub(super) users::Model);

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
        let claims = tokens::verify(token, &jwt.secret, super::clock(ctx)?.now())?;
        let account = accounts::find_by_id(&ctx.db, claims.sub).await?;
        Ok(Self(accounts::require_active(account)?))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue, Request};
    use chrono::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::modules::identity::controllers::test_support::{context, start, SECRET};

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

    /// The rejection code of a request with this `Authorization` header (`None`: no header).
    async fn rejection(ctx: &AppContext, authorization: Option<String>) -> (u16, String) {
        let mut request = Request::builder().uri("/api/me");
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        let (mut parts, ()) = request.body(()).unwrap().into_parts();
        let err = Authenticated::from_request_parts(&mut parts, ctx)
            .await
            .err()
            .expect("the request is rejected");
        (err.status().as_u16(), err.body().code.clone())
    }

    fn token(lifetime_secs: u64) -> String {
        tokens::issue(Uuid::new_v4(), start(), lifetime_secs, SECRET).unwrap()
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
        let header = Some(format!("Bearer {}", token(100)));
        clock.advance(Duration::seconds(100));
        assert_eq!(rejection(&ctx, header).await, (401, "token_expired".into()));
    }

    #[tokio::test]
    async fn a_valid_token_goes_on_to_the_database() {
        // One second before expiry the token is accepted; the dummy database then fails,
        // which is an `internal_error`.
        let (ctx, clock) = context().await;
        let header = Some(format!("Bearer {}", token(100)));
        clock.advance(Duration::seconds(99));
        assert_eq!(
            rejection(&ctx, header).await,
            (500, "internal_error".into())
        );
    }
}
