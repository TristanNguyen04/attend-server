//! Signing in and proving who you are.
//!
//! - `controller.rs`: `POST /api/auth/sign-in`.
//! - `dto.rs`: the sign-in request and response, and the validation of the request.
//! - `service.rs`: the sign-in logic (password check, status check, token).
//! - `token.rs`: access tokens (JWT, HS256) issued and verified against the injected `Clock`.
//! - `extractor.rs`: [`Authenticated`], which a handler takes to require a valid access token.

mod controller;
mod dto;
mod extractor;
mod service;
mod token;

use std::sync::Arc;

use loco_rs::{app::AppContext, config::JWT, controller::Routes};

pub(super) use extractor::Authenticated;

use super::errors;
use crate::shared_lib::{clock::Clock, error::ApiError};

pub(super) fn routes() -> Routes {
    controller::routes()
}

/// The `auth.jwt` section of the configuration.
fn jwt_config(ctx: &AppContext) -> Result<&JWT, ApiError> {
    ctx.config
        .auth
        .as_ref()
        .and_then(|auth| auth.jwt.as_ref())
        .ok_or_else(|| errors::internal("auth.jwt is not configured"))
}

/// The application's clock (see `initializers::clock`).
fn clock(ctx: &AppContext) -> Result<Arc<dyn Clock>, ApiError> {
    ctx.shared_store
        .get::<Arc<dyn Clock>>()
        .ok_or_else(|| errors::internal("no clock in the shared store"))
}

#[cfg(test)]
mod tests {
    use loco_rs::tests_cfg::app::get_app_context;

    use super::*;
    use crate::modules::identity::test_support::{context, start, SECRET};

    #[tokio::test]
    async fn the_jwt_settings_are_read_from_the_configuration() {
        let (ctx, _) = context().await;
        let jwt = jwt_config(&ctx).unwrap();
        assert_eq!(jwt.secret, SECRET);
        assert_eq!(jwt.expiration, 86_400);
    }

    #[tokio::test]
    async fn missing_jwt_settings_are_an_internal_error() {
        let mut ctx = get_app_context().await;
        ctx.config.auth = None;
        assert_eq!(jwt_config(&ctx).unwrap_err().body().code, "internal_error");
    }

    #[tokio::test]
    async fn the_clock_is_read_from_the_shared_store() {
        let (ctx, _) = context().await;
        assert_eq!(clock(&ctx).unwrap().now(), start());
    }

    #[tokio::test]
    async fn a_missing_clock_is_an_internal_error() {
        let ctx = get_app_context().await;
        assert_eq!(
            clock(&ctx).map(|_| ()).unwrap_err().body().code,
            "internal_error"
        );
    }

    #[test]
    fn contributes_one_route() {
        let _ = routes();
    }
}
