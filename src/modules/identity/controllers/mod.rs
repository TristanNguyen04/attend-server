//! HTTP handlers (Loco controllers). Private to the module: expose routes through `super::routes`.

mod auth;
mod authenticated;
mod me;

use std::sync::Arc;

use loco_rs::{app::AppContext, config::JWT, controller::Routes};

use super::models::errors;
use crate::shared_lib::{clock::Clock, error::ApiError};

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

pub(super) fn routes() -> Vec<Routes> {
    vec![auth::routes(), me::routes()]
}

#[cfg(test)]
pub(super) mod test_support {
    use std::sync::Arc;

    use chrono::{DateTime, TimeZone, Utc};
    use loco_rs::{
        app::AppContext,
        config::{Auth, JWT},
        tests_cfg::app::get_app_context,
    };

    use crate::shared_lib::clock::{Clock, FixedClock};

    pub const SECRET: &str =
        "ZGV2LWFuZC10ZXN0LW9ubHktand0LXNlY3JldC1uZXZlci11c2VkLWluLXByb2R1Y3Rpb24hIQ==";

    pub fn start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap()
    }

    /// A context with a JWT secret and a `FixedClock` at [`start`]; its database is a dummy
    /// without the `identity` schema. Returns the clock so a test can move time.
    pub async fn context() -> (AppContext, FixedClock) {
        let mut ctx = get_app_context().await;
        ctx.config.auth = Some(Auth {
            jwt: Some(JWT {
                location: None,
                secret: SECRET.to_string(),
                expiration: 86_400,
            }),
        });
        let clock = FixedClock::new(start());
        let shared: Arc<dyn Clock> = Arc::new(clock.clone());
        ctx.shared_store.insert(shared);
        (ctx, clock)
    }
}

#[cfg(test)]
mod tests {
    use loco_rs::tests_cfg::app::get_app_context;

    use super::*;

    #[tokio::test]
    async fn the_jwt_settings_are_read_from_the_configuration() {
        let (ctx, _) = test_support::context().await;
        let jwt = jwt_config(&ctx).unwrap();
        assert_eq!(jwt.secret, test_support::SECRET);
        assert_eq!(jwt.expiration, 86_400);
    }

    #[tokio::test]
    async fn missing_jwt_settings_are_an_internal_error() {
        let mut ctx = get_app_context().await;
        ctx.config.auth = None;
        assert_eq!(
            jwt_config(&ctx).err().unwrap().body().code,
            "internal_error"
        );
    }

    #[tokio::test]
    async fn the_clock_is_read_from_the_shared_store() {
        let (ctx, _) = test_support::context().await;
        assert_eq!(clock(&ctx).unwrap().now(), test_support::start());
    }

    #[tokio::test]
    async fn a_missing_clock_is_an_internal_error() {
        let ctx = get_app_context().await;
        assert_eq!(clock(&ctx).err().unwrap().body().code, "internal_error");
    }
}
