//! Commutative operand extraction, candidate generation, and transformation.

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactIdentity,
    ExactSources, Expr, FactorSide, Node, NumericValue, OperandDemand, OperandKind, OperandMatch,
    PartialSelection, Polynomial, Postorder, RemainingOperand, Selection, StoredForm, Substitution,
    Substitutions, WeightedOperand,
};

mod candidates;
mod linear;
mod operands;
mod power_matching;
mod projection;
mod subsets;
mod transform;

pub use linear::LinearOperands;
pub use power_matching::PowerProjection;
pub use subsets::{MatcherHints, OperandSource, SimplePolyFilter};

#[cfg(test)]
pub use transform::SubstitutionPass;
