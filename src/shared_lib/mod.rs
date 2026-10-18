//! Code shared by every module.
//!
//! Rules (see `docs/architecture.md`):
//! 1. It depends on **no module**. Every module may depend on it.
//! 2. No business rules: only small, stable, domain-neutral helpers and contracts.
//! 3. If only two modules need something, put it in the upstream module's `api` instead.
//! 4. Changes here affect everyone, so they need review from every team.

pub mod clock;
pub mod error;
pub mod events;
pub mod policies;
