//! Canonical roots, reciprocals, counted positive integer factors, and wide
//! integer and literal powers.

#![expect(
    unsafe_code,
    reason = "quotient operands are read by edge ordinal, which a quotient bounds by its two edges"
)]

use core::{convert::Infallible, iter::from_fn};

use alloc::{vec, vec::Vec};

use num_anafis::{Number, RationalType, Real};

use crate::collections::FixedSet;

use super::{
    BuiltinFunction, ExactIdentity, Expr, FunctionDefinition, Node, NumericValue, Postorder,
    UnaryRule,
};

impl Node {
    /// $b^n$ for an `i128` exponent $n$. An exponent beyond `i64` splits by
    /// binary exponentiation, $b^{2m+r} = (b^2)^m b^r$, into native powers.
    #[must_use]
    pub fn integer_power(base: &Expr, exponent: i128) -> Expr {
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
        Expr::product(factors)
    }

    /// $a / b^n$ for finite literals $a$, $b \ne 0$ and $n > 0$.
    /// An exact quotient that exceeds the backend stays $a b^{-n}$; an
    /// approximate operand folds the undivided factors in floating point.
    #[must_use]
    pub fn scalar_quotient(
        numerator: &NumericValue,
        denominator: &NumericValue,
        count: i64,
    ) -> Expr {
        let (coefficient, remaining) = numerator.divide_power(denominator, count);
        if remaining == 0 {
            return Expr::number(coefficient);
        }
        let base = if numerator.has_float() || denominator.has_float() {
            denominator.value().to_float()
        } else {
            denominator.value().clone()
        };
        Expr::product(vec![
            Expr::number(coefficient),
            Expr::number(base).pow(remaining.saturating_neg()),
        ])
    }

