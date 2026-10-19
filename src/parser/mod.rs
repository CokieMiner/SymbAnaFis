//! Text expressions resolved through scoped symbols and known function identities.

use crate::{
    core::{Context, Expr, Function},
    errors::{ParseError, Span},
};

mod input;
mod lexer;
mod tokens;

pub use lexer::Lexer;
pub use tokens::{BinaryOp, NumberKind, Token};

#[cfg(test)]
mod tests;
