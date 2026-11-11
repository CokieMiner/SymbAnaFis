//! Sealed storage and construction machinery.

use crate::{
    collections::FixedState,
    errors::{FunctionError, SubstitutionError, SymbolError},
};

use super::ExprView;

mod expression;
mod functions;
mod interner;
mod numeric;
mod substitution;
mod symbols;

pub use expression::{
    AdditiveOperands, AlgebraicOperands, CompactStorage, ConcreteMatcher, ExactIdentity,
    ExactSources, Expr, ExprChildren, Facts, Forest, Mapped, OperandDemand, OperandMatch,
    Postorder, Properties, Property, SettledStage, StoredForm, Truth,
};
pub use functions::{
    BinaryRule, BuiltinFunction, BuiltinSet, Function, FunctionDefinition, UnaryRule, UserFunction,
};
pub use interner::Interner;
pub use numeric::{Kernel, NumericValue};
pub use substitution::{Substitution, Substitutions};
pub use symbols::{Context, Symbol, SymbolData};

#[cfg(test)]
pub use symbols::IdSequence;

#[cfg(test)]
mod tests;