    /// Collect factors of one base with exact positive rational exponents,
    /// $x^{a} x^{b} = x^{a+b}$, reading $\sqrt{x}$ as $x^{1/2}$. For the
    /// principal power $x^{a} = e^{a \operatorname{Log} x}$ this holds at
    /// every complex $x \ne 0$, and both sides vanish at $x = 0$ since
    /// $a, b > 0$. So collection is associative: $(\sqrt{x}\sqrt{x})\sqrt{x}$
    /// and $\sqrt{x}\sqrt{x}\sqrt{x}$ are both $x^{3/2}$.
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
        let mut ordered = Self::ordered_factors(&factors).peekable();
        // Only collected powers are built here. A run that does not collect
        // keeps its originals, which the tail append re-adds unchanged, so an
        // uncollected product allocates nothing.
        let mut result = Vec::new();
        let mut collected_bases = FixedSet::default();
        let mut rebuild = false;
        while let Some((base, power)) = ordered.next() {
            // Equal bases are adjacent; a run collects only when its total
            // power fits. Exponents become rationals only in a run, which is
            // rare.
            let ratio = |(numerator, denominator): (i64, u64)| {
                RationalType::from_parts(numerator, denominator)
            };
            let mut total = None;
            while let Some((_, next)) = ordered.next_if(|(next, _)| *next == base) {
                total = Some(
                    total
                        .unwrap_or_else(|| Some(ratio(power)))
                        .and_then(|sum: RationalType| sum.checked_add(&ratio(next))),
                );
            }
            if let Some(Some(collected_power)) = total {
                let collected = Self::normalize_power(
                    base.clone(),
                    Expr::number(Number::from_rational(collected_power)),
                );
                rebuild |= !matches!(collected.node(), Self::Pow(collected_base, _) if collected_base == base);
                result.push(collected);
                collected_bases.insert(base.pointer());
            }
        }
        drop(ordered);
        if result.is_empty() {
            return (factors, false);
        }
        result.extend(
            factors
                .iter()
                .filter(|factor| {
                    Self::rational_factor(factor)
                        .is_none_or(|(base, _)| !collected_bases.contains(&base.pointer()))
                })
                .cloned(),
        );
        Self::sort_operands(&mut result);
        (result, rebuild)
    }

    /// Normalize $x^{1/2}=\sqrt{x}$ and $e^x=\exp(x)$.
    /// Exact nonzero literals with rational exponents $p/q > 1$ split as
    /// $b^{p/q} = b^{k} b^{r/q}$ for $p = k q + r$, $0 < r < q$, when $b^k$
    /// folds, and split $b^{p/q} = \sqrt{b} \, b^{p/q - 1/2}$ when $p/q > 1/2$
    /// and $\sqrt{b}$ folds exactly, so literal roots associate across products
    /// and integer powers.
    /// An integer power $n > 1$ of a power with an exact positive rational
    /// exponent $a$ uses $(x^a)^n=x^{na}$ when the count fits, and
    /// $(\sqrt{x})^n=x^{n/2}$.
    /// Known negative real exponents use reciprocals. Unknown and complex signs
    /// remain symbolic; zero powers retain their original domain exclusions.
    pub fn normalize_power(mut base: Expr, mut exponent: Expr) -> Expr {
        loop {
            let ratio = exponent.exact_ratio();
            if ratio == Some((1, 1)) {
                return base;
            }
            if matches!(base.node(), Self::Euler) {
                return FunctionDefinition::construct(
                    BuiltinFunction::Exp.function(),
                    vec![exponent],
                );
            }
            if ratio == Some((1, 2)) {
                return FunctionDefinition::construct(BuiltinFunction::Sqrt.function(), vec![base]);
            }
            if let Some((count, 1)) = ratio
                && count > 1
            {
                match base.node() {
                    Self::Symbol(_)
                    | Self::Pi
                    | Self::Sum(_)
                    | Self::Poly(_)
                    | Self::Derivative { .. } => {
                        return Expr::intern(Self::Pow(base, exponent));
                    }
                    Self::Product(_) | Self::Div(..) => {
                        return Self::distribute_power(&base, &exponent);
                    }
                    Self::Function { name, args } => {
                        if *name == BuiltinFunction::Sqrt.function()
                            && let [inner] = args.as_slice()
                        {
                            exponent = Expr::number(Number::from_rational(
                                RationalType::from_parts(count, 2),
                            ));
                            base = inner.clone();
                            continue;
                        }
                        return Expr::intern(Self::Pow(base, exponent));
                    }
                    // $(x^{a})^{n} = e^{n a \operatorname{Log} x} = x^{n a}$
                    // for an integer $n$ and any exact positive rational $a$.
                    Self::Pow(inner, previous) => {
                        if let Some((numerator, denominator)) = previous.exact_ratio()
                            && numerator > 0
                            && let Some(combined) = count.checked_mul(numerator)
                        {
                            exponent = Expr::number(Number::from_rational(
                                RationalType::from_parts(combined, denominator),
                            ));
                            base = inner.clone();
                            continue;
                        }
                        return Expr::intern(Self::Pow(base, exponent));
                    }
                    Self::Number(_) | Self::ImaginaryUnit | Self::Euler => {}
                }
            }
            if let Some(value) = base.literal()
                && value.value().is_finite()
            {
                if let Some(power) = exponent.literal()
                    && power.value().is_finite()
                    && let Some(folded) = value.pow(power)
                {
                    return Expr::number(folded);
                }
                if !value.has_float()
                    && !value.value().is_zero()
                    && let Some((numerator, denominator)) = ratio
                    && let Some(split) =
                        Self::split_literal_rational_power(&base, value, numerator, denominator)
                {
                    return split;
                }
            }
            if let Some(denominator) = Self::negative_power(&base, &exponent, ratio) {
                return Self::normalize_quotient(Expr::number(1), denominator);
            }
            return Expr::intern(Self::Pow(base, exponent));
        }
    }

    /// Normalize $(aA)/(bB)=(a/b)(A/B)$, keeping exact coefficients outside.
    /// Coefficient division requires a nonzero divisor and a representable result.
    /// Denominator quotients stay intact: $1/(1/x)$ must retain $x\ne0$.
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

    /// Read $c(A/B)$ as $(cA,B)$ for iterative denominator collection.
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

    /// Split a finite nonzero exact literal power $b^{p/q}$ ($q > 1$) using
    /// principal $b^{a + c} = b^a b^c$: extract $b^{\lfloor p/q \rfloor}$ when
    /// it folds, and extract an exact $\sqrt{b}$ when $p/q \in (1/2, 1)$.
    fn split_literal_rational_power(
        base: &Expr,
        value: &NumericValue,
        numerator: i64,
        denominator: u64,
    ) -> Option<Expr> {
        if denominator <= 1 {
            return None;
        }
        let positive_numerator = u64::try_from(numerator).ok()?;
        if positive_numerator >= denominator {
            let quotient = positive_numerator.checked_div(denominator)?;
            let remainder = positive_numerator.checked_rem(denominator)?;
            if remainder == 0 {
                return None;
            }
            let whole_exponent = i64::try_from(quotient).ok()?;
            let fractional_numerator = i64::try_from(remainder).ok()?;
            let whole = value.pow(&NumericValue::new(Number::from_int(whole_exponent)))?;
            let fractional = Self::normalize_power(
                base.clone(),
                Expr::number(Number::from_rational(RationalType::from_parts(
                    fractional_numerator,
                    denominator,
                ))),
            );
            return Some(Expr::product(vec![Expr::number(whole), fractional]));
        }
        let half = denominator.checked_div(2)?;
        if positive_numerator <= half {
            return None;
        }
        let root = value
            .evaluate(UnaryRule::Sqrt)
            .filter(|candidate| candidate.is_exact() && candidate.is_finite())?;
        let (rest_numerator, rest_denominator) = if denominator & 1 == 0 {
            let diff = i64::try_from(positive_numerator.checked_sub(half)?).ok()?;
            (diff, denominator)
        } else {
            let diff = i64::try_from(
                positive_numerator
                    .checked_mul(2)?
                    .checked_sub(denominator)?,
            )
            .ok()?;
            (diff, denominator.checked_mul(2)?)
        };
        let fractional = Self::normalize_power(
            base.clone(),
            Expr::number(Number::from_rational(RationalType::from_parts(
                rest_numerator,
                rest_denominator,
            ))),
        );
        Some(Expr::product(vec![Expr::number(root), fractional]))
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
    fn negative_power(base: &Expr, exponent: &Expr, ratio: Option<(i64, u64)>) -> Option<Expr> {
        if let Some((numerator, denominator)) = ratio {
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
                    Some(Self::integer_power(
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

    /// The collectable factors as `(base, exponent)`, equal bases adjacent.
    fn ordered_factors(factors: &[Expr]) -> impl Iterator<Item = (&Expr, (i64, u64))> {
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
                    (class == kind).then_some((base, power))
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
    /// `(numerator, denominator)`: $\sqrt{b}$ is $b^{1/2}$, a power
    /// $b^{p/q}$ has its own, and any other nonliteral factor $f$ is
    /// $f^{1}$. Literals and other powers are not collected.
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
