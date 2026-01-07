//! User-facing API builders
//!
//! This module contains the builder pattern APIs:
//! - `Diff` - Differentiation builder
//! - `Simplify` - Simplification builder
//! - Helper functions (gradient, jacobian, hessian)

pub mod builder;
mod helpers;

pub use builder::{Diff, Simplify};
pub use helpers::{
    evaluate_str, gradient, gradient_str, hessian, hessian_str, jacobian, jacobian_str,
};
