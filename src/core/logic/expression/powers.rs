//! Collect positive integer factors without changing other power domains.

use core::iter::from_fn;

use alloc::vec::Vec;

use super::{Expr, Node, NumericValue};

impl Node {
    /// Preserve the ordered vector unless a group can actually be collected.
    pub fn collect_positive_factors(factors: Vec<Expr>) -> (Vec<Expr>, bool) {
        let has_power = factors
            .iter()
            .any(|factor| matches!(factor.node(), Self::Pow(..)));
        if !has_power
            && !factors.windows(2).any(|pair| {
                let [left, right] = pair else {
                    return false;
                };
                left.same_node(right)
            })
        {
            return (factors, false);
        }
        let mut previous: Option<&Expr> = None;
        let repeated = Self::ordered_factors(&factors).any(|(base, _, _)| {
            let same = previous.is_some_and(|prior| prior.same_node(base));
            previous = Some(base);
            same
        });
        if !repeated {
            return (factors, false);
        }
        let grouped: Vec<_> = Self::ordered_factors(&factors).collect();
        let mut result = Vec::with_capacity(factors.len());
        let mut changed = false;
        for group in grouped.chunk_by(|(left, _, _), (right, _, _)| left.same_node(right)) {
            let Some((base, _, _)) = group.first() else {
                continue;
            };
            let total = group
                .iter()
                .try_fold(0_i64, |sum, (_, power, _)| sum.checked_add(*power));
            if group.len() > 1
                && let Some(power) = total
            {
                result.push(Expr::pow_static((*base).clone(), Expr::number(power)));
                changed = true;
            } else {
                result.extend(group.iter().map(|(_, _, original)| (*original).clone()));
            }
        }
        if !changed {
            return (factors, false);
        }
        result.extend(
            factors
                .iter()
                .filter(|factor| Self::positive_factor(factor).is_none())
                .cloned(),
        );
        Self::sort_operands(&mut result);
        (result, true)
    }

    fn ordered_factors(factors: &[Expr]) -> impl Iterator<Item = (&Expr, i64, &Expr)> {
        // Non-power factors are already ordered by their own structure. Pow
        // factors are ordered by base first. Removing excluded factors keeps
        // each stream ordered, so base groups need only a linear merge.
        let mut plain = factors
            .iter()
            .filter(|factor| !matches!(factor.node(), Self::Pow(..)))
            .filter_map(|factor| {
                Self::positive_factor(factor).map(|(base, power)| (base, power, factor))
            })
            .peekable();
        let mut powered = factors
            .iter()
            .filter(|factor| matches!(factor.node(), Self::Pow(..)))
            .filter_map(|factor| {
                Self::positive_factor(factor).map(|(base, power)| (base, power, factor))
            })
            .peekable();
        from_fn(move || match (plain.peek(), powered.peek()) {
            (Some((first, _, _)), Some((second, _, _))) => {
                if Self::compare(first, second).is_le() {
                    plain.next()
                } else {
                    powered.next()
                }
            }
            (Some(_), None) => plain.next(),
            (None, Some(_)) => powered.next(),
            (None, None) => None,
        })
    }

    /// Return a factor's scalar base and proved positive integer exponent.
    /// General powers remain opaque factors and are not collected here.
    pub fn positive_factor(factor: &Expr) -> Option<(&Expr, i64)> {
        match factor.node() {
            Self::Pow(base, exponent) => exponent
                .literal()
                .and_then(NumericValue::gaussian)
                .filter(|(real, imaginary)| *real > 0 && *imaginary == 0)
                .map(|(power, _)| (base, power)),
            Self::Number(_) | Self::ImaginaryUnit => None,
            Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Poly(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Derivative { .. } => Some((factor, 1)),
        }
    }
}
