//! # `records`
//!
//! Attendance records, their history, manual changes and appeals.
//!
//! - Postgres schema: `records`
//! - Tables: `attendance_records`, `attendance_record_lifecycle_items`, `appeals`, `appeal_lifecycle_items`
//! - May depend on: `calendar`, `identity`, `courses`, `capture`
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
