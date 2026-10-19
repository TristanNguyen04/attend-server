//! The sign-in logic.

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use loco_rs::hash::{hash_password, verify_password};
use sea_orm::DatabaseConnection;

use super::{dto::Credentials, token};
use crate::{
    modules::identity::{account, entities::users, errors},
    shared_lib::error::ApiError,
};

/// An access token and how long it lives.
#[derive(Debug)]
pub(crate) struct SignedIn {
    pub access_token: String,
    /// Seconds from `now` until the token expires.
    pub expires_in: u64,
}

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

/// The account if `password` is its password and it is `ACTIVE`.
///
/// The password is always checked, against a dummy hash when there is no account, and the
/// status is looked at only after the password matched, so neither reveals anything about an
/// account to someone who does not know its password.
///
/// # Errors
/// `invalid_credentials` for no account or a wrong password; `account_not_active` for a correct
/// password on an account that is not `ACTIVE`.
fn check_credentials(
    account: Option<users::Model>,
    password: &str,
) -> Result<users::Model, ApiError> {
    let password_ok = password_matches(password, hash_to_check(account.as_ref()));
    let account = account
        .filter(|_| password_ok)
        .ok_or_else(errors::invalid_credentials)?;
    account::ensure_active(&account)?;
    Ok(account)
}

/// An access token for `account`, valid from `now` for `lifetime_secs` seconds.
///
/// # Errors
/// `internal_error` if `secret` is not valid base64.
fn signed_in(
    account: &users::Model,
    now: DateTime<Utc>,
    lifetime_secs: u64,
    secret: &str,
) -> Result<SignedIn, ApiError> {
    let access_token =
        token::issue(account.id, now, lifetime_secs, secret).map_err(errors::internal)?;
    Ok(SignedIn {
        access_token,
        expires_in: lifetime_secs,
    })
}

/// Signs in the account with these credentials. The email may be in any case and have
/// surrounding whitespace.
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
    let found = account::find_by_email(db, &credentials.email).await?;
    let account = check_credentials(found, &credentials.password)?;
    signed_in(&account, now, lifetime_secs, secret)
}

#[cfg(test)]
mod tests {
    use sea_orm::Database;

    use super::*;
    use crate::modules::identity::test_support::{
        database, insert_account, start, PASSWORD, SECRET,
    };

    fn credentials(email: &str, password: &str) -> Credentials {
        Credentials {
            email: email.to_string(),
            password: password.to_string(),
        }
    }

    fn code<T: std::fmt::Debug>(result: Result<T, ApiError>) -> String {
        result.unwrap_err().body().code.clone()
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
        assert!(!password_matches(PASSWORD, hash));
    }

    #[tokio::test]
    async fn a_known_email_is_checked_against_its_own_hash() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        assert_eq!(hash_to_check(Some(&alex)), alex.password_hash);
    }

    #[tokio::test]
    async fn the_right_password_of_an_active_account_passes() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        let id = alex.id;
        assert_eq!(check_credentials(Some(alex), PASSWORD).unwrap().id, id);
    }

    #[tokio::test]
    async fn a_wrong_password_and_an_unknown_email_fail_the_same_way() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        let wrong = check_credentials(Some(alex), "Wrong#999").unwrap_err();
        let unknown = check_credentials(None, PASSWORD).unwrap_err();
        assert_eq!(wrong.status(), unknown.status());
        assert_eq!(wrong.body(), unknown.body());
        assert_eq!(wrong.body().code, "invalid_credentials");
        assert_eq!(wrong.body().message, "Invalid email or password");
    }

    #[tokio::test]
    async fn an_account_that_is_not_active_is_refused_only_after_the_password_matched() {
        let db = database().await;
        for (email, status) in [("a@example.edu", "INVITED"), ("b@example.edu", "SUSPENDED")] {
            let account = insert_account(&db, email, status).await;
            assert_eq!(
                code(check_credentials(Some(account.clone()), PASSWORD)),
                "account_not_active"
            );
            assert_eq!(
                code(check_credentials(Some(account), "Wrong#999")),
                "invalid_credentials"
            );
        }
    }

    #[tokio::test]
    async fn issues_a_token_that_lives_for_the_lifetime() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;

        let signed_in = signed_in(&alex, start(), 86_400, SECRET).unwrap();

        assert_eq!(signed_in.expires_in, 86_400);
        let claims = token::verify(&signed_in.access_token, SECRET, start()).unwrap();
        assert_eq!(claims.sub, alex.id);
    }

    #[tokio::test]
    async fn a_secret_that_is_not_base64_is_an_internal_error() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        assert_eq!(
            code(signed_in(&alex, start(), 86_400, "not base64!")),
            "internal_error"
        );
    }

    #[tokio::test]
    async fn signs_in_with_any_case_and_surrounding_spaces() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;

        let signed_in = sign_in(
            &db,
            start(),
            &credentials(" ALEX@example.edu ", PASSWORD),
            SECRET,
            86_400,
        )
        .await
        .unwrap();

        let claims = token::verify(&signed_in.access_token, SECRET, start()).unwrap();
        assert_eq!(claims.sub, alex.id);
    }

    #[tokio::test]
    async fn refuses_an_unknown_email_a_wrong_password_and_an_inactive_account() {
        let db = database().await;
        insert_account(&db, "alex@example.edu", "ACTIVE").await;
        insert_account(&db, "banned@example.edu", "SUSPENDED").await;
        let attempts = [
            ("nobody@example.edu", PASSWORD, "invalid_credentials"),
            ("alex@example.edu", "Wrong#999", "invalid_credentials"),
            ("banned@example.edu", PASSWORD, "account_not_active"),
        ];
        for (email, password, expected) in attempts {
            let result = sign_in(&db, start(), &credentials(email, password), SECRET, 86_400).await;
            assert_eq!(code(result), expected, "{email}");
        }
    }

    #[tokio::test]
    async fn a_database_failure_is_an_internal_error() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let result = sign_in(&db, start(), &credentials("a@b.c", "p"), SECRET, 86_400).await;
        assert_eq!(code(result), "internal_error");
    }
}
