//! Internal parser implementation details.

mod implicit_mul;
mod lexer;
mod pratt;
mod tokens;

pub(super) use implicit_mul::insert_implicit_multiplication;
pub(super) use lexer::{balance_parentheses, lex};
pub(super) use pratt::parse_expression;

#[cfg(test)]
mod test;
