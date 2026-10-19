//! Borrowed numerical access to canonical stored expression data.

use core::num::NonZeroU64;

use super::{EvaluationView, Expr, Node};

impl<'expr> EvaluationView<'expr> {
    /// Borrow an immutable expression without constructing logical monomials.
    pub const fn new(expression: &'expr Expr) -> Self {
        Self { expression }
    }

    /// Borrow stored DAG edges, including polynomial coefficients and bases.
    pub fn children(self) -> impl Iterator<Item = &'expr Expr> {
        Node::children(self.expression)
    }

    /// Borrow stored additive operands without projecting compact monomials.
    pub fn sum(self) -> Option<&'expr [Expr]> {
        if let Node::Sum(terms) = self.expression.node() {
            Some(terms)
        } else {
            None
        }
    }

    /// Borrow canonically ordered sparse monomials with positive integer powers.
    pub fn monomials(
        self,
    ) -> Option<impl ExactSizeIterator<Item = (&'expr Expr, &'expr [(Expr, u32)])>> {
        if let Node::Poly(polynomial) = self.expression.node() {
            Some(
                polynomial
                    .terms
                    .iter()
                    .map(|term| (&term.coefficient, term.powers.as_slice())),
            )
        } else {
            None
        }
    }

    /// Recognize an exact multiplicative identity without discarding a float tier.
    pub fn exact_one(expression: &Expr) -> bool {
        matches!(expression.node(), Node::Number(value) if value.gaussian() == Some((1, 0)))
    }

    /// Read an exact positive integer when it fits a bounded power-chain count.
    /// Approximate and larger exponents retain the backend power operation.
    pub fn positive_integer(expression: &Expr) -> Option<NonZeroU64> {
        let (numerator, denominator) = expression.literal()?.exact_ratio()?;
        if denominator != 1 {
            return None;
        }
        u64::try_from(numerator).ok().and_then(NonZeroU64::new)
    }
}
