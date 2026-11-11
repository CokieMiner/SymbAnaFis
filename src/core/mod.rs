//! Immutable, globally shared symbolic expressions and registry-owned symbols.

mod api;
mod logic;

pub use api::ExprView;
pub use logic::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, BuiltinSet, CompactStorage,
    ConcreteMatcher, Context, ExactIdentity, ExactSources, Expr, ExprChildren, Forest, Function,
    FunctionDefinition, Kernel, Mapped, OperandDemand, OperandMatch, Postorder, Properties,
    Property, SettledStage, StoredForm, Substitution, Substitutions, Symbol, SymbolData, Truth,
    UserFunction,
};
