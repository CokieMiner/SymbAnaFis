//! Domain-preserving cancellation of finite, proved nonzero scalar factors.

use core::ops::{Div, Mul};

use alloc::vec::Vec;

use num_anafis::Real;

use crate::core::{Expr, ExprView, Property, Truth};

use super::{DomainSafety, PreparedStep};

impl PreparedStep<'_> {
    /// Cancel only factors proved nonzero; \(a/a=1\) never removes an unknown pole.
    pub fn cancel(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        match expression.view() {
            ExprView::Product(factors) => {
                let zero = factors
                    .iter()
                    .find(|factor| Self::exact_integer(factor) == Some(0))?;
                factors
                    .iter()
                    .all(|factor| Self::permitted(factor, Property::Finite, safety))
                    .then(|| zero.clone())
            }
            ExprView::Pow(base, exponent) => {
                if Self::exact_integer(exponent) == Some(0)
                    && Self::permitted(base, Property::Nonzero, safety)
                {
                    Some(Expr::number(1))
                } else if (base.is_one_num() && Self::permitted(exponent, Property::Finite, safety))
                    || (Self::exact_integer(base) == Some(0)
                        && exponent.as_number().is_none()
                        && Self::proved(exponent, Property::Positive))
                {
                    Some(base.clone())
                } else {
                    None
                }
            }
            ExprView::Div(numerator, denominator) => {
                if Self::exact_integer(numerator) == Some(0)
                    && Self::permitted(denominator, Property::Nonzero, safety)
                {
                    return Some(numerator.clone());
                }
                if let ExprView::Div(inner, divisor) = denominator.view()
                    && Self::permitted(divisor, Property::Nonzero, safety)
                    && Self::proved(numerator, Property::Finite)
                    && Self::proved(inner, Property::Finite)
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

    fn cancel_factors(numerator: &Expr, denominator: &Expr, safety: DomainSafety) -> Option<Expr> {
        let mut top = Self::factors(numerator);
        let mut bottom = Self::factors(denominator);
        let mut changed = false;
        for (base, count) in &mut top {
            // Literal coefficient promotion belongs to the canonical numeric kernel.
            if base.as_number().is_some() {
                continue;
            }
            let Some((_, other)) = bottom.iter_mut().find(|(candidate, _)| candidate == base)
            else {
                continue;
            };
            if !Self::permitted(base, Property::Nonzero, safety) {
                continue;
            }
            let common = (*count).min(*other);
            // Manifest counts are positive; their minimum cannot exceed either.
            *count = count.saturating_sub(common);
            *other = other.saturating_sub(common);
            changed |= common != 0;
        }
        changed.then(|| Div::div(Self::reconstruct(top), Self::reconstruct(bottom)))
    }

    /// Decompose manifest positive exact integer powers, retaining all other factors.
    pub fn factors(expression: &Expr) -> Vec<(Expr, i64)> {
        if let ExprView::Product(operands) = expression.view() {
            operands.iter().map(Self::factor_power).collect()
        } else {
            alloc::vec![Self::factor_power(expression)]
        }
    }

    pub fn reconstruct(factors: Vec<(Expr, i64)>) -> Expr {
        Expr::product(
            factors
                .into_iter()
                .filter_map(|(base, count)| match count {
                    0 => None,
                    1 => Some(base),
                    _ => Some(base.pow(count)),
                })
                .collect(),
        )
    }

    fn factor_power(expression: &Expr) -> (Expr, i64) {
        if let ExprView::Pow(base, exponent) = expression.view()
            && let Some(count) = Self::exact_integer(exponent).filter(|count| *count > 0)
        {
            (base.clone(), count)
        } else {
            (expression.clone(), 1)
        }
    }

    /// Recognize native exact integer representations without projecting floats.
    pub fn exact_integer(expression: &Expr) -> Option<i64> {
        Self::exact_ratio(expression)
            .filter(|(_, denominator)| *denominator == 1)
            .map(|(numerator, _)| numerator)
    }

    pub fn exact_ratio(expression: &Expr) -> Option<(i64, u64)> {
        let parts = expression.as_number()?.extract_complex().ok()?;
        if parts.im.is_float() || !parts.im.is_zero() {
            return None;
        }
        match parts.re {
            Real::Int(numerator) => Some((numerator, 1)),
            Real::Rational(ratio) => Some((*ratio.numer(), *ratio.denom())),
            Real::Float(_) | _ => None,
        }
    }

    pub fn proved(expression: &Expr, property: Property) -> bool {
        expression.properties().get(property) == Truth::True
    }

    /// A false proof never becomes an assumption; unknown facts require opt-in.
    pub fn permitted(expression: &Expr, property: Property, safety: DomainSafety) -> bool {
        match expression.properties().get(property) {
            Truth::True => true,
            Truth::False => false,
            Truth::Unknown => safety == DomainSafety::AssumptionDependent,
        }
    }
}
