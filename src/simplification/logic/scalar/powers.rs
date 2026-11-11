//! Principal-power identities with explicit nonzero and branch predicates.
//!
//! Float and complex exponent literals keep their representation: they never
//! combine with another exponent.

use core::{
    iter::once,
    num::NonZeroU64,
    ops::{Add, Div, Mul, Neg, Sub},
};

use alloc::{vec, vec::Vec};

use num_anafis::{NonZeroU64Ext, Number, RationalType};

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, ExactIdentity, Expr, ExprView, Property, Truth},
};

use super::{Domain, DomainSafety};

/// Namespace for principal-power combination and exponent laws.
pub struct Powers;

/// References into a disjoint mutable flag iterator identify accepted factors
/// without unchecked indices. A total is present exactly when a factor was kept.
/// Guards run only for a base shared by at least two factors.
struct PowerGroup<'factor> {
    members: usize,
    total: Option<Expr>,
    nonzero: Option<bool>,
    accepted: Vec<&'factor mut bool>,
}

impl Powers {
    /// Combine powers only when their logarithmic definitions have the same branch.
    pub fn combine(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        match expression.view() {
            ExprView::Product(factors) => Self::combine_product(factors, safety)
                .or_else(|| Self::common_exponent_product(factors, safety)),
            ExprView::Div(top, bottom) => {
                // A plain side is the power of its base with exponent one.
                let (base, numerator, denominator) = match (top.view(), bottom.view()) {
                    (ExprView::Pow(base, numerator), ExprView::Pow(other, denominator))
                        if base == other =>
                    {
                        (base, numerator.clone(), denominator.clone())
                    }
                    (ExprView::Pow(base, numerator), _) if base == bottom => {
                        (base, numerator.clone(), Expr::number(1))
                    }
                    (_, ExprView::Pow(base, denominator)) if base == top => {
                        (base, Expr::number(1), denominator.clone())
                    }
                    _ => return Self::common_exponent_quotient(top, bottom, safety),
                };
                if Domain::admits(base.properties().get(Property::Nonzero), safety)
                    && numerator.properties().get(Property::Finite) == Truth::True
                    && denominator.properties().get(Property::Finite) == Truth::True
                {
                    Self::exponent(&numerator, &denominator, i64::checked_sub, |left, right| {
                        Sub::sub(left, right)
                    })
                    .map(|exponent| base.pow(exponent))
                } else {
                    None
                }
            }
            ExprView::Pow(inner, outer) => {
                if let ExprView::Pow(base, exponent) = inner.view() {
                    return Self::nested_power(inner, base, exponent, outer, safety);
                }
                if let ExprView::Div(numerator, denominator) = inner.view()
                    && Domain::admits(numerator.properties().get(Property::Positive), safety)
                    && Domain::admits(denominator.properties().get(Property::Positive), safety)
                    && Domain::admits(outer.properties().get(Property::Real), safety)
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

    /// Combine \((z^a)^b\) for `inner` \(=z^a\). Each operand's properties are
    /// computed at most once.
    fn nested_power(
        inner: &Expr,
        base: &Expr,
        exponent_expr: &Expr,
        outer: &Expr,
        safety: DomainSafety,
    ) -> Option<Expr> {
        let facts = base.properties();
        if let Some((count, 1)) = exponent_expr.exact_ratio()
            && count > 0
            && count % 2 == 0
            && let Some((numerator, denominator)) = outer.exact_ratio()
            && numerator > 0
            && denominator > 1
            && Domain::admits(facts.get(Property::Real), safety)
        {
            // \((x^{2n})^{p/q}=|x|^{2np/q}\) for real x and p/q>0. Cancelling
            // \(g=\gcd(2n,q)\) first keeps \((2n/g)\,p/(q/g)\) reduced, and a
            // native overflow of the numerator retains the nesting.
            let (scale, reduced) =
                NonZeroU64::new(count.unsigned_abs())?.cofactors(NonZeroU64::new(denominator)?);
            let total = numerator.checked_mul(i64::try_from(scale.get()).ok()?)?;
            let power = Number::from_rational(RationalType::from_parts(total, reduced.get()));
            return Some(base.abs().pow(power));
        }
        let positive = Domain::admits(facts.get(Property::Positive), safety);
        let nonzero = Domain::admits(facts.get(Property::Nonzero), safety);
        if positive || nonzero {
            let inner_facts = exponent_expr.properties();
            let outer_facts = outer.properties();
            if (positive
                && Domain::admits(inner_facts.get(Property::Real), safety)
                && Domain::admits(outer_facts.get(Property::Real), safety))
                || (nonzero
                    && inner_facts.get(Property::Finite) == Truth::True
                    && outer_facts.get(Property::Integer) == Truth::True)
            {
                return Self::exponent(exponent_expr, outer, i64::checked_mul, |left, right| {
                    Mul::mul(left, right)
                })
                .map(|count| base.pow(count));
            }
        }
        Self::negative_symbolic_power(inner, outer, safety)
    }

    fn combine_product(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        if factors.len() <= 2 {
            return Self::combine_pair(factors, safety);
        }
        if !factors
            .iter()
            .any(|factor| matches!(factor.view(), ExprView::Pow(..)))
        {
            return None;
        }
        // Each factor as its base and stored exponent. A plain factor equal to
        // a power base joins that group with exponent one.
        let parts = factors
            .iter()
            .map(|factor| {
                if let ExprView::Pow(base, exponent) = factor.view() {
                    (base, Some(exponent))
                } else {
                    (factor, None)
                }
            })
            .collect::<Vec<_>>();
        // Flag storage must outlive the groups borrowing its disjoint slots.
        let mut consumed = vec![false; factors.len()];
        let mut groups = FixedMap::default();
        for (base, stored) in &parts {
            if stored.is_some() {
                groups.entry(*base).or_insert_with(|| PowerGroup {
                    members: 0,
                    total: None,
                    nonzero: None,
                    accepted: Vec::new(),
                });
            }
        }
        for (base, _) in &parts {
            if let Some(group) = groups.get_mut(*base) {
                // Each factor is counted once, so the count stays below `factors.len()`.
                group.members = group.members.saturating_add(1);
            }
        }
        for ((base, stored), flag) in parts.iter().zip(&mut consumed) {
            let Some(group) = groups.get_mut(*base).filter(|group| group.members > 1) else {
                continue;
            };
            let factor_exponent = stored.map_or_else(|| Expr::number(1), Clone::clone);
            if !*group.nonzero.get_or_insert_with(|| {
                Domain::admits(base.properties().get(Property::Nonzero), safety)
            }) || factor_exponent
                .as_number()
                .is_some_and(|number| number.is_complex() || !number.is_exact())
                || factor_exponent.properties().get(Property::Finite) != Truth::True
            {
                continue;
            }
            let total = if let Some(previous) = group.total.take() {
                let Some(total) = Self::exponent(
                    &previous,
                    &factor_exponent,
                    i64::checked_add,
                    |left, right| Add::add(left, right),
                ) else {
                    group.total = Some(previous);
                    continue;
                };
                total
            } else {
                factor_exponent
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
            let ExprView::Pow(base, factor_exponent) = first.view() else {
                continue;
            };
            for (position, second) in factors
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index)
            {
                let count = if let ExprView::Pow(other, count) = second.view() {
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
                // The guards of the first factor run only once it has a partner.
                if !Domain::admits(base.properties().get(Property::Nonzero), safety)
                    || factor_exponent.properties().get(Property::Finite) != Truth::True
                {
                    break;
                }
                if count.properties().get(Property::Finite) == Truth::True {
                    let Some(total) =
                        Self::exponent(factor_exponent, &count, i64::checked_add, |left, right| {
                            Add::add(left, right)
                        })
                    else {
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

    /// The sum, difference, or product of two exponents. Exact integers fold
    /// natively, and a native overflow keeps the powers apart; other
    /// exponents combine symbolically. A float or complex literal never
    /// combines.
    fn exponent(
        left: &Expr,
        right: &Expr,
        exact: fn(i64, i64) -> Option<i64>,
        symbolic: fn(&Expr, &Expr) -> Expr,
    ) -> Option<Expr> {
        if [left, right].into_iter().any(|operand| {
            operand
                .as_number()
                .is_some_and(|number| number.is_complex() || !number.is_exact())
        }) {
            return None;
        }
        if let (Some((first, 1)), Some((second, 1))) = (left.exact_ratio(), right.exact_ratio()) {
            exact(first, second).map(Expr::number)
        } else {
            Some(symbolic(left, right))
        }
    }

    /// \(p^a q^a=(pq)^a\) for the first pair of positive bases sharing an
    /// exponent. Exponents compare by identity before any property is proved.
    fn common_exponent_product(factors: &[Expr], safety: DomainSafety) -> Option<Expr> {
        let powers = factors
            .iter()
            .enumerate()
            .filter_map(|(index, factor)| {
                Self::principal_power(factor)
                    .map(|(base, exponent)| (index, factor, base, exponent))
            })
            .collect::<Vec<_>>();
        for (offset, (index, first, base, exponent)) in powers.iter().enumerate() {
            if exponent
                .as_number()
                .is_some_and(|number| number.is_complex() || !number.is_exact())
            {
                continue;
            }
            // The first factor's guards are proved once, at its first partner.
            let mut admitted = None;
            for (position, second, other, count) in powers.iter().skip(offset.saturating_add(1)) {
                if count != exponent {
                    continue;
                }
                if !*admitted.get_or_insert_with(|| {
                    Domain::admits(base.properties().get(Property::Positive), safety)
                        && Domain::admits(exponent.properties().get(Property::Finite), safety)
                }) {
                    break;
                }
                if !Domain::admits(other.properties().get(Property::Positive), safety) {
                    continue;
                }
                let combined = Mul::mul(*base, *other).pow(exponent);
                if combined == Mul::mul(*first, *second) {
                    continue;
                }
                return Some(Expr::product(
                    factors
                        .iter()
                        .enumerate()
                        .filter(|(slot, _)| slot != index && slot != position)
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
            || exponent
                .as_number()
                .is_some_and(|number| number.is_complex() || !number.is_exact())
            || !Domain::admits(base.properties().get(Property::Positive), safety)
            || !Domain::admits(other.properties().get(Property::Positive), safety)
        {
            return None;
        }
        let result = Div::div(base, other).pow(exponent);
        (result != Div::div(top, bottom)).then_some(result)
    }

    /// A stored power, or a square root as the power \(1/2\) of its radicand.
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

    /// \(z^{-ca}=1/z^{ca}\) for a manifest negative exact coefficient \(-c\).
    fn negative_symbolic_power(base: &Expr, exponent: &Expr, safety: DomainSafety) -> Option<Expr> {
        let ExprView::Product(factors) = exponent.view() else {
            return None;
        };
        let first = factors.first()?;
        let (numerator, _) = first.exact_ratio()?;
        // Native negation of the minimum integer wraps.
        if numerator >= 0 || numerator == i64::MIN {
            return None;
        }
        if !Domain::admits(base.properties().get(Property::Nonzero), safety) {
            return None;
        }
        let positive = Neg::neg(first);
        let power = Expr::product(
            once(positive)
                .chain(factors.iter().skip(1).cloned())
                .collect(),
        );
        Some(Div::div(Expr::number(1), base.pow(power)))
    }
}
