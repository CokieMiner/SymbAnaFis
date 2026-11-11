//! Text expressions resolved through scoped symbols and known function identities.

mod input;
mod lexer;

pub use lexer::{BinaryOp, Lexer, Token};

#[cfg(test)]
mod tests;
