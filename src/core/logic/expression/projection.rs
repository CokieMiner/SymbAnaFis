//! Borrowed multiplicative views and counted arithmetic slots.

use core::slice::from_ref;

use alloc::{vec, vec::Vec};

use super::{AdditiveOperands, AlgebraicOperands, Expr, FactorSide, Node, WeightedOperand};

impl AlgebraicOperands {
    /// Project positive powers, scalar prefixes, and signed integer addends.
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
                } else if let Some((base, count)) = Node::positive_factor(original) {
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
                    FactorSide::Numerator => 4,
                    FactorSide::Denominator => 8,
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
