//! Accounts: the `identity.users` table. Lookups and the rules about who may use the API, plus
//! `GET /api/me` (see `controller.rs`).

mod controller;
mod dto;
mod email;

use loco_rs::controller::Routes;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::{entities::users, errors};
use crate::shared_lib::error::ApiError;

/// The only status that may sign in and use the API.
const ACTIVE: &str = "ACTIVE";

pub(super) fn routes() -> Routes {
    controller::routes()
}

/// The account with this id, if there is one.
///
/// # Errors
/// `internal_error` if the database fails.
pub(super) async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<users::Model>, ApiError> {
    users::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(errors::internal)
}

/// The account with this email, in any case and with surrounding whitespace, if there is one.
///
/// # Errors
/// `internal_error` if the database fails.
pub(super) async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<users::Model>, ApiError> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email::normalize(email)))
        .one(db)
        .await
        .map_err(errors::internal)
}

/// The account if it exists and is `ACTIVE`.
///
/// # Errors
/// `invalid_token` if there is no such account; `account_not_active` if it is not `ACTIVE`.
pub(super) fn require_active(account: Option<users::Model>) -> Result<users::Model, ApiError> {
    let account = account.ok_or_else(errors::invalid_token)?;
    ensure_active(&account)?;
    Ok(account)
}

/// # Errors
/// `account_not_active` if the account is not `ACTIVE`.
pub(super) fn ensure_active(account: &users::Model) -> Result<(), ApiError> {
    if account.status == ACTIVE {
        Ok(())
    } else {
        Err(errors::account_not_active())
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use sea_orm::{Database, DatabaseConnection};
    use uuid::Uuid;

    use super::*;
    use crate::modules::identity::test_support::{database, insert_account};

    async fn without_the_table() -> DatabaseConnection {
        Database::connect("sqlite::memory:").await.unwrap()
    }

    #[test]
    fn contributes_one_route() {
        let _ = routes();
    }

    #[tokio::test]
    async fn finds_an_account_by_id() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        let found = find_by_id(&db, alex.id).await.unwrap();
        assert_eq!(found.unwrap().email, "alex@example.edu");
        assert!(find_by_id(&db, Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn finds_an_account_by_email_in_any_case_with_surrounding_spaces() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        for email in [
            "alex@example.edu",
            "ALEX@Example.edu",
            "  alex@example.edu\t",
        ] {
            let found = find_by_email(&db, email).await.unwrap();
            assert_eq!(found.unwrap().id, alex.id, "{email:?}");
        }
        assert!(find_by_email(&db, "nobody@example.edu")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn a_database_failure_is_an_internal_error() {
        let db = without_the_table().await;
        let by_id = find_by_id(&db, Uuid::new_v4()).await.unwrap_err();
        assert_eq!(by_id.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let by_email = find_by_email(&db, "alex@example.edu").await.unwrap_err();
        assert_eq!(by_email.body().code, "internal_error");
    }

    #[tokio::test]
    async fn an_active_account_is_let_through() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        let id = alex.id;
        assert_eq!(require_active(Some(alex)).unwrap().id, id);
    }

    #[tokio::test]
    async fn an_account_that_is_not_active_is_refused() {
        let db = database().await;
        for (email, status) in [("a@example.edu", "INVITED"), ("b@example.edu", "SUSPENDED")] {
            let account = insert_account(&db, email, status).await;
            let err = require_active(Some(account)).unwrap_err();
            assert_eq!(err.status(), StatusCode::FORBIDDEN, "{status}");
            assert_eq!(err.body().code, "account_not_active");
        }
    }

    #[test]
    fn a_token_for_a_missing_account_is_invalid() {
        assert_eq!(
            require_active(None).unwrap_err().body().code,
            "invalid_token"
        );
    }
}
