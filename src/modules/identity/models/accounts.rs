//! Accounts: finding one by email or id, and signing it in.

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use loco_rs::hash::{hash_password, verify_password};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::{credentials::Credentials, emails, errors, tokens};
use crate::{modules::identity::entities::users, shared_lib::error::ApiError};

/// An access token and how long it lives.
pub(crate) struct SignedIn {
    pub access_token: String,
    /// Seconds from `now` until the token expires.
    pub expires_in: u64,
}

/// The only status that may sign in and use the API.
const ACTIVE: &str = "ACTIVE";

/// The password the dummy hash was made from. It never matches a real attempt by design: the
/// dummy hash exists only so an unknown email costs the same Argon2 work as a known one (NFR2).
const DUMMY_PASSWORD: &str = "dummy password for unknown accounts";

/// A hash made with the same Argon2 parameters as real passwords (both come from Loco).
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password(DUMMY_PASSWORD).expect("the dummy password can be hashed"));

/// Whether `password` is the one `password_hash` (Argon2) was made from.
pub(crate) fn password_matches(password: &str, password_hash: &str) -> bool {
    verify_password(password, password_hash)
}

/// The hash a password attempt is checked against: the account's, or the dummy one when there
/// is no such account.
fn hash_to_check(account: Option<&users::Model>) -> &str {
    account.map_or(DUMMY_HASH.as_str(), |account| {
        account.password_hash.as_str()
    })
}

/// The account if it exists and is `ACTIVE`.
///
/// # Errors
/// `invalid_token` if there is no such account; `account_not_active` if it is not `ACTIVE`.
pub(crate) fn require_active(account: Option<users::Model>) -> Result<users::Model, ApiError> {
    let account = account.ok_or_else(errors::invalid_token)?;
    ensure_active(&account)?;
    Ok(account)
}

/// # Errors
/// `account_not_active` if the account is not `ACTIVE`.
fn ensure_active(account: &users::Model) -> Result<(), ApiError> {
    if account.status == ACTIVE {
        Ok(())
    } else {
        Err(errors::account_not_active())
    }
}

/// The account with this id, if there is one.
///
/// # Errors
/// `internal_error` if the database fails.
pub(crate) async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<users::Model>, ApiError> {
    users::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(errors::internal)
}

/// The account with this email, in canonical form, if there is one.
async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<users::Model>, ApiError> {
    users::Entity::find()
        .filter(users::Column::Email.eq(emails::normalize(email)))
        .one(db)
        .await
        .map_err(errors::internal)
}

/// Signs in the account with these credentials. The email may be in any case and have
/// surrounding whitespace.
///
/// The password is always checked, against a dummy hash when the email is unknown, and the
/// status is looked at only after the password matched, so neither reveals anything about an
/// account to someone who does not know its password.
///
/// # Errors
/// `invalid_credentials` for an unknown email or a wrong password; `account_not_active` for a
/// correct password on an account that is not `ACTIVE`; `internal_error` if the database
/// fails or the signing secret is not valid base64.
pub(crate) async fn sign_in(
    db: &DatabaseConnection,
    now: DateTime<Utc>,
    credentials: &Credentials,
    secret: &str,
    lifetime_secs: u64,
) -> Result<SignedIn, ApiError> {
    let account = find_by_email(db, &credentials.email).await?;
    let password_ok = password_matches(&credentials.password, hash_to_check(account.as_ref()));
    let account = account
        .filter(|_| password_ok)
        .ok_or_else(errors::invalid_credentials)?;
    ensure_active(&account)?;
    let access_token =
        tokens::issue(account.id, now, lifetime_secs, secret).map_err(errors::internal)?;
    Ok(SignedIn {
        access_token,
        expires_in: lifetime_secs,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use loco_rs::hash::hash_password;
    use uuid::Uuid;

    use super::*;

    fn account(status: &str, password: &str) -> users::Model {
        let now = Utc::now().fixed_offset();
        users::Model {
            id: Uuid::new_v4(),
            email: "alex@example.edu".to_string(),
            full_name: "Alex Tan".to_string(),
            password_hash: hash_password(password).unwrap(),
            status: status.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn a_password_matches_its_argon2_hash() {
        let hash = hash_password("Correct#123").unwrap();
        assert!(hash.starts_with("$argon2"), "the hash is Argon2");
        assert!(password_matches("Correct#123", &hash));
    }

    #[test]
    fn another_password_does_not_match() {
        let hash = hash_password("Correct#123").unwrap();
        assert!(!password_matches("Wrong#999", &hash));
        assert!(!password_matches("correct#123", &hash), "case-sensitive");
    }

    #[test]
    fn an_unknown_email_is_checked_against_the_dummy_hash() {
        let hash = hash_to_check(None);
        assert!(
            hash.starts_with("$argon2"),
            "a real Argon2 hash, so the work is the same"
        );
        assert!(password_matches(DUMMY_PASSWORD, hash), "and a valid one");
        assert!(!password_matches("Correct#123", hash));
    }

    #[test]
    fn a_known_email_is_checked_against_its_own_hash() {
        let account = account("ACTIVE", "Correct#123");
        assert_eq!(hash_to_check(Some(&account)), account.password_hash);
    }

    #[test]
    fn an_active_account_is_let_through() {
        let account = account("ACTIVE", "Correct#123");
        let id = account.id;
        assert_eq!(require_active(Some(account)).unwrap().id, id);
    }

    #[test]
    fn an_account_that_is_not_active_is_refused() {
        for status in ["INVITED", "SUSPENDED"] {
            let err = require_active(Some(account(status, "p"))).err().unwrap();
            assert_eq!(err.status(), axum::http::StatusCode::FORBIDDEN, "{status}");
            assert_eq!(err.body().code, "account_not_active");
        }
    }

    #[test]
    fn a_token_for_a_missing_account_is_invalid() {
        let err = require_active(None).err().unwrap();
        assert_eq!(err.body().code, "invalid_token");
    }
}
