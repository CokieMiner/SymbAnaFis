//! Unified context module — symbol registry + user-defined functions.
//!
//! - [`api_user`] — public types: `Context`, `UserFunction`, `BodyFn`, `PartialFn`
//! - [`api_crate`] — crate-internal type aliases: `BodyFn`, `PartialFn`
//! - `logic/` — Implementation details

mod api;
mod logic;

pub use api::*;
