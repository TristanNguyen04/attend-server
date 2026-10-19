//! Makes the application's [`Clock`] available to every module.
//!
//! The clock lives in Loco's shared store as an `Arc<dyn Clock>`. Production gets the
//! [`SystemClock`]; the Cucumber harness puts its own `FixedClock` there before the first
//! request, so scenarios can move time without sleeping.

use std::sync::Arc;

use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Initializer},
    Result,
};

use crate::shared_lib::clock::{Clock, SystemClock};

/// Installs the [`SystemClock`] unless a clock is already in the shared store.
pub struct SystemClockInitializer;

#[async_trait]
impl Initializer for SystemClockInitializer {
    fn name(&self) -> String {
        "system-clock".to_string()
    }

    async fn before_run(&self, ctx: &AppContext) -> Result<()> {
        if !ctx.shared_store.contains::<Arc<dyn Clock>>() {
            let clock: Arc<dyn Clock> = Arc::new(SystemClock);
            ctx.shared_store.insert(clock);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{TimeZone, Utc};
    use loco_rs::{app::Initializer, tests_cfg::app::get_app_context};

    use super::SystemClockInitializer;
    use crate::shared_lib::clock::{Clock, FixedClock};

    #[tokio::test]
    async fn installs_the_system_clock_when_there_is_none() {
        let ctx = get_app_context().await;
        SystemClockInitializer.before_run(&ctx).await.unwrap();
        let clock = ctx.shared_store.get::<Arc<dyn Clock>>().expect("a clock");
        let drift = (Utc::now() - clock.now()).num_seconds().abs();
        assert!(drift < 5, "the installed clock tells the real time");
    }

    #[tokio::test]
    async fn keeps_a_clock_that_is_already_there() {
        let ctx = get_app_context().await;
        let fixed = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
        let installed: Arc<dyn Clock> = Arc::new(FixedClock::new(fixed));
        ctx.shared_store.insert(installed);
        SystemClockInitializer.before_run(&ctx).await.unwrap();
        let clock = ctx.shared_store.get::<Arc<dyn Clock>>().expect("a clock");
        assert_eq!(clock.now(), fixed);
    }
}
