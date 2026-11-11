//! Commutative operand extraction, candidate generation, and transformation.

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactIdentity,
    ExactSources, Expr, FactorSide, FunctionDefinition, Node, NumericValue, OperandDemand,
    OperandMatch, PartialSelection, Postorder, RemainingOperand, Selection, StoredForm,
    Substitution, Substitutions, WeightedOperand,
};

mod candidates;
mod linear;
mod operands;
mod power_matching;
mod powers;
mod projection;
mod subsets;
mod terms;
mod transform;

pub use linear::LinearOperands;
pub use power_matching::PowerProjection;
pub use subsets::OperandSource;

#[cfg(test)]
pub use transform::SubstitutionPass;
