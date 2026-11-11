//! Canonical roots, reciprocals, counted positive integer factors, and wide
//! integer and literal powers.

#![expect(
    unsafe_code,
    reason = "quotient operands are read by edge ordinal, which a quotient bounds by its two edges"
)]

use core::{convert::Infallible, iter::from_fn};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType, Real};

use super::{
    BuiltinFunction, ExactIdentity, Expr, FunctionDefinition, Node, NumericValue, Postorder,
};

impl Expr {
    /// \(b^n\) for an `i128` exponent \(n\). An exponent beyond `i64` splits by
    /// binary exponentiation, \(b^{2m+r} = (b^2)^m\,b^r\), into native powers.
    #[must_use]
    pub fn integer_power(base: &Self, exponent: i128) -> Self {
        if let Ok(native) = i64::try_from(exponent) {
            return base.pow(native);
        }
        let mut remaining = exponent.unsigned_abs();
        let mut block = if exponent < 0 {
            base.pow(-1)
        } else {
            base.clone()
        };
        let mut factors = Vec::new();
        loop {
            if let Ok(native) = i64::try_from(remaining) {
                factors.push(block.pow(native));
                break;
            }
            if remaining & 1 != 0 {
                factors.push(block.clone());
            }
            remaining >>= 1;
            block = block.pow(2);
        }
        Self::product(factors)
    }

    /// \(a / b^n\) for finite literals \(a\), \(b \ne 0\) and \(n > 0\).
    /// An exact quotient that exceeds the backend stays \(a\,b^{-n}\); an
    /// approximate operand folds the undivided factors in floating point.
    #[must_use]
    pub fn scalar_quotient(
        numerator: &NumericValue,
        denominator: &NumericValue,
        count: i64,
    ) -> Self {
        let (coefficient, remaining) = numerator.divide_power(denominator, count);
        if remaining == 0 {
            return Self::number(coefficient);
        }
        let base = if numerator.has_float() || denominator.has_float() {
            denominator.value().to_float()
        } else {
            denominator.value().clone()
        };
        Self::product(vec![
            Self::number(coefficient),
            Self::number(base).pow(remaining.saturating_neg()),
        ])
    }
}

impl Node {
    /// Collect factors of one base with exact positive rational exponents,
    /// \(x^{a} x^{b} = x^{a+b}\), reading \(\sqrt{x}\) as \(x^{1/2}\). For the
    /// principal power \(x^{a} = e^{a \operatorname{Log} x}\) this holds at
    /// every complex \(x \ne 0\), and both sides vanish at \(x = 0\) since
    /// \(a, b > 0\). So collection is associative: \((\sqrt{x}\sqrt{x})\sqrt{x}\)
    /// and \(\sqrt{x}\sqrt{x}\sqrt{x}\) are both \(x^{3/2}\).
    ///
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
            && !factors
                .iter()
                .zip(factors.iter().skip(1))
                .any(|(left, right)| left == right)
            && !factors.iter().any(|factor| {
                matches!(factor.node(), Self::Function { name, .. }
                    if *name == BuiltinFunction::Sqrt.function())
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
        let mut ordered = Self::ordered_factors(&factors).peekable();
        let mut result = Vec::with_capacity(factors.len());
        let mut changed = false;
        let mut rebuild = false;
        while let Some((base, power, original)) = ordered.next() {
            // Equal bases are adjacent; keep the run's originals unless its
            // total power fits and the run collects into one power.
            let start = result.len();
            result.push(original.clone());
            // Exponents become rationals only in a run, which is rare.
            let ratio = |(numerator, denominator): (i64, u64)| {
                RationalType::from_parts(numerator, denominator)
            };
            let mut total = None;
            while let Some((_, next, same)) = ordered.next_if(|(next, _, _)| *next == base) {
                total = Some(
                    total
                        .unwrap_or_else(|| Some(ratio(power)))
                        .and_then(|sum: RationalType| sum.checked_add(&ratio(next))),
                );
                result.push(same.clone());
            }
            if let Some(Some(collected_power)) = total {
                result.truncate(start);
                let collected = Self::normalize_power(
                    base.clone(),
                    Expr::number(Number::from_rational(collected_power)),
                );
                rebuild |= !matches!(collected.node(), Self::Pow(collected_base, _) if collected_base == base);
                result.push(collected);
                changed = true;
            }
        }
        drop(ordered);
        if !changed {
            return (factors, false);
        }
        result.extend(
            factors
                .iter()
                .filter(|factor| Self::rational_factor(factor).is_none())
                .cloned(),
        );
        Self::sort_operands(&mut result);
        (result, rebuild)
    }

    /// Normalize \(x^{1/2}=\sqrt{x}\) and \(e^x=\exp(x)\).
    /// An integer power \(n > 1\) of a power with an exact positive rational
    /// exponent \(a\) uses \((x^a)^n=x^{na}\) when the count fits, and
    /// \((\sqrt{x})^n=x^{n/2}\).
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
            if exponent.is_exact_one() {
                return base;
            }
            if exponent.exact_ratio() == Some((1, 2)) {
                return FunctionDefinition::construct(BuiltinFunction::Sqrt.function(), vec![base]);
            }
            if let (Some(value), Some(power)) = (base.literal(), exponent.literal())
                && value.value().is_finite()
                && power.value().is_finite()
                && let Some(folded) = value.pow(power)
            {
                return Expr::number(folded);
            }
            if let Some((count, 1)) = exponent.exact_ratio()
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
                    // $(x^{a})^{n} = e^{n a \operatorname{Log} x} = x^{n a}$
                    // for an integer $n$ and any exact positive rational $a$.
                    Self::Pow(inner, previous)
                        if let Some((numerator, denominator)) = previous.exact_ratio()
                            && numerator > 0
                            && let Some(combined) = count.checked_mul(numerator) =>
                    {
                        exponent = Expr::number(Number::from_rational(RationalType::from_parts(
                            combined,
                            denominator,
                        )));
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
            if let Some(denominator) = Self::negative_power(&base, &exponent) {
                return Self::normalize_quotient(Expr::number(1), denominator);
            }
            return Expr::intern(Self::Pow(base, exponent));
        }
    }

