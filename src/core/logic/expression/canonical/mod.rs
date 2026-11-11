//! Canonical form construction, literal normalization, and structural ordering.

use super::{ExactIdentity, Expr, Node, NumericValue, Polynomial, SYMBOLS, Symbol, SymbolData};

mod constructors;
mod identity;
mod normalize;
mod operators;
mod ordering;
