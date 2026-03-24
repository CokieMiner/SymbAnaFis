//! Implementation details for the `expr` module.
//!
//! This folder contains all logic that is internal to the expression subsystem.
//! Nothing here is part of the public API — access goes through `expr::mod.rs`
//! and `expr::api`.

pub(super) mod analysis;
pub(super) mod constructors;
pub(super) mod hash;
pub(super) mod ordering;

// display is pub(in crate::core) so upper modules can wire the Display impl
pub(in crate::core) mod display;

#[cfg(test)]
mod tests;
