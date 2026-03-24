//! Abstract Syntax Tree for mathematical expressions.
//!
//! - [`api_user`] — public types: `Expr`, `ExprKind`, `CustomEvalMap`
//! - [`api_crate`] — crate-internal: ID counter, cached constants, `arc_number`
//! - [`logic`] — implementation details (not accessible outside this module)

mod api_crate;
mod api_user;
mod logic;

pub use api_crate::*;
pub use api_user::*;
