//! Sealed storage and construction machinery.

use crate::{
    collections::FixedState,
    errors::{FunctionError, SubstitutionError, SymbolError},
};

use super::ExprView;

mod expression;
mod functions;
mod interner;
mod mutex;
mod numeric;
mod substitution;
mod symbols;

pub use expression::{
    AdditiveOperands, AlgebraicOperands, CompactStorage, ConcreteMatcher, ExactIdentity,
    ExactSources, Expr, ExprChildren, Forest, Mapped, OperandDemand, OperandMatch, Postorder,
    Properties, Property, StoredForm, Truth,
};
pub use functions::{
    BinaryRule, BuiltinFunction, BuiltinSet, Function, FunctionDefinition, UnaryRule, UserFunction,
};
pub use interner::Interner;
pub use mutex::Mutex;
pub use numeric::{EvaluationValue, Kernel, NumericValue, Scalar};
pub use substitution::{Substitution, Substitutions};
pub use symbols::{Context, SYMBOLS, Symbol, SymbolData};

#[cfg(test)]
pub use symbols::IdSequence;

#[cfg(test)]
mod tests;
