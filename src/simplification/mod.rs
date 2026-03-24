//! Simplification framework.
//!
//! Public entry points live in [`api`], while the implementation details live in
//! [`logic`].

mod api;
mod logic;

pub use api::{CustomBodyMap, simplify_expr};
pub use api::{Simplify, simplify};

pub(in crate::simplification) use logic::helpers;
