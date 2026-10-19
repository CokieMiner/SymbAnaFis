//! Principal-power identities with explicit nonzero and branch predicates.

use core::{
    iter::once,
    ops::{Add, Div, Mul, Sub},
};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType};

use crate::{
    collections::HashMap,
    core::{BuiltinFunction, Expr, ExprView, Property},
};

use super::{DomainSafety, PreparedStep};

/// References into a disjoint mutable flag iterator identify accepted factors
/// without unchecked indices. A total is present exactly when a factor was kept.
struct PowerGroup<'factor> {
    total: Option<Expr>,
    nonzero: Option<bool>,
    accepted: Vec<&'factor mut bool>,
}

impl PreparedStep<'_> {
    /// Combine powers only when their logarithmic definitions have the same branch.
    pub fn combine_powers(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        match expression.view() {
            ExprView::Product(factors) => Self::combine_product(factors, safety)
                .or_else(|| Self::common_exponent_product(factors, safety)),
            ExprView::Div(top, bottom) => {
                if !matches!(top.view(), ExprView::Pow(..))
                    && !matches!(bottom.view(), ExprView::Pow(..))
                {
                    return Self::common_exponent_quotient(top, bottom, safety);
                }
                let (base, numerator) = Self::power_or_unit(top);
                let (other, denominator) = Self::power_or_unit(bottom);
                if base != other {
                    return Self::common_exponent_quotient(top, bottom, safety);
                }
                if Self::permitted(base, Property::Nonzero, safety)
                    && Self::proved(&numerator, Property::Finite)
                    && Self::proved(&denominator, Property::Finite)
                {
                    Self::exponent_difference(&numerator, &denominator)
                        .map(|exponent| base.pow(exponent))
                } else {
                    None
                }
            }
            ExprView::Pow(inner, outer) => {
                if let Some((base, exponent)) = Self::power(inner)
                    && let Some(count) = Self::exact_integer(exponent)
                    && count > 0
                    && count % 2 == 0
                    && let Some((numerator, denominator)) = Self::exact_ratio(outer)
                    && numerator > 0
                    && denominator > 1
                    && Self::permitted(base, Property::Real, safety)
                {
                    // \((x^{2n})^r=|x|^{2nr}\) for real x and positive r.
                    // Reduce before multiplying so native overflow retains nesting.
                    let mut common = count.unsigned_abs();
                    let mut divisor = denominator;
                    while let Some(remainder) = common.checked_rem(divisor) {
                        common = divisor;
                        divisor = remainder;
                    }
                    let scale = count.checked_div(i64::try_from(common).ok()?)?;
                    let total = numerator.checked_mul(scale)?;
                    let power = Number::from_rational(RationalType::from_parts(
                        total,
                        denominator.checked_div(common)?,
                    ));
                    return Some(base.abs().pow(power));
                }
                if let Some((base, exponent)) = Self::power(inner)
                    && ((Self::permitted(base, Property::Positive, safety)
                        && Self::permitted(exponent, Property::Real, safety)
                        && Self::permitted(outer, Property::Real, safety))
                        || (Self::permitted(base, Property::Nonzero, safety)
                            && Self::proved(exponent, Property::Finite)
                            && Self::proved(outer, Property::Integer)))
                {
                    return Self::exponent_product(exponent, outer).map(|count| base.pow(count));
                }
                if let ExprView::Div(numerator, denominator) = inner.view()
                    && Self::permitted(numerator, Property::Positive, safety)
                    && Self::permitted(denominator, Property::Positive, safety)
                    && Self::permitted(outer, Property::Real, safety)
                {
                    return Some(Div::div(numerator.pow(outer), denominator.pow(outer)));
                }
                Self::negative_symbolic_power(inner, outer, safety)
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

    fn combine_product(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        if factors.len() <= 2 {
            return Self::combine_pair(factors, safety);
        }
        if !factors.iter().any(|factor| Self::power(factor).is_some()) {
            return None;
        }
        // Flag storage must outlive the groups borrowing its disjoint slots.
        let mut consumed = vec![false; factors.len()];
        let mut groups = HashMap::new();
        for factor in factors {
            if let Some((base, _)) = Self::power(factor) {
                groups.entry(base).or_insert_with(|| PowerGroup {
                    total: None,
                    nonzero: None,
                    accepted: Vec::new(),
                });
            }
        }
        for (factor, flag) in factors.iter().zip(&mut consumed) {
            let (base, exponent) = Self::power_or_unit(factor);
            let Some(group) = groups.get_mut(base) else {
                continue;
            };
            if !*group
                .nonzero
                .get_or_insert_with(|| Self::permitted(base, Property::Nonzero, safety))
                || Self::retained_literal(&exponent)
                || !Self::proved(&exponent, Property::Finite)
            {
                continue;
            }
            let total = if let Some(previous) = group.total.take() {
                let Some(total) = Self::exponent_sum(&previous, &exponent) else {
                    group.total = Some(previous);
                    continue;
                };
                total
            } else {
                exponent
            };
            group.total = Some(total);
            group.accepted.push(flag);
        }
        let mut combined = Vec::new();
        for (base, group) in groups {
            if let Some(total) = group.total
                && group.accepted.len() > 1
            {
                for flag in group.accepted {
                    *flag = true;
                }
                combined.push(base.pow(total));
            }
        }
        if combined.is_empty() {
            return None;
        }
        combined.extend(
            factors
                .iter()
                .zip(consumed)
                .filter(|(_, removed)| !removed)
                .map(|(factor, _)| factor.clone()),
        );
        Some(Expr::product(combined))
    }

    fn combine_pair(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        for (index, first) in factors.iter().enumerate() {
            let Some((base, exponent)) = Self::power(first) else {
                continue;
            };
            if !Self::permitted(base, Property::Nonzero, safety)
                || !Self::proved(exponent, Property::Finite)
            {
                continue;
            }
            for (position, second) in factors
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index)
            {
                let count = if let Some((other, count)) = Self::power(second) {
                    if base != other {
                        continue;
                    }
                    count.clone()
                } else {
                    if base != second {
                        continue;
                    }
                    Expr::number(1)
                };
                if Self::proved(&count, Property::Finite) {
                    let Some(total) = Self::exponent_sum(exponent, &count) else {
                        continue;
                    };
                    let combined = base.pow(total);
                    let remaining: Vec<_> = factors
                        .iter()
                        .enumerate()
                        .filter(|(slot, _)| *slot != index && *slot != position)
                        .map(|(_, factor)| factor.clone())
                        .chain(once(combined))
                        .collect();
                    return Some(Expr::product(remaining));
                }
            }
        }
        None
    }

    fn exponent_sum(left: &Expr, right: &Expr) -> Option<Expr> {
        if Self::retained_literal(left) || Self::retained_literal(right) {
            return None;
        }
        if let (Some(first), Some(second)) = (Self::exact_integer(left), Self::exact_integer(right))
        {
            first.checked_add(second).map(Expr::number)
        } else {
            Some(Add::add(left, right))
        }
    }

    fn exponent_difference(left: &Expr, right: &Expr) -> Option<Expr> {
        if Self::retained_literal(left) || Self::retained_literal(right) {
            return None;
        }
        if let (Some(first), Some(second)) = (Self::exact_integer(left), Self::exact_integer(right))
        {
            first.checked_sub(second).map(Expr::number)
        } else {
            Some(Sub::sub(left, right))
        }
    }

    fn exponent_product(left: &Expr, right: &Expr) -> Option<Expr> {
        if Self::retained_literal(left) || Self::retained_literal(right) {
            return None;
        }
        if let (Some(first), Some(second)) = (Self::exact_integer(left), Self::exact_integer(right))
        {
            first.checked_mul(second).map(Expr::number)
        } else {
            Some(Mul::mul(left, right))
        }
    }

    fn retained_literal(expression: &Expr) -> bool {
        expression.as_number().is_some_and(|number| {
            number.is_complex() || number.re().is_float() || number.im().is_float()
        })
    }

    fn power_or_unit(expression: &Expr) -> (&Expr, Expr) {
        Self::power(expression).map_or_else(
            || (expression, Expr::number(1)),
            |(base, exponent)| (base, exponent.clone()),
        )
    }

    fn power(expression: &Expr) -> Option<(&Expr, &Expr)> {
        if let ExprView::Pow(base, exponent) = expression.view() {
            Some((base, exponent))
        } else {
            None
        }
    }

    fn common_exponent_product(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        for (index, first) in factors.iter().enumerate() {
            let Some((base, exponent)) = Self::principal_power(first) else {
                continue;
            };
            if Self::retained_literal(&exponent)
                || !Self::permitted(base, Property::Positive, safety)
                || !Self::permitted(&exponent, Property::Finite, safety)
            {
                continue;
            }
            for (position, second) in factors.iter().enumerate().skip(index.saturating_add(1)) {
                let Some((other, count)) = Self::principal_power(second) else {
                    continue;
                };
                if count != exponent || !Self::permitted(other, Property::Positive, safety) {
                    continue;
                }
                let combined = Mul::mul(base, other).pow(&exponent);
                if combined == Mul::mul(first, second) {
                    continue;
                }
                return Some(Expr::product(
                    factors
                        .iter()
                        .enumerate()
                        .filter(|(slot, _)| *slot != index && *slot != position)
                        .map(|(_, factor)| factor.clone())
                        .chain(once(combined))
                        .collect(),
                ));
            }
        }
        None
    }

    fn common_exponent_quotient(top: &Expr, bottom: &Expr, safety: DomainSafety) -> Option<Expr> {
        let (base, exponent) = Self::principal_power(top)?;
        let (other, count) = Self::principal_power(bottom)?;
        if exponent != count
            || Self::retained_literal(&exponent)
            || !Self::permitted(base, Property::Positive, safety)
            || !Self::permitted(other, Property::Positive, safety)
        {
            return None;
        }
        let result = Div::div(base, other).pow(exponent);
        (result != Div::div(top, bottom)).then_some(result)
    }

    fn principal_power(expression: &Expr) -> Option<(&Expr, Expr)> {
        match expression.view() {
            ExprView::Pow(base, exponent) => Some((base, exponent.clone())),
            ExprView::Function { name, args } if *name == BuiltinFunction::Sqrt.function() => {
                let [base] = args else {
                    return None;
                };
                Some((base, Div::div(Expr::number(1), 2)))
            }
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Sum(_)
            | ExprView::Product(_)
            | ExprView::Div(..)
            | ExprView::Function { .. }
            | ExprView::Derivative { .. } => None,
        }
    }

    fn negative_symbolic_power(base: &Expr, exponent: &Expr, safety: DomainSafety) -> Option<Expr> {
        if !Self::permitted(base, Property::Nonzero, safety) {
            return None;
        }
        let ExprView::Product(factors) = exponent.view() else {
            return None;
        };
        let first = factors.first()?;
        let (numerator, _) = Self::exact_ratio(first)?;
        if numerator >= 0 || numerator == i64::MIN {
            return None;
        }
        let positive = Sub::sub(Expr::number(0), first);
        let power = Expr::product(
            once(positive)
                .chain(factors.iter().skip(1).cloned())
                .collect(),
        );
        Some(Div::div(Expr::number(1), base.pow(power)))
    }
}
