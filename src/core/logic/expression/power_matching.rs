//! Exact rational power groups preserving principal branches and zero exclusions.

use alloc::{vec, vec::Vec};

use super::{
    BuiltinFunction, ConcreteMatcher, ExactSources, Expr, Node, NonZeroI128, NonZeroU64,
    NumericValue, PartialSelection, PowerProjection, RemainingOperand, Selection,
};

impl ConcreteMatcher<'_> {
    /// Consume \(b^{q r+s}=(b^r)^q b^s\) for integral \(q\).
    /// Rational exponents use the same principal base. A fractional outer power
    /// never exposes its base's own powers, preventing branch-changing matches.
    pub fn select_powers(&self, expression: &Expr) -> Option<Selection> {
        let target = PowerProjection::new(expression)?;
        let mut remaining = target.exponent.get();
        let mut denominator = target.denominator;
        let mut retained_nonzero = false;
        let mut inserted = Vec::new();
        let mut apply = |source: &Expr, destination: &Expr| {
            let Some(required) = PowerProjection::new(source) else {
                return;
            };
            if required.base != target.base
                || required.nonzero && !target.nonzero
                || !target.nonzero && !target.projected && !required.projected
            {
                return;
            }
            let Some((copies, residual, residual_denominator)) =
                required.decompose(remaining, denominator)
            else {
                return;
            };
            let preserves_zero_exclusion =
                retained_nonzero || required.nonzero || copies.get() < 0 || residual < 0;
            if target.nonzero && !preserves_zero_exclusion {
                return;
            }
            inserted.push(Expr::integer_power(destination, copies.get()));
            remaining = residual;
            denominator = residual_denominator;
            retained_nonzero |= required.nonzero || copies.get() < 0;
        };
        match self.exact {
            ExactSources::One(entry) => apply(entry.from(), entry.to()),
            ExactSources::Many(entries) => {
                for (source, destination) in entries.iter() {
                    apply(source, destination);
                }
            }
        }
        if inserted.is_empty() {
            return None;
        }
        Some(Selection::Partial(PartialSelection {
            sum: false,
            remainder: NonZeroI128::new(remaining).map_or_else(Vec::new, |exponent| {
                vec![RemainingOperand::Power {
                    base: target.base.clone(),
                    exponent,
                    denominator,
                }]
            }),
            inserted,
        }))
    }
}

impl<'expr> PowerProjection<'expr> {
    pub fn new(expression: &'expr Expr) -> Option<Self> {
        let mut base = expression;
        let mut exponent = 1_i128;
        let mut denominator = NonZeroU64::MIN;
        let mut nonzero = false;
        let mut projected = false;
        let mut nested = false;
        loop {
            if denominator.get() != 1 {
                break;
            }
            let next = match base.node() {
                Node::Pow(inner, power) => power
                    .literal()
                    .and_then(NumericValue::exact_ratio)
                    .map(|(numerator, divisor)| (inner, numerator, divisor)),
                Node::Div(unit, inner) if unit.is_one_num() => Some((inner, -1, 1)),
                Node::Function { name, args }
                    if *name == BuiltinFunction::Sqrt.function()
                        && let [inner] = args.as_slice() =>
                {
                    Some((inner, 1, 2))
                }
                Node::Number(_)
                | Node::ImaginaryUnit
                | Node::Pi
                | Node::Euler
                | Node::Symbol(_)
                | Node::Function { .. }
                | Node::Sum(_)
                | Node::Poly(_)
                | Node::Product(_)
                | Node::Div(..)
                | Node::Derivative { .. } => None,
            };
            let Some((inner, numerator, divisor)) = next else {
                break;
            };
            if numerator == 0 {
                break;
            }
            let Some(combined) = exponent.checked_mul(i128::from(numerator)) else {
                break;
            };
            let nonzero_divisor = NonZeroU64::new(divisor)?;
            let (reduced, reduced_denominator) = Self::reduce(combined, nonzero_divisor)?;
            nonzero |= numerator < 0;
            projected |= nested || numerator < 0 || reduced_denominator.get() != 1;
            exponent = reduced;
            denominator = reduced_denominator;
            base = inner;
            nested = true;
        }
        base.literal().is_none().then_some(Self {
            base,
            exponent: NonZeroI128::new(exponent)?,
            denominator,
            nonzero,
            projected,
        })
    }

    /// Find an integral number of groups using checked common-denominator counts.
    fn decompose(
        &self,
        available: i128,
        denominator: NonZeroU64,
    ) -> Option<(NonZeroI128, i128, NonZeroU64)> {
        let (numerator, divisor, common) = if denominator == self.denominator {
            (available, self.exponent.get(), denominator)
        } else {
            let mut first = denominator.get();
            let mut second = self.denominator.get();
            while second != 0 {
                let remainder = first.checked_rem(second)?;
                first = second;
                second = remainder;
            }
            let left_scale = self.denominator.get().checked_div(first)?;
            let right_scale = denominator.get().checked_div(first)?;
            (
                available.checked_mul(i128::from(left_scale))?,
                self.exponent.get().checked_mul(i128::from(right_scale))?,
                NonZeroU64::new(denominator.get().checked_mul(left_scale)?)?,
            )
        };
        let native = match (i64::try_from(numerator), i64::try_from(divisor)) {
            (Ok(native_numerator), Ok(native_divisor)) => native_numerator
                .checked_div(native_divisor)
                .zip(native_numerator.checked_rem(native_divisor))
                .map(|(quotient, remainder)| (i128::from(quotient), i128::from(remainder))),
            _ => None,
        };
        let (copies, residual) = native.or_else(|| {
            numerator
                .checked_div(divisor)
                .zip(numerator.checked_rem(divisor))
        })?;
        let count = NonZeroI128::new(copies)?;
        let (reduced, reduced_denominator) = Self::reduce(residual, common)?;
        Some((count, reduced, reduced_denominator))
    }

    /// Reduce widened traversal counts; numeric literals still use `Number`.
    fn reduce(numerator: i128, denominator: NonZeroU64) -> Option<(i128, NonZeroU64)> {
        if denominator == NonZeroU64::MIN {
            return Some((numerator, denominator));
        }
        let mut first = numerator.unsigned_abs();
        let mut second = u128::from(denominator.get());
        while second != 0 {
            let remainder = first.checked_rem(second)?;
            first = second;
            second = remainder;
        }
        let divisor = u64::try_from(first).ok()?;
        Some((
            numerator.checked_div(i128::from(divisor))?,
            NonZeroU64::new(denominator.get().checked_div(divisor)?)?,
        ))
    }
}
