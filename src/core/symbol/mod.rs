//! Symbol interning and ergonomic expression building.
//!
//! - [`api_user`] — public entry points for library consumers
//! - [`api_crate`] — crate-internal helpers (`InternedSymbol`, `symb_interned`, …)
//! - [`logic`] — implementation details (not accessible outside this module)

mod api_crate;
mod api_user;
mod logic;

pub use api_crate::*;
pub use api_user::*;
