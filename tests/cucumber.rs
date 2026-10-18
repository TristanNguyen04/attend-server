//! The Cucumber (BDD) harness.
//!
//! Run everything:      cargo test --test cucumber
//! Run one feature:     cargo test --test cucumber -- --name "Health"
//!
//! The real Loco app is booted once and driven over HTTP through an in-process test server,
//! the same request path production uses. Features live in `tests/features/<module>/` and
//! their step definitions in `tests/steps/`.

mod steps;

use std::sync::Arc;

use attend_server::app::App;
use cucumber::World;
use loco_rs::{app::AppContext, testing::prelude::boot_test, TestServer};
use tokio::sync::OnceCell;

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

#[tokio::main]
async fn main() {
    AppWorld::cucumber()
        // Scenarios share one database, so they run one at a time.
        .max_concurrent_scenarios(1)
        // An undefined or skipped step is a failure: BDD "red" must be visible.
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
