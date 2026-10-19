//! The application's modules, listed from the lowest layer to the highest.
//!
//! Dependencies only point **down** this list (`records` may use `capture`, never the other
//! way round). `tests/architecture.rs` fails the build if that rule is broken.

pub mod calendar;
pub mod capture;
pub mod courses;
pub mod identity;
pub mod records;

use loco_rs::controller::Routes;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};

/// The Postgres schema of every module (each module owns the schema with its own name).
const SCHEMAS: [&str; 5] = ["calendar", "identity", "courses", "capture", "records"];

/// Routes of every module.
#[must_use]
pub fn routes() -> Vec<Routes> {
    let mut routes = Vec::new();
    routes.extend(calendar::routes());
    routes.extend(identity::routes());
    routes.extend(courses::routes());
    routes.extend(capture::routes());
    routes.extend(records::routes());
    routes
}

/// Empties every table of every module schema. Loco calls this on boot only when
/// `database.dangerously_truncate` is true, which is the case in the test environment only.
///
/// # Errors
/// Returns the database error if a truncate statement fails.
pub async fn truncate(db: &DatabaseConnection) -> Result<(), DbErr> {
    for schema in SCHEMAS.iter().rev() {
        // `schema` is one of the constants above, never user input.
        db.execute_unprepared(&format!(
            "DO $$ DECLARE t record; BEGIN \
               FOR t IN SELECT tablename FROM pg_tables WHERE schemaname = '{schema}' LOOP \
                 EXECUTE format('TRUNCATE TABLE %I.%I RESTART IDENTITY CASCADE', '{schema}', t.tablename); \
               END LOOP; \
             END $$"
        ))
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use loco_rs::tests_cfg::db::dummy_connection;

    use super::*;

    #[test]
    fn collects_the_routes_of_every_module() {
        assert!(!routes().is_empty(), "identity contributes routes");
        assert!(calendar::routes().is_empty());
        assert!(courses::routes().is_empty());
        assert!(capture::routes().is_empty());
        assert!(records::routes().is_empty());
    }

    #[tokio::test]
    async fn truncating_a_database_that_is_not_postgres_fails() {
        // The statement is Postgres-only, so the dummy SQLite database refuses it.
        let db = dummy_connection().await;
        assert!(truncate(&db).await.is_err());
    }
}
