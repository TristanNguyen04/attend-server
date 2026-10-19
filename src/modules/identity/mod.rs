//! # `identity`
//!
//! Accounts, login, device binding and rebinding.
//!
//! - Postgres schema: `identity`
//! - Tables: `users`, `devices`, `device_lifecycle_items`
//! - May depend on: `calendar`
//! - Public surface: [`api`] and [`routes`]. Everything else in this module is private,
//!   so other modules cannot reach it (the compiler enforces this).
//!
//! Code is grouped by feature, one folder each:
//! - `account/`: the `users` table (lookups and account rules) and `GET /api/me`.
//! - `auth/`: signing in (`POST /api/auth/sign-in`) and proving who you are (access tokens and
//!   the `Authenticated` extractor).
//! - `errors.rs`: every error this module returns, with its stable code and message.
//! - `entities/`: generated from the migrated database; never edited by hand.
//!
//! See `docs/architecture.md` for the module rules.

mod account;
mod auth;
mod entities;
mod errors;
#[cfg(test)]
mod test_support;

pub mod api;

use loco_rs::controller::Routes;

/// HTTP routes this module contributes to the application.
#[must_use]
pub fn routes() -> Vec<Routes> {
    vec![auth::routes(), account::routes()]
}

#[cfg(test)]
mod tests {
    #[test]
    fn contributes_the_sign_in_and_profile_routes() {
        assert_eq!(super::routes().len(), 2);
    }
}
