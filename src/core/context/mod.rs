//! Unified context module — symbol registry + user-defined functions.
//!
//! - [`api_user`] — public types: `Context`, `UserFunction`, `BodyFn`, `PartialFn`
//! - [`api_crate`] — crate-internal type aliases: `BodyFn`, `PartialFn`
//! - `logic/` — Implementation details

mod api_crate;
mod api_user;
mod logic;

pub use api_crate::*;
pub use api_user::*;
