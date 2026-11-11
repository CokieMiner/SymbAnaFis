//! Text expressions resolved through scoped symbols and known function identities.

mod cache;
mod input;
mod lexer;

pub use cache::{Cache, PowerCache, slots_for};
pub use input::Value;
pub use lexer::{BinaryOp, Lexer, Token};

#[cfg(test)]
mod tests;
