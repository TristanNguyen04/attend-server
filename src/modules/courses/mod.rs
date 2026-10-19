//! # `courses`
//!
//! Courses, rosters and the course attendance policies.
//!
//! - Postgres schema: `courses`
//! - Tables: `courses`, `course_members`
//! - May depend on: `calendar`, `identity`
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
