//! Compiled, guarded mathematical rule families.

use super::{DomainSafety, PreparedRule, PreparedStep, SimplificationStep};

mod algebraic;
mod assumptions;
mod compilation;
mod conditions;
mod exponential;
mod factoring;
mod hyperbolic;
mod registry;
mod roots;
mod trigonometric;
