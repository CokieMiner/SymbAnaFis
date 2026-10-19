//! Builtin passes and adapters for the core's existing transformation API.

use core::fmt::{Debug, Formatter, Result as FmtResult};

use alloc::sync::Arc;

use crate::core::{Expr, RewriteRule};

type NodeTransform = dyn Fn(&Expr) -> Option<Expr> + Send + Sync;

/// One complete DAG pass in an ordered simplification sequence.
///
/// Builtin passes preserve domains by default. [`super::DomainSafety`] controls
/// whether additional caller-assumed domain and branch hypotheses are permitted.
/// User rules and callbacks retain the core's caller-owned equivalence contract.
#[derive(Clone)]
#[non_exhaustive]
pub enum SimplificationStep {
    /// Guarded scalar and exact polynomial cancellation, signs, and fractions.
    Algebraic,
    /// Pythagorean, angle, parity, period, inverse, and quotient identities.
    Trigonometric,
    /// Hyperbolic identities, angle formulas, and exponential representations.
    Hyperbolic,
    /// Principal logarithm and exponential identities with conservative guards.
    Exponential,
    /// Principal square roots and real cube roots on proved domains.
    Roots,
    /// Distribute products and positive exact integer powers over sums.
    Expand,
    /// Extract common factors and recognize squares, cubes, and differences of squares.
    Factor,
    /// Select native numerical intrinsics and common factors for compilation.
    Compilation,
    /// Apply an existing core rewrite once; use capture predicates for guards.
    Rewrite(RewriteRule),
    /// Apply a pure callback bottom-up to each distinct logical input node.
    /// Return `None` to retain the rebuilt node. Inserted nodes are not revisited.
    /// Callback panics propagate; equal input nodes must produce equal results.
    Map(Arc<NodeTransform>),
}

impl From<RewriteRule> for SimplificationStep {
    fn from(rule: RewriteRule) -> Self {
        Self::Rewrite(rule)
    }
}

impl Debug for SimplificationStep {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Algebraic => formatter.write_str("Algebraic"),
            Self::Trigonometric => formatter.write_str("Trigonometric"),
            Self::Hyperbolic => formatter.write_str("Hyperbolic"),
            Self::Exponential => formatter.write_str("Exponential"),
            Self::Roots => formatter.write_str("Roots"),
            Self::Expand => formatter.write_str("Expand"),
            Self::Factor => formatter.write_str("Factor"),
            Self::Compilation => formatter.write_str("Compilation"),
            Self::Rewrite(rule) => formatter.debug_tuple("Rewrite").field(rule).finish(),
            Self::Map(_) => formatter.write_str("Map(<callback>)"),
        }
    }
}
