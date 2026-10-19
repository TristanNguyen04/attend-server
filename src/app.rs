use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

use crate::{initializers::clock::SystemClockInitializer, modules};

/// The Loco application. This file only aggregates what each module contributes;
/// keep business logic out of it.
pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(SystemClockInitializer)])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        modules::routes()
            .into_iter()
            .fold(AppRoutes::with_default_routes(), AppRoutes::add_route)
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    async fn truncate(ctx: &AppContext) -> Result<()> {
        modules::truncate(&ctx.db).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loco_rs::tests_cfg::app::get_app_context;

    #[test]
    fn the_app_is_named_after_the_crate() {
        assert_eq!(App::app_name(), "attend_server");
    }

    #[test]
    fn the_version_is_the_package_version_followed_by_the_build() {
        let version = App::app_version();
        let prefix = format!("{} (", env!("CARGO_PKG_VERSION"));
        assert!(
            version.starts_with(&prefix),
            "unexpected version: {version}"
        );
        assert!(version.ends_with(')'), "unexpected version: {version}");
    }

    #[test]
    fn no_task_is_registered_yet() {
        let mut tasks = Tasks::default();
        App::register_tasks(&mut tasks);
        assert!(tasks.names().is_empty());
    }

    #[tokio::test]
    async fn only_the_clock_initializer_is_wired_and_no_worker_or_seed_yet() {
        let ctx = get_app_context().await;

        let initializers = App::initializers(&ctx).await.unwrap();
        let names: Vec<_> = initializers.iter().map(|i| i.name()).collect();
        assert_eq!(names, ["system-clock"]);
        App::connect_workers(&ctx, &Queue::empty()).await.unwrap();
        App::seed(&ctx, Path::new("")).await.unwrap();
    }
}
