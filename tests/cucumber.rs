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

use std::{collections::HashMap, env, fs, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use attend_server::{
    app::App,
    shared_lib::clock::{Clock, FixedClock},
};
use chrono::{DateTime, TimeZone, Utc};
use cucumber::{
    gherkin::{Feature, Rule, Scenario},
    writer, World, WriterExt,
};
use loco_rs::{
    app::{AppContext, Hooks},
    testing::prelude::boot_test,
    TestServer,
};
use tokio::sync::OnceCell;
use uuid::Uuid;

const FEATURES: &str = "tests/features";

/// Booting the app runs migrations against the shared test database. Doing that per scenario
/// would race on the migration table, so we boot once and share the result.
static BOOTED: OnceCell<Booted> = OnceCell::const_new();

/// Every scenario starts at this instant (see `reset_state`).
fn start_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0)
        .single()
        .expect("a valid instant")
}

#[derive(Clone)]
struct Booted {
    server: Arc<TestServer>,
    ctx: AppContext,
    clock: FixedClock,
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
            // The app reads its clock from the shared store (the production initializer put a
            // `SystemClock` there). Replace it with the world's `FixedClock` before any request:
            // the store is shared with the router, so the app sees the swap.
            let clock = FixedClock::new(start_time());
            let app_clock: Arc<dyn Clock> = Arc::new(clock.clone());
            boot.app_context.shared_store.insert(app_clock);
            let server = TestServer::new(
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .expect("failed to build the test server");
            Booted {
                server: Arc::new(server),
                ctx: boot.app_context.clone(),
                clock,
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
    /// The clock the app uses. Steps move time with it; no scenario sleeps.
    pub clock: FixedClock,
    /// Ids of the accounts a scenario created, by email.
    pub accounts: HashMap<String, Uuid>,
    /// The access token of the last successful sign-in.
    pub access_token: Option<String>,
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
            clock: booted.clock,
            accounts: HashMap::new(),
            access_token: None,
            last_status: None,
            last_json: None,
        }
    }
}

/// Runs before every scenario: no data left over from the previous one, and time back at the
/// start, so a Background can create the same account every time.
fn reset_state<'a>(
    _feature: &'a Feature,
    _rule: Option<&'a Rule>,
    _scenario: &'a Scenario,
    world: &'a mut AppWorld,
) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
    Box::pin(async move {
        App::truncate(&world.ctx)
            .await
            .expect("failed to empty the test database");
        world.clock.set(start_time());
    })
}

/// Set this to a directory to also write `cucumber.json` (Cucumber JSON) and `cucumber.xml`
/// (`JUnit`) there. Unset, the run prints to the console only. CI sets it.
const REPORT_DIR_VAR: &str = "CUCUMBER_REPORT_DIR";

#[tokio::main]
async fn main() {
    let cucumber = AppWorld::cucumber()
        .before(reset_state)
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
