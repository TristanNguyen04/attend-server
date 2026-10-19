//! # `capture`
//!
//! Rooms, NFC tags, class sessions and tap verification.
//!
//! - Postgres schema: `capture`
//! - Tables: `rooms`, `nfc_tags`, `class_sessions`, `scan_events`
//! - May depend on: `calendar`, `identity`, `courses`
//! - Public surface: [`api`] and [`routes`]. Everything else in this module is private,
//!   so other modules cannot reach it (the compiler enforces this).
//!
//! See `docs/architecture.md` for the module rules.

mod entities;

pub mod api;

use loco_rs::controller::Routes;

/// HTTP routes this module contributes to the application.
#[must_use]
pub fn routes() -> Vec<Routes> {
    Vec::new()
}
