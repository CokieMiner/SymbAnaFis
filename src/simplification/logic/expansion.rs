//! Bounded distributivity and positive exact integer polynomial powers.

use core::ops::Mul;

use alloc::{vec, vec::Vec};

use crate::{
    core::{Expr, ExprView},
    errors::SimplifyError,
};

use super::PreparedStep;

impl PreparedStep<'_> {
    /// Expand products of sums and positive exact integer powers of sums.
    /// The distributive identity is \((a+b)c=ac+bc\).
    /// All generated Cartesian products reserve their work before materialization.
    pub fn expand(
        expression: &Expr,
        remaining: &mut usize,
        limit: usize,
    ) -> Result<Option<Expr>, SimplifyError> {
        match expression.view() {
            ExprView::Product([first, factors @ ..])
                if Self::is_sum(first) || factors.iter().any(Self::is_sum) =>
            {
                let mut result = first.clone();
                for factor in factors {
                    result = Self::multiply(&result, factor, remaining, limit)?;
                }
                Ok(Some(result))
            }
            ExprView::Pow(base, exponent) if Self::is_sum(base) => {
                let Some(mut count) = Self::exact_integer(exponent).filter(|count| *count > 1)
                else {
                    return Ok(None);
                };
                let mut power = base.clone();
                // A positive exponent has a lowest set bit. Start there to
                // avoid a Cartesian product with an allocated unit expression.
                while count & 1 == 0 {
                    power = Self::multiply(&power, &power, remaining, limit)?;
                    count >>= 1;
                }
                let mut result = power.clone();
                count >>= 1;
                while count != 0 {
                    power = Self::multiply(&power, &power, remaining, limit)?;
                    if count & 1 != 0 {
                        result = Self::multiply(&result, &power, remaining, limit)?;
                    }
                    count >>= 1;
                }
                Ok(Some(result))
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(..)
            | ExprView::Pow(..)
            | ExprView::Derivative { .. } => Ok(None),
        }
    }

    fn multiply(
        left: &Expr,
        right: &Expr,
        remaining: &mut usize,
        limit: usize,
    ) -> Result<Expr, SimplifyError> {
        if !Self::is_sum(left) && !Self::is_sum(right) {
            return Ok(Mul::mul(left, right));
        }
        let left_terms = Self::terms(left);
        let right_terms = Self::terms(right);
        let count = left_terms
            .len()
            .checked_mul(right_terms.len())
            .ok_or(SimplifyError::ExpansionLimitExceeded { limit })?;
        *remaining = remaining
            .checked_sub(count)
            .ok_or(SimplifyError::ExpansionLimitExceeded { limit })?;
        let mut products = Vec::with_capacity(count);
        for first in &left_terms {
            for second in &right_terms {
                products.push(Mul::mul(first, second));
            }
        }
        Ok(Expr::sum(products))
    }

    fn terms(expression: &Expr) -> Vec<Expr> {
        if Self::is_sum(expression) {
            expression.children().collect()
        } else {
            vec![expression.clone()]
        }
    }

    fn is_sum(expression: &Expr) -> bool {
        matches!(expression.view(), ExprView::Sum(_))
    }
}
