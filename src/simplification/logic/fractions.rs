//! Addition of finite scalar fractions with the same zero exclusions.

use core::{
    iter::once,
    ops::{Add, Div, Mul},
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::core::{Expr, ExprView, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    /// Combine two fractions without cancelling any symbolic denominator factor.
    /// \(a/b+c/d=(ad+cb)/(bd)\); equal denominators retain one copy.
    pub fn combine_fractions(expression: &Expr) -> Option<Expr> {
        let ExprView::Sum(children) = expression.view() else {
            return None;
        };
        let terms = children.collect::<Vec<_>>();
        let mut selected = terms
            .iter()
            .enumerate()
            .filter_map(|(index, term)| Self::fraction(term).map(|parts| (index, parts)));
        let (first_index, (first_top, first_bottom)) = selected.next()?;
        let second = selected.next().or_else(|| {
            terms
                .iter()
                .enumerate()
                .find(|(index, term)| *index != first_index && Self::proved(term, Property::Finite))
                .map(|(index, term)| (index, (term.clone(), Expr::number(1))))
        })?;
        let (second_index, (second_top, second_bottom)) = second;
        let combined = if first_bottom == second_bottom {
            Div::div(Add::add(first_top, second_top), first_bottom)
        } else {
            Div::div(
                Add::add(
                    Mul::mul(first_top, &second_bottom),
                    Mul::mul(second_top, &first_bottom),
                ),
                Mul::mul(first_bottom, second_bottom),
            )
        };
        Some(Expr::sum(
            terms
                .into_iter()
                .enumerate()
                .filter_map(|(index, term)| {
                    (index != first_index && index != second_index).then_some(term)
                })
                .chain(once(combined))
                .collect(),
        ))
    }

    fn fraction(expression: &Expr) -> Option<(Expr, Expr)> {
        let (weight, quotient) = if let ExprView::Product(factors) = expression.view()
            && let [coefficient, quotient] = factors
            && coefficient.as_number().is_some_and(Number::is_finite)
        {
            (Some(coefficient), quotient)
        } else {
            (None, expression)
        };
        let ExprView::Div(numerator, denominator) = quotient.view() else {
            return None;
        };
        if !Self::proved(numerator, Property::Finite)
            || !Self::proved(denominator, Property::Finite)
        {
            return None;
        }
        Some((
            weight.map_or_else(
                || numerator.clone(),
                |coefficient| Expr::product(vec![coefficient.clone(), numerator.clone()]),
            ),
            denominator.clone(),
        ))
    }
}
