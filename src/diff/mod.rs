//! Differentiation engine - symbolic derivative computation
//!
//! This module provides the core differentiation logic, implementing:
//! - Standard calculus rules (sum, product, quotient, chain)
//! - Logarithmic differentiation for variable exponents (e.g., x^x)
//! - Custom function derivative support
//! - Mixed partial derivatives (∂²f/∂x∂y)
//! - O(N) polynomial differentiation
//!
//! The main entry point is [`Expr::derive()`](crate::Expr::derive),
//! typically called via the [`Diff`](crate::Diff) builder API.

mod api;
mod logic;

pub use api::{Diff, diff};
