//! Parser module.
//!
//! Public entry points live in [`api`], while the implementation details live in
//! [`logic`].

mod api;
mod logic;

pub use api::parse;
