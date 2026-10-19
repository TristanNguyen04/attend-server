//! The Cucumber (BDD) harness.
//!
//! Run everything:      cargo test --test cucumber
//! Run one feature:     cargo test --test cucumber -- --name "Health"
//! With a report:       `CUCUMBER_REPORT_DIR=target/cucumber` cargo test --test cucumber
//!                      (also writes cucumber.json and cucumber.xml there; CI does this)
//!
//! The real Loco app is booted once and driven over HTTP through an in-process test server,
//! the same request path production uses. Features live in `tests/features/<module>/` and
//! their step definitions in `tests/steps/`.

mod steps;

use std::{env, fs, path::PathBuf, sync::Arc};

use attend_server::app::App;
use cucumber::{writer, World, WriterExt};
use loco_rs::{app::AppContext, testing::prelude::boot_test, TestServer};
use tokio::sync::OnceCell;

const FEATURES: &str = "tests/features";

/// Booting the app runs migrations against the shared test database. Doing that per scenario
/// would race on the migration table, so we boot once and share the result.
static BOOTED: OnceCell<Booted> = OnceCell::const_new();

#[derive(Clone)]
struct Booted {
    server: Arc<TestServer>,
    ctx: AppContext,
}

async fn booted() -> Booted {
    BOOTED
        .get_or_init(|| async {
            let boot = boot_test::<App>()
                .await
                .expect("failed to boot the test app");
            let router = boot
                .router
                .clone()
                .expect("the boot result should include a router");
            let server = TestServer::new(
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .expect("failed to build the test server");
            Booted {
                server: Arc::new(server),
                ctx: boot.app_context.clone(),
            }
        })
        .await
        .clone()
}

/// State shared by the steps of one scenario.
#[derive(cucumber::World)]
#[world(init = Self::new)]
pub struct AppWorld {
    pub server: Arc<TestServer>,
    pub ctx: AppContext,
    /// Status of the last HTTP response.
    pub last_status: Option<u16>,
    /// Parsed JSON body of the last HTTP response, if it had one.
    pub last_json: Option<serde_json::Value>,
}

impl std::fmt::Debug for AppWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppWorld")
            .field("last_status", &self.last_status)
            .finish_non_exhaustive()
    }
}

impl AppWorld {
    async fn new() -> Self {
        let booted = booted().await;
        Self {
            server: booted.server,
            ctx: booted.ctx,
            last_status: None,
            last_json: None,
        }
    }
}

/// Set this to a directory to also write `cucumber.json` (Cucumber JSON) and `cucumber.xml`
/// (`JUnit`) there. Unset, the run prints to the console only. CI sets it.
const REPORT_DIR_VAR: &str = "CUCUMBER_REPORT_DIR";

#[tokio::main]
async fn main() {
    let cucumber = AppWorld::cucumber()
        // Scenarios share one database, so they run one at a time.
        .max_concurrent_scenarios(1);

    // `with_writer` replaces the writer, so `fail_on_skipped` comes after it in both branches.
    // An undefined or skipped step is a failure: BDD "red" must be visible.
    match env::var_os(REPORT_DIR_VAR).map(PathBuf::from) {
        None => cucumber.fail_on_skipped().run_and_exit(FEATURES).await,
        Some(dir) => {
            fs::create_dir_all(&dir).expect("failed to create the report directory");
            let create = |name: &str| {
                fs::File::create(dir.join(name)).expect("failed to create a report file")
            };
            // The console output stays; the report writers receive the same events.
            let writer = writer::Basic::stdout()
                .summarized()
                .tee::<AppWorld, _>(writer::Json::for_tee(create("cucumber.json")))
                .tee::<AppWorld, _>(writer::JUnit::for_tee(create("cucumber.xml"), 0))
                .normalized();
            cucumber
                .with_writer(writer)
                .fail_on_skipped()
                .run_and_exit(FEATURES)
                .await;
        }
    }
}
