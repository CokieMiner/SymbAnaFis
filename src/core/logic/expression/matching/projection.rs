//! Borrowed multiplicative views and counted arithmetic slots.

use core::{num::NonZeroU64, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::NonZeroU64Ext;

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactIdentity, Expr, FactorSide, Node,
    PowerProjection, WeightedOperand,
};

impl AlgebraicOperands {
    /// Project positive powers, scalar prefixes, and signed integer addends.
    /// For a product, `units` holds the `ConcreteMatcher` unit bits of the sides
    /// that receive a unit coefficient slot when they have no literal.
    pub fn new(expression: &Expr, sum: bool, units: u8) -> Option<Self> {
        let additive = if sum {
            Some(expression.additive_operands()?)
        } else {
            None
        };
        let originals = additive
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|value| (value, FactorSide::Numerator))
            .chain(Self::factors(expression).filter(|_| !sum));
        let mut slots: Vec<_> = originals
            .map(|(original, side)| {
                let mut slot = WeightedOperand {
                    original: original.clone(),
                    base: original.clone(),
                    count: 1,
                    denominator: NonZeroU64::MIN,
                    coefficient: false,
                    negative: false,
                    changed: false,
                    side,
                };
                if sum {
                    if let Node::Product(factors) = original.node()
                        && let Some((coefficient, rest)) = factors.split_first()
                        && let Some(integer) = Self::integer(coefficient)
                        && let Some(count) = integer.checked_abs().filter(|count| *count > 0)
                        && !rest.is_empty()
                    {
                        slot.count = count;
                        slot.base = if let [base] = rest {
                            base.clone()
                        } else {
                            Expr::product(rest.to_vec())
                        };
                        slot.negative = integer < 0;
                    }
                } else if let Some((base, count)) = original.positive_factor() {
                    slot.base = base.clone();
                    slot.count = count;
                } else if original
                    .literal()
                    .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
                {
                    slot.coefficient = true;
                }
                slot
            })
            .collect();
        if units != 0 && !sum {
            for side in [FactorSide::Numerator, FactorSide::Denominator] {
                let mask = match side {
                    FactorSide::Numerator => ConcreteMatcher::NUMERATOR_UNIT,
                    FactorSide::Denominator => ConcreteMatcher::DENOMINATOR_UNIT,
                };
                if units & mask != 0
                    && !slots
                        .iter()
                        .any(|slot| slot.side == side && slot.original.literal().is_some())
                {
                    let unit = Expr::number(1);
                    slots.push(WeightedOperand {
                        original: unit.clone(),
                        base: unit,
                        count: 1,
                        denominator: NonZeroU64::MIN,
                        coefficient: true,
                        negative: false,
                        changed: false,
                        side,
                    });
                }
            }
        }
        Some(Self { sum, slots })
    }

    /// Count positive exact powers in units of a common rational step.
    ///
    /// With \(g = \gcd(D, q)\) and \(L = \operatorname{lcm}(D, q) = q\,(D/g)\),
    /// the power \(b^{p/q}\) becomes \(p\,(D/g)\) units of \(b^{1/L}\).
    /// Principal bases are unchanged, and powers containing a pole keep their
    /// opaque slot.
    pub fn refine_powers(&mut self, denominator: NonZeroU64) {
        for slot in &mut self.slots {
            if slot.coefficient {
                continue;
            }
            let Some(power) = PowerProjection::new(&slot.original) else {
                continue;
            };
            if power.nonzero || power.exponent.get() <= 0 {
                continue;
            }
            let (scale, _) = denominator.cofactors(power.denominator);
            let Some(common) = power.denominator.checked_mul(scale) else {
                continue;
            };
            let Some(count) = power
                .exponent
                .get()
                .checked_mul(i128::from(scale.get()))
                .and_then(|count| i64::try_from(count).ok())
            else {
                continue;
            };
            slot.base = power.base.clone();
            slot.count = count;
            slot.denominator = common;
        }
    }

    /// Borrow ordinary factors; project numerator quotients only when present.
    pub fn factors(expression: &Expr) -> impl Iterator<Item = (&Expr, FactorSide)> {
        let borrowed = match expression.node() {
            Node::Product(factors)
                if !factors
                    .iter()
                    .any(|factor| matches!(factor.node(), Node::Div(..))) =>
            {
                Some(factors.as_slice())
            }
            Node::Product(_) | Node::Div(..) => None,
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Pow(..)
            | Node::Derivative { .. } => Some(from_ref(expression)),
        };
        let mut result = Vec::new();
        if borrowed.is_none() {
            let mut pending = vec![(expression, FactorSide::Numerator)];
            while let Some((value, side)) = pending.pop() {
                match value.node() {
                    Node::Product(factors) => {
                        pending.extend(factors.iter().rev().map(|factor| (factor, side)));
                    }
                    Node::Div(numerator, denominator) if side == FactorSide::Numerator => {
                        pending.extend([(denominator, FactorSide::Denominator), (numerator, side)]);
                    }
                    Node::Number(_)
                    | Node::ImaginaryUnit
                    | Node::Pi
                    | Node::Euler
                    | Node::Symbol(_)
                    | Node::Function { .. }
                    | Node::Sum(_)
                    | Node::Poly(_)
                    | Node::Div(..)
                    | Node::Pow(..)
                    | Node::Derivative { .. } => result.push((value, side)),
                }
            }
        }
        borrowed
            .unwrap_or(&[])
            .iter()
            .map(|value| (value, FactorSide::Numerator))
            .chain(result)
    }
}
