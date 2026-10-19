//! Sealed storage and construction machinery.

use foldhash::fast::FixedState;

use crate::errors::{FunctionError, RewriteError, SubstitutionError, SymbolError};

use super::ExprView;

mod builtin;
mod context;
mod expression;
mod ids;
mod interner;
mod mutex;
mod names;
mod numeric;
mod pattern;
mod registry;
mod scalar;
mod substitution;
mod symbol;
mod symbols;
mod user_function;

pub use builtin::BuiltinFunction;
pub use context::Context;
pub use expression::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, EvaluationView, ExactSources, Expr,
    ExprChildren, OperandDemand, OperandMatch, Properties, Property, Truth,
};
pub use ids::IdSequence;
pub use interner::Interner;
pub use mutex::Mutex;
pub use names::FunctionName;
pub use numeric::NumericValue;
pub use pattern::{Pattern, Replacement, RewriteRule};
pub use registry::{BinaryRule, FunctionDefinition, UnaryRule};
pub use scalar::{EvaluationValue, Kernel, Scalar};
pub use substitution::{Substitution, Substitutions};
pub use symbol::Symbol;
pub use symbols::{SYMBOLS, SymbolData};
pub use user_function::UserFunction;

#[cfg(test)]
mod tests;
