//! Exact rational power groups preserving principal branches and zero exclusions.

use core::num::{NonZeroI64, NonZeroI128, NonZeroU64};

use alloc::{vec, vec::Vec};

use num_anafis::NonZeroU64Ext;

use super::{
    BuiltinFunction, ConcreteMatcher, ExactIdentity, ExactSources, Expr, Node, PartialSelection,
    RemainingOperand, Selection,
};

pub struct PowerProjection<'expr> {
    pub base: &'expr Expr,
    pub exponent: NonZeroI128,
    pub denominator: NonZeroU64,
    pub nonzero: bool,
    pub projected: bool,
}

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
        match &self.exact {
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
                    .exact_ratio()
                    .map(|(numerator, divisor)| (inner, numerator, divisor)),
                Node::Div(unit, inner) if unit.is_exact_one() => Some((inner, -1, 1)),
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
            let (reduced, reduced_denominator) = NonZeroU64::new(divisor)?.reduce(combined);
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
    ///
    /// With \(g = \gcd(d, s)\) and \(L = d\,(s/g)\), the available exponent
    /// \(a/d\) and the required exponent \(e/s\) are the integer counts
    /// \(a\,(s/g)\) and \(e\,(d/g)\) of \(1/L\).
    fn decompose(
        &self,
        available: i128,
        denominator: NonZeroU64,
    ) -> Option<(NonZeroI128, i128, NonZeroU64)> {
        let (numerator, divisor, common) = if denominator == self.denominator {
            (available, self.exponent, denominator)
        } else {
            let (required_scale, available_scale) = denominator.cofactors(self.denominator);
            (
                available.checked_mul(i128::from(available_scale.get()))?,
                self.exponent
                    .checked_mul(NonZeroI128::from(required_scale))?,
                denominator.checked_mul(available_scale)?,
            )
        };
        // Counts that fit 64 bits divide in one native instruction instead of a
        // 128-bit library call. The divisor is nonzero, so `MIN / -1` is the
        // only quotient that fails.
        let native = match (i64::try_from(numerator), NonZeroI64::try_from(divisor)) {
            (Ok(native_numerator), Ok(native_divisor)) => native_numerator
                .checked_div(native_divisor.get())
                .zip(native_numerator.checked_rem(native_divisor.get()))
                .map(|(quotient, remainder)| (i128::from(quotient), i128::from(remainder))),
            _ => None,
        };
        let (copies, residual) = native.or_else(|| {
            numerator
                .checked_div(divisor.get())
                .zip(numerator.checked_rem(divisor.get()))
        })?;
        let count = NonZeroI128::new(copies)?;
        let (reduced, reduced_denominator) = common.reduce(residual);
        Some((count, reduced, reduced_denominator))
    }
}
