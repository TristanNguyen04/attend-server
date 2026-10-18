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
//! See `docs/architecture.md` for the module rules.

mod controllers;
mod dtos;
mod entities;
mod handlers;
mod models;
mod workers;

pub mod api;

use loco_rs::controller::Routes;

/// HTTP routes this module contributes to the application.
#[must_use]
pub fn routes() -> Vec<Routes> {
    controllers::routes()
}
