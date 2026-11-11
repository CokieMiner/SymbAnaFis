//! Canonical form construction, literal normalization, and structural ordering.

use super::{
    BuiltinFunction, ExactIdentity, Expr, FunctionDefinition, Interner, MonomialStatus, NODES,
    Node, NumericValue, Polynomial, Postorder, Stored, Symbol, SymbolData, UnaryRule,
};

mod constructors;
mod identity;
mod normalize;
mod operators;
mod ordering;
mod powers;
mod terms;
