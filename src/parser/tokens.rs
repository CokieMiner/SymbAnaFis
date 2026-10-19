//! Borrowed lexical tokens and scalar operator precedence.

use super::Span;

/// A binary scalar operation in the source grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

/// The numeric representation selected by the literal's source syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberKind {
    Integer,
    Float,
}

/// A source token retaining its UTF-8 byte range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token<'source> {
    Number(&'source str, Span, NumberKind),
    Name(&'source str, Span),
    Operator(BinaryOp, Span),
    Open(Span),
    Close(Span),
    Comma(Span),
}

impl Token<'_> {
    /// Return the token's half-open byte range.
    pub const fn span(self) -> Span {
        match self {
            Self::Number(_, span, _)
            | Self::Name(_, span)
            | Self::Operator(_, span)
            | Self::Open(span)
            | Self::Close(span)
            | Self::Comma(span) => span,
        }
    }
}

impl BinaryOp {
    /// Rank addition below multiplication and right-associative exponentiation.
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Add | Self::Subtract => 1,
            Self::Multiply | Self::Divide => 2,
            Self::Power => 4,
        }
    }
}
