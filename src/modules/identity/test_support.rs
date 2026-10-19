//! Helpers for the unit tests of this module: a context with a JWT secret, a `FixedClock` and an
//! in-memory database that has the `identity.users` table.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use loco_rs::{
    app::AppContext,
    config::{Auth, JWT},
    hash::hash_password,
    tests_cfg::app::get_app_context,
};
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Set,
};
use uuid::Uuid;

use super::entities::users;
use crate::shared_lib::clock::{Clock, FixedClock};

pub const SECRET: &str = "RrG2eLupO9JYL4YLcJGeIMP/4iVIpi6RqXXkdbQtQqwLSX5LwtdaI0rVD3QkALHL";
pub const PASSWORD: &str = "Correct#123";

pub fn start() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap()
}

/// An in-memory database with an empty `identity.users` table (SQLite, the schema attached
/// under the name `identity`). One connection only, so the attachment is always there.
pub async fn database() -> DatabaseConnection {
    let mut options = ConnectOptions::new("sqlite::memory:");
    options.max_connections(1);
    let db = Database::connect(options).await.unwrap();
    db.execute_unprepared("ATTACH DATABASE ':memory:' AS identity")
        .await
        .unwrap();
    db.execute_unprepared(
        "CREATE TABLE identity.users (
             id BLOB PRIMARY KEY, email TEXT NOT NULL UNIQUE, full_name TEXT NOT NULL,
             password_hash TEXT NOT NULL, status TEXT NOT NULL,
             created_at TEXT NOT NULL, updated_at TEXT NOT NULL)",
    )
    .await
    .unwrap();
    db
}

/// Inserts an account whose password is [`PASSWORD`].
pub async fn insert_account(db: &DatabaseConnection, email: &str, status: &str) -> users::Model {
    let now = start().fixed_offset();
    users::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email.to_string()),
        full_name: Set("Alex Tan".to_string()),
        password_hash: Set(hash_password(PASSWORD).unwrap()),
        status: Set(status.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .unwrap()
}

/// A context with a JWT secret, a `FixedClock` at [`start`] and the in-memory database of
/// [`database`]. Returns the clock so a test can move time.
pub async fn context() -> (AppContext, FixedClock) {
    let mut ctx = get_app_context().await;
    ctx.db = database().await;
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

/// Takes the clock out of the context, as if the initializer had not run.
pub fn remove_clock(ctx: &AppContext) {
    let removed = ctx.shared_store.remove::<Arc<dyn Clock>>();
    assert!(removed.is_some(), "the context had a clock");
}
