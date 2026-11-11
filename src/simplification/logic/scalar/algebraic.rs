//! The direct Algebraic identities: cancellation of finite, proved nonzero
//! scalar factors and of certified exact polynomial GCDs, absolute values with
//! a proved sign, and addition of finite fractions with the same zero
//! exclusions.

#![expect(
    unsafe_code,
    reason = "a factor's position indexes the count list built from the same factor list"
)]

use core::{
    iter::once,
    ops::{Add, Div, Mul, Neg},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::{
    core::{BuiltinFunction, ExactIdentity, Expr, ExprView, Property, StoredForm, Truth},
    errors::SimplifyError,
    polynomial::{PolynomialGcd, PolynomialInput, PolynomialWork},
};

use super::{Domain, DomainSafety};

/// Namespace for direct algebraic cancellation, absolute-value, and fraction identities.
pub struct Algebraic;

impl Algebraic {
    /// $\lvert a\rvert = a$ for nonnegative and $-a$ for nonpositive finite
    /// $a$. Otherwise a manifest negative coefficient leaves the argument, and
    /// $\lvert b^n\rvert = \lvert b\rvert^n$ for a positive integer $n$.
    pub fn absolute(expression: &Expr) -> Option<Expr> {
        let ExprView::Function {
            name,
            args: [argument],
        } = expression.view()
        else {
            return None;
        };
        if *name != BuiltinFunction::Abs.function() {
            return None;
        }
        let facts = argument.properties();
        if facts.get(Property::Finite) != Truth::True {
            return None;
        }
        if facts.get(Property::Nonnegative) == Truth::True {
            return Some(argument.clone());
        }
        if facts.get(Property::Nonpositive) == Truth::True {
            return Some(Neg::neg(argument));
        }
        if let Some(positive) = Self::positive_argument(argument) {
            return Some(positive.abs());
        }
        // Property analysis proves a power finite only from a finite base.
        if let ExprView::Pow(base, exponent) = argument.view()
            && let Some((count, 1)) = exponent.exact_ratio()
            && count > 0
        {
            return Some(base.abs().pow(exponent));
        }
        None
    }

    /// Cancel only factors proved nonzero; \(a/a=1\) never removes an unknown pole.
    pub fn cancel(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        match expression.view() {
            ExprView::Product(factors) => {
                let zero = factors
                    .iter()
                    .find(|factor| factor.exact_ratio() == Some((0, 1)))?;
                factors
                    .iter()
                    .all(|factor| Domain::admits(factor.properties().get(Property::Finite), safety))
                    .then(|| zero.clone())
            }
            ExprView::Pow(base, exponent) => {
                if exponent.exact_ratio() == Some((0, 1))
                    && Domain::admits(base.properties().get(Property::Nonzero), safety)
                {
                    Some(Expr::number(1))
                } else if (base.is_one_num()
                    && Domain::admits(exponent.properties().get(Property::Finite), safety))
                    || (base.exact_ratio() == Some((0, 1))
                        && exponent.as_number().is_none()
                        && exponent.properties().get(Property::Positive) == Truth::True)
                {
                    Some(base.clone())
                } else {
                    None
                }
            }
            ExprView::Div(numerator, denominator) => {
                if numerator.exact_ratio() == Some((0, 1))
                    && Domain::admits(denominator.properties().get(Property::Nonzero), safety)
                {
                    return Some(numerator.clone());
                }
                if let ExprView::Div(inner, divisor) = denominator.view()
                    && Domain::admits(divisor.properties().get(Property::Nonzero), safety)
                    && numerator.properties().get(Property::Finite) == Truth::True
                    && inner.properties().get(Property::Finite) == Truth::True
                {
                    return Some(Div::div(Mul::mul(numerator, divisor), inner));
                }
                Self::cancel_factors(numerator, denominator, safety)
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Derivative { .. } => None,
        }
    }

    /// Remove the common counts of equal bases from the manifest positive
    /// exact integer powers of a quotient; other factors count once.
    ///
    /// Both factor lists are read in place. Their counts are copied out at the
    /// first admitted common base, so a quotient without one allocates and
    /// clones nothing, and a factor whose count is unchanged is kept as it is.
    fn cancel_factors(numerator: &Expr, denominator: &Expr, safety: DomainSafety) -> Option<Expr> {
        let [top, bottom] = [numerator, denominator].map(|side| {
            if let ExprView::Product(factors) = side.view() {
                factors
            } else {
                from_ref(side)
            }
        });
        let mut counts: Option<(Vec<i64>, Vec<i64>)> = None;
        for (index, factor) in top.iter().enumerate() {
            let base = factor.positive_factor().map_or(factor, |(base, _)| base);
            // Literal coefficient promotion belongs to the canonical numeric kernel.
            if base.as_number().is_some() {
                continue;
            }
            let Some(position) = bottom.iter().position(|other| {
                other
                    .positive_factor()
                    .map_or(other, |(candidate, _)| candidate)
                    == base
            }) else {
                continue;
            };
            if !Domain::admits(base.properties().get(Property::Nonzero), safety) {
                continue;
            }
            let (tops, bottoms) = counts.get_or_insert_with(|| {
                let count =
                    |operand: &Expr| operand.positive_factor().map_or(1, |(_, count)| count);
                (
                    top.iter().map(count).collect(),
                    bottom.iter().map(count).collect(),
                )
            });
            // SAFETY: `tops` holds one count per factor of `top`, which `index`
            // enumerates, and `bottoms` one per factor of `bottom`, in which
            // `position` was found.
            let (remaining, available) = unsafe {
                (
                    tops.get_unchecked_mut(index),
                    bottoms.get_unchecked_mut(position),
                )
            };
            let common = (*remaining).min(*available);
            // Manifest counts are positive; their minimum cannot exceed either.
            *remaining = remaining.saturating_sub(common);
            *available = available.saturating_sub(common);
        }
        // The first admitted common base removes at least one unit, since
        // every count starts positive.
        let (tops, bottoms) = counts?;
        let product = |factors: &[Expr], kept: Vec<i64>| {
            Expr::product(
                factors
                    .iter()
                    .zip(kept)
                    .filter_map(|(factor, count)| {
                        let (base, original) = factor.positive_factor().unwrap_or((factor, 1));
                        match count {
                            0 => None,
                            _ if count == original => Some(factor.clone()),
                            1 => Some(base.clone()),
                            _ => Some(base.pow(count)),
                        }
                    })
                    .collect(),
            )
        };
        Some(Div::div(product(top, tops), product(bottom, bottoms)))
    }

    /// Cancel a verified exact polynomial GCD. Preserving mode additionally
    /// proves that the removed expression is nonzero, retaining every pole.
    pub fn polynomial_cancellation(
        expression: &Expr,
        safety: DomainSafety,
        work: &mut PolynomialWork,
    ) -> Result<Option<Expr>, SimplifyError> {
        let ExprView::Div(numerator, denominator) = expression.view() else {
            return Ok(None);
        };
        // A side is polynomial work only when it holds a sum.
        let summed = |side: &Expr| match side.view() {
            ExprView::Sum(_) => true,
            ExprView::Pow(base, _) => matches!(base.view(), ExprView::Sum(_)),
            ExprView::Product(factors) => factors
                .iter()
                .any(|factor| matches!(factor.view(), ExprView::Sum(_))),
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Div(..)
            | ExprView::Derivative { .. } => false,
        };
        if !summed(numerator) && !summed(denominator) {
            return Ok(None);
        }
        if !Domain::admits(numerator.properties().get(Property::Finite), safety)
            || !Domain::admits(denominator.properties().get(Property::Finite), safety)
        {
            return Ok(None);
        }
        let Some((top, bottom, bases)) = PolynomialInput::pair(numerator, denominator, work)?
        else {
            return Ok(None);
        };
        if top.is_zero() || bottom.is_zero() || top.is_constant() || bottom.is_constant() {
            return Ok(None);
        }
        // Symbols carry no assumptions, so a nonconstant polynomial in symbols
        // alone has a zero and no nonzero proof exists.
        if safety == DomainSafety::Preserving
            && bases
                .iter()
                .all(|base| matches!(base.view(), ExprView::Symbol(_)))
        {
            return Ok(None);
        }
        let Some(result) = PolynomialGcd::cancel(&top, &bottom, work)? else {
            return Ok(None);
        };
        if result.common.is_constant()
            || !result.common.to_expr(&bases).is_some_and(|common| {
                Domain::admits(common.properties().get(Property::Nonzero), safety)
            })
        {
            return Ok(None);
        }
        let (Some(reduced_top), Some(reduced_bottom)) = (
            result.numerator.to_expr(&bases),
            result.denominator.to_expr(&bases),
        ) else {
            return Ok(None);
        };
        let output = Div::div(reduced_top, reduced_bottom);
        Ok((output != *expression).then_some(output))
    }

    /// Combine two fractions without cancelling any symbolic denominator factor.
    /// \(a/b+c/d=(ad+cb)/(bd)\); equal denominators retain one copy.
    pub fn combine_fractions(expression: &Expr) -> Option<Expr> {
        let ExprView::Sum(children) = expression.view() else {
            return None;
        };
        // A monomial over scalar-leaf bases is a coefficient times positive
        // powers, never a quotient, so such storage is not projected.
        if expression.simple_bases() == Some(true) {
            return None;
        }
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
                .find(|(index, term)| {
                    *index != first_index && term.properties().get(Property::Finite) == Truth::True
                })
                .map(|(index, term)| (index, ((*term).clone(), Expr::number(1))))
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
                .filter(|(index, _)| *index != first_index && *index != second_index)
                .map(|(_, term)| term.clone())
                .chain(once(combined))
                .collect(),
        ))
    }

    /// A finite quotient, optionally scaled by a finite literal, as its scaled
    /// numerator and its denominator.
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
        if numerator.properties().get(Property::Finite) != Truth::True
            || denominator.properties().get(Property::Finite) != Truth::True
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

    /// The product with its manifest negative leading coefficient negated.
    pub fn positive_argument(argument: &Expr) -> Option<Expr> {
        let ExprView::Product(factors) = argument.view() else {
            return None;
        };
        let first = factors.first()?;
        if !first.as_number().is_some_and(Number::is_negative) {
            return None;
        }
        let positive = Neg::neg(first);
        // Native minimum-integer negation wraps; it cannot establish a positive prefix.
        if !positive.as_number().is_some_and(Number::is_positive) {
            return None;
        }
        Some(Expr::product(
            once(positive)
                .chain(factors.iter().skip(1).cloned())
                .collect(),
        ))
    }
}
