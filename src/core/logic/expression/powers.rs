//! Canonical roots, reciprocals, and counted positive integer factors.

use core::iter::from_fn;

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType, Real};

use crate::collections::HashMap;

use super::{BuiltinFunction, Expr, FunctionDefinition, Node, NumericValue};

impl Node {
    /// Preserve the ordered vector unless a group can actually be collected.
    /// Request another normalization pass only when a collected power changes
    /// its base or produces a literal, which can merge with another operand.
    pub fn collect_positive_factors(factors: Vec<Expr>) -> (Vec<Expr>, bool) {
        let has_power = factors
            .iter()
            .rev()
            .find(|factor| !matches!(factor.node(), Self::Derivative { .. }))
            .is_some_and(|factor| matches!(factor.node(), Self::Pow(..)));
        if !has_power
            && !factors.windows(2).any(|pair| {
                let [left, right] = pair else {
                    return false;
                };
                left == right
            })
        {
            return (factors, false);
        }
        let mut previous: Option<&Expr> = None;
        let repeated = Self::ordered_factors(&factors).any(|(base, _, _)| {
            let same = previous == Some(base);
            previous = Some(base);
            same
        });
        if !repeated {
            return (factors, false);
        }
        let grouped: Vec<_> = Self::ordered_factors(&factors).collect();
        let mut result = Vec::with_capacity(factors.len());
        let mut changed = false;
        let mut rebuild = false;
        for group in grouped.chunk_by(|(left, _, _), (right, _, _)| left == right) {
            let Some((base, _, _)) = group.first() else {
                continue;
            };
            let total = group
                .iter()
                .try_fold(0_i64, |sum, (_, power, _)| sum.checked_add(*power));
            if group.len() > 1
                && let Some(power) = total
            {
                let collected = Self::normalize_power((*base).clone(), Expr::number(power));
                rebuild |= !matches!(collected.node(), Self::Pow(collected_base, _) if collected_base == *base);
                result.push(collected);
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
        (result, rebuild)
    }

    /// Normalize \(x^{1/2}=\sqrt{x}\) and \(e^x=\exp(x)\).
    /// Positive integer nesting uses \((x^m)^n=x^{mn}\) when the count fits.
    /// Known negative real exponents use reciprocals. Unknown and complex signs
    /// remain symbolic; zero powers retain their original domain exclusions.
    pub fn normalize_power(mut base: Expr, mut exponent: Expr) -> Expr {
        loop {
            if matches!(base.node(), Self::Euler) {
                return FunctionDefinition::construct(
                    BuiltinFunction::Exp.function(),
                    vec![exponent],
                );
            }
            if exponent.is_one_num() {
                return base;
            }
            if exponent.literal().and_then(NumericValue::exact_ratio) == Some((1, 2)) {
                return FunctionDefinition::construct(BuiltinFunction::Sqrt.function(), vec![base]);
            }
            if let (Some(value), Some(power)) = (base.literal(), exponent.literal())
                && value.value().is_finite()
                && power.value().is_finite()
                && let Some(folded) = value.pow(power)
            {
                return Expr::number(folded);
            }
            if let Some((count, 1)) = exponent.literal().and_then(NumericValue::exact_ratio)
                && count > 1
            {
                match base.node() {
                    Self::Product(_) | Self::Div(..) => {
                        return Self::distribute_power(&base, &exponent);
                    }
                    Self::Function { name, args }
                        if *name == BuiltinFunction::Sqrt.function()
                            && let [inner] = args.as_slice() =>
                    {
                        exponent =
                            Expr::number(Number::from_rational(RationalType::from_parts(count, 2)));
                        base = inner.clone();
                        continue;
                    }
                    Self::Pow(inner, previous)
                        if let Some((inner_count, 1)) =
                            previous.literal().and_then(NumericValue::exact_ratio)
                            && inner_count > 0
                            && let Some(combined) = count.checked_mul(inner_count) =>
                    {
                        exponent = Expr::number(combined);
                        base = inner.clone();
                        continue;
                    }
                    Self::Number(_)
                    | Self::ImaginaryUnit
                    | Self::Pi
                    | Self::Euler
                    | Self::Symbol(_)
                    | Self::Function { .. }
                    | Self::Sum(_)
                    | Self::Poly(_)
                    | Self::Pow(..)
                    | Self::Derivative { .. } => {}
                }
            }
            if let Some(denominator) = exponent
                .literal()
                .and_then(|power| Self::negative_power(&base, power))
            {
                return Self::normalize_quotient(Expr::number(1), denominator);
            }
            return Expr::intern(Self::Pow(base, exponent));
        }
    }

    /// Normalize \((aA)/(bB)=(a/b)(A/B)\), keeping exact coefficients outside.
    /// Coefficient division requires a nonzero divisor and a representable result.
    /// Denominator quotients stay intact: \(1/(1/x)\) must retain \(x\ne0\).
    pub fn normalize_quotient(mut left: Expr, mut right: Expr) -> Expr {
        if right.is_one_num() {
            return left;
        }
        if let (Some(numerator), Some(denominator)) = (left.literal(), right.literal()) {
            if numerator.value().is_finite()
                && denominator.value().is_finite()
                && let Some(value) = numerator.div(denominator)
            {
                return Expr::number(value);
            }
            return Expr::intern(Self::Div(left, right));
        }
        if let Some((numerator, denominator)) = Self::split_quotient(&left) {
            right = Expr::product(vec![denominator, right]);
            left = numerator;
        }
        let (numerator_coefficient, numerators) = Self::coefficient_parts(&left);
        let (denominator_coefficient, denominators) = Self::coefficient_parts(&right);
        if numerator_coefficient.is_some() || denominator_coefficient.is_some() {
            let unit = NumericValue::new(Number::one());
            let divisor = denominator_coefficient.unwrap_or(&unit);
            let coefficient = numerator_coefficient.unwrap_or(&unit);
            if !divisor.value().is_zero() && !divisor.has_float() && !coefficient.has_float() {
                let (value, remaining) = coefficient.divide_power(divisor, 1);
                if remaining == 0 {
                    let numerator = if let [single] = numerators {
                        single.clone()
                    } else {
                        Expr::product(numerators.to_vec())
                    };
                    let quotient = if denominators.is_empty() {
                        numerator
                    } else {
                        let denominator = if let [single] = denominators {
                            single.clone()
                        } else {
                            Expr::product(denominators.to_vec())
                        };
                        Expr::intern(Self::Div(numerator, denominator))
                    };
                    return if value.is_one() {
                        quotient
                    } else {
                        Expr::product(vec![Expr::number(value), quotient])
                    };
                }
            }
        }
        Expr::intern(Self::Div(left, right))
    }

    /// Read \(c(A/B)\) as \((cA,B)\) for iterative denominator collection.
    pub fn split_quotient(expression: &Expr) -> Option<(Expr, Expr)> {
        let (coefficient, quotient) = if let Self::Product(factors) = expression.node() {
            let [coefficient, quotient] = factors.as_slice() else {
                return None;
            };
            if !coefficient
                .literal()
                .is_some_and(|value| value.value().is_finite())
            {
                return None;
            }
            (Some(coefficient), quotient)
        } else {
            (None, expression)
        };
        let Self::Div(numerator, denominator) = quotient.node() else {
            return None;
        };
        let top = coefficient.map_or_else(
            || numerator.clone(),
            |value| Expr::product(vec![value.clone(), numerator.clone()]),
        );
        Some((top, denominator.clone()))
    }

    /// Distribute an integer power over products and quotients in DAG order.
    /// Opaque leaves use the canonical power kernel; shared operands run once.
    fn distribute_power(base: &Expr, exponent: &Expr) -> Expr {
        let mut pending = vec![(base, false)];
        let mut memo: HashMap<&Expr, Expr> = HashMap::new();
        while let Some((original, ready)) = pending.pop() {
            if memo.contains_key(original) {
                continue;
            }
            if !ready && matches!(original.node(), Self::Product(_) | Self::Div(..)) {
                pending.push((original, true));
                pending.extend(Self::children(original).map(|child| (child, false)));
                continue;
            }
            let child = |input: &Expr| {
                memo.get(input)
                    .expect("power operands precede their parent")
                    .clone()
            };
            let value = match original.node() {
                Self::Product(factors) => Expr::product(factors.iter().map(child).collect()),
                Self::Div(numerator, denominator) => {
                    Self::normalize_quotient(child(numerator), child(denominator))
                }
                Self::Number(_)
                | Self::ImaginaryUnit
                | Self::Pi
                | Self::Euler
                | Self::Symbol(_)
                | Self::Function { .. }
                | Self::Sum(_)
                | Self::Poly(_)
                | Self::Pow(..)
                | Self::Derivative { .. } => {
                    Self::normalize_power(original.clone(), exponent.clone())
                }
            };
            memo.insert(original, value);
        }
        memo.remove(base)
            .expect("power traversal includes its root")
    }

    /// Construct a negative exponent's positive denominator without overflowing
    /// the native numerator. The minimum integer uses logarithmic power blocks.
    fn negative_power(base: &Expr, power: &NumericValue) -> Option<Expr> {
        if let Some((numerator, denominator)) = power.exact_ratio() {
            if numerator >= 0 {
                return None;
            }
            return numerator.checked_neg().map_or_else(
                || {
                    let root = (denominator != 1).then(|| {
                        Self::normalize_power(
                            base.clone(),
                            Expr::number(Number::from_rational(RationalType::from_parts(
                                1,
                                denominator,
                            ))),
                        )
                    });
                    Some(Expr::integer_power(
                        root.as_ref().unwrap_or(base),
                        i128::from(numerator).saturating_neg(),
                    ))
                },
                |magnitude| {
                    Some(Self::normalize_power(
                        base.clone(),
                        Expr::number(Number::from_rational(RationalType::from_parts(
                            magnitude,
                            denominator,
                        ))),
                    ))
                },
            );
        }
        power.parts().and_then(|parts| match parts.re {
            Real::Float(value) if parts.im.is_zero() && value.is_finite() && value < 0.0 => {
                Some(Self::normalize_power(base.clone(), Expr::number(-value)))
            }
            Real::Int(_) | Real::Rational(_) | Real::Float(_) | _ => None,
        })
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
