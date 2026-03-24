//! Internal parser implementation details.

pub(super) mod implicit_mul;
pub(super) mod lexer;
pub(super) mod pratt;
pub(super) mod tokens;

#[cfg(test)]
mod test;
