//! Core types for symbolic mathematics.
//!
//! Public entry points are declared in [`api_user`].
//! Implementation details live in the submodules.

mod api_crate;
mod api_user;
mod helpers;

mod context;
mod expr;
mod symbol;

pub use api_crate::*;
pub use api_user::*;