    /// Normalize \((aA)/(bB)=(a/b)(A/B)\), keeping exact coefficients outside.
    /// Coefficient division requires a nonzero divisor and a representable result.
    /// Denominator quotients stay intact: \(1/(1/x)\) must retain \(x\ne0\).
    pub fn normalize_quotient(mut left: Expr, mut right: Expr) -> Expr {
        if right.is_exact_one() {
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
            if !divisor.has_float()
                && !coefficient.has_float()
                && let Some(value) = coefficient.div(divisor)
            {
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
        let Ok(distributed) = Postorder::factors(base).evaluate(|node| {
            let original = node.expression();
            Ok::<_, Infallible>(match original.node() {
                Self::Product(_) => Expr::product(node.values().cloned().collect()),
                Self::Div(..) => {
                    // SAFETY: a quotient has two stored edges.
                    let (numerator, denominator) = unsafe { (node.child(0), node.child(1)) };
                    Self::normalize_quotient(numerator.clone(), denominator.clone())
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
            })
        });
        distributed
    }

    /// Construct a negative exponent's positive denominator without overflowing
    /// the native numerator. The minimum integer uses logarithmic power blocks.
    fn negative_power(base: &Expr, exponent: &Expr) -> Option<Expr> {
        if let Some((numerator, denominator)) = exponent.exact_ratio() {
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
        // A real value has the exact imaginary part `0`; a complex literal with
        // imaginary part `-0.0` keeps its representation, so it stays a power.
        match exponent.as_number()?.components() {
            (Real::Float(value), Real::Int(0)) if value.is_finite() && *value < 0.0 => {
                Some(Self::normalize_power(base.clone(), Expr::number(-value)))
            }
            _ => None,
        }
    }

    /// The collectable factors as `(base, exponent, factor)`, equal bases
    /// adjacent.
    fn ordered_factors(factors: &[Expr]) -> impl Iterator<Item = (&Expr, (i64, u64), &Expr)> {
        // Factors are ordered by kind and then structure: other factors by
        // their own structure, square roots (one function name) by their
        // argument, and powers by base first. Removing excluded factors keeps
        // each of these three streams ordered by base, so base groups need
        // only a linear merge.
        let stream = |kind: u8| {
            factors
                .iter()
                .filter_map(move |factor| {
                    let (base, power) = Self::rational_factor(factor)?;
                    let class = if matches!(factor.node(), Self::Pow(..)) {
                        2
                    } else {
                        u8::from(base != factor)
                    };
                    (class == kind).then_some((base, power, factor))
                })
                .peekable()
        };
        let mut streams = [stream(0), stream(1), stream(2)];
        from_fn(move || {
            let mut least: Option<(usize, &Expr)> = None;
            for (index, candidate) in streams.iter_mut().enumerate() {
                if let Some(&(base, ..)) = candidate.peek()
                    && least.is_none_or(|(_, known)| Self::compare(base, known).is_lt())
                {
                    least = Some((index, base));
                }
            }
            streams.get_mut(least?.0)?.next()
        })
    }

    /// A factor's base and exact positive rational exponent in lowest terms,
    /// `(numerator, denominator)`: \(\sqrt{b}\) is \(b^{1/2}\), a power
    /// \(b^{p/q}\) has its own, and any other nonliteral factor \(f\) is
    /// \(f^{1}\). Literals and other powers are not collected.
    fn rational_factor(factor: &Expr) -> Option<(&Expr, (i64, u64))> {
        match factor.node() {
            Self::Function { name, args }
                if *name == BuiltinFunction::Sqrt.function()
                    && let [inner] = args.as_slice() =>
            {
                Some((inner, (1, 2)))
            }
            Self::Pow(base, exponent) => exponent
                .exact_ratio()
                .filter(|(numerator, _)| *numerator > 0)
                .map(|ratio| (base, ratio)),
            Self::Number(_) | Self::ImaginaryUnit => None,
            Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Poly(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Derivative { .. } => Some((factor, (1, 1))),
        }
    }
}
