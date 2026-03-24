//! Mathematical function evaluations
//!
//! This module centralizes all mathematical function implementations,
//! organized by category for maintainability.
//!
//! # Academic References
//!
//! Implementations follow standard numerical methods from:
//!
//! - **DLMF**: NIST Digital Library of Mathematical Functions <https://dlmf.nist.gov>
//! - **A&S**: Abramowitz & Stegun, "Handbook of Mathematical Functions" (1964)
//! - **NR**: Press et al., "Numerical Recipes" (3rd ed., 2007)
//! - Lanczos, C. "A Precision Approximation of the Gamma Function" (1964)
//! - Corless et al. "On the Lambert W Function" (1996)
//!
//! # Domain Validation
//!
//! Functions that can produce undefined results (poles, branch cuts, domain errors)
//! return `Option<T>` and check their inputs. Domain errors produce `Some(NaN)`
//! so invalid values propagate consistently through arithmetic. Known poles return
//! `Some(±infinity)`. For IEEE-754-style consistency, invalid integer indices also
//! return `Some(NaN)`.
//! Key validations include:
//!
//! - **Gamma functions**: Non-positive integers are poles
//! - **Zeta function**: s=1 is a pole
//! - **Logarithms**: Non-positive inputs are domain errors
//! - **Inverse trig**: |x| > 1 is a domain error for asin/acos
//! - **Square root**: Negative inputs return NaN
//! - **Poles**: Return infinity when the real-valued limit diverges

mod api;
mod logic;

pub use api::*;
