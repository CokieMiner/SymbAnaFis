//! Uncertainty propagation for symbolic expressions
//!
//! Computes uncertainty propagation formulas using the standard GUM formula:
//! `σ_f²` = Σᵢ Σⱼ (∂f/∂xᵢ)(∂f/∂xⱼ) Cov(xᵢ, xⱼ)
//!
//! For uncorrelated variables, this simplifies to:
//! `σ_f²` = Σᵢ (∂f/∂xᵢ)² σᵢ²
//!
//! # Reference
//!
//! JCGM 100:2008 "Evaluation of measurement data — Guide to the expression
//! of uncertainty in measurement" (GUM), Section 5.1.2
//! <https://www.bipm.org/documents/20126/2071204/JCGM_100_2008_E.pdf>

pub mod api;
mod logic;

// Public re-exports
pub use api::*;
