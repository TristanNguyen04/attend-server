//! `attend_server`: the attendance backend, a modular monolith built on Loco.
//!
//! - [`modules`]: the business modules (`calendar`, `identity`, `courses`, `capture`, `records`).
//! - [`shared_lib`]: small, stable code every module may use (events, clock, error envelope).
//! - [`app`]: the Loco application that wires the modules together.

pub mod app;
pub mod initializers;
pub mod modules;
pub mod shared_lib;
pub mod tasks;
