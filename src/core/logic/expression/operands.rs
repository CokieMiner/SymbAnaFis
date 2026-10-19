//! Counted arithmetic matching without expanding powers or collected terms.

use core::{
    num::{NonZeroI128, NonZeroU64},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactSources, Expr, FactorSide, Node,
    NumericValue, OperandDemand, OperandMatch, PartialSelection, RemainingOperand, Selection,
};

impl ConcreteMatcher<'_> {
    /// Prefer explicit original matches, then consume compressed arithmetic groups.
    pub fn select(&self, expression: &Expr) -> Option<Selection> {
        let explicit = match self.select_explicit(expression) {
            Some(Selection::Whole(replacement)) => return Some(Selection::Whole(replacement)),
            Some(Selection::Partial(partial)) => Some(partial),
            None => None,
        };
        if self.arithmetic & 16 != 0
            && let Some(selected) = self.select_powers(expression)
        {
            return Some(selected);
        }
        let sum = matches!(expression.node(), Node::Sum(_) | Node::Poly(_));
        if matches!(expression.node(), Node::Number(_) | Node::ImaginaryUnit)
            || !sum
                && self.arithmetic & 0b1100 == 0
                && !matches!(
                    expression.node(),
                    Node::Product(_) | Node::Pow(..) | Node::Div(..)
                )
        {
            return explicit.map(Selection::Partial);
        }
        if self.arithmetic & (if sum { 1 } else { 2 }) == 0 {
            return explicit.map(Selection::Partial);
        }
        let remaining = explicit.as_ref().map(|partial| {
            let originals = partial
                .remainder
                .iter()
                .filter_map(|operand| {
                    if let RemainingOperand::Original(original) = operand {
                        Some(original.clone())
                    } else {
                        None
                    }
                })
                .collect();
            if sum {
                Expr::sum(originals)
            } else {
                Expr::product(originals)
            }
        });
        if sum {
            return self.select_linear(remaining.as_ref().unwrap_or(expression), explicit);
        }
        let Some(mut operands) = AlgebraicOperands::new(
            remaining.as_ref().unwrap_or(expression),
            sum,
            self.arithmetic & 0b1100,
        ) else {
            return explicit.map(Selection::Partial);
        };
        let mut inserted = Vec::new();
        let mut apply = |source: &Expr, destination: &Expr| {
            let eligible = AlgebraicOperands::mode(source) == Some(sum);
            if eligible {
                while let Some(selected) = operands.concrete(source) {
                    let repetitions = operands.consume(&selected);
                    inserted.push(AlgebraicOperands::repeat(
                        destination.clone(),
                        repetitions,
                        sum,
                    ));
                }
            }
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
            return explicit.map(Selection::Partial);
        }
        if let Some(partial) = explicit {
            inserted.extend(partial.inserted);
        }
        Some(Selection::Partial(PartialSelection {
            sum,
            remainder: operands.remainder(),
            inserted,
        }))
    }
}

impl AlgebraicOperands {
    /// Divide a projected expression by a proved finite nonzero scalar.
    pub fn divide_scalar(expression: &Expr, divisor: &Expr) -> Option<Expr> {
        let denominator = divisor.literal()?;
        if !denominator.value().is_finite() || denominator.value().is_zero() {
            return None;
        }
        if let Some(numerator) = expression.literal()
            && numerator.value().is_finite()
        {
            return Some(Expr::scalar_quotient(numerator, denominator, 1));
        }
        Some(Expr::product(vec![
            Expr::scalar_quotient(&NumericValue::new(Number::one()), denominator, 1),
            expression.clone(),
        ]))
    }

    /// Remove one manifest factor group without dividing by symbolic values.
    /// The owned remainder retains the original inputs for simultaneous traversal.
    pub fn remove_once(expression: &Expr, required: &Expr) -> Option<Self> {
        let mut operands = Self::new(expression, false, 0)?;
        for selected in operands.concrete(required)? {
            match selected {
                OperandMatch::Count { index, amount } => {
                    let slot = operands.slots.get_mut(index)?;
                    slot.count = slot.count.checked_sub(amount)?;
                    slot.changed = true;
                }
                OperandMatch::Coefficient { index, divisor } => {
                    let slot = operands.slots.get_mut(index)?;
                    slot.base = Expr::scalar_quotient(slot.base.literal()?, divisor.literal()?, 1);
                    slot.changed = true;
                }
            }
        }
        Some(operands)
    }

    /// Find a concrete group by pointer identity in the counted projection.
    pub fn concrete(&self, expression: &Expr) -> Option<Vec<OperandMatch>> {
        let additive = if self.sum {
            Some(expression.additive_operands()?)
        } else {
            None
        };
        let required = additive
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|value| (value, FactorSide::Numerator))
            .chain(Self::factors(expression).filter(|_| !self.sum));
        let mut selected = Vec::new();
        for (expected, side) in required {
            let demand = Self::demand(expected, self.sum);
            let found = (0..self.slots.len()).find_map(|index| {
                let (candidate, consumed) = self.candidate(
                    index,
                    demand,
                    Some(expected),
                    selected.iter(),
                    side == FactorSide::Denominator,
                )?;
                (candidate == *expected).then_some(consumed)
            })?;
            selected.push(found);
        }
        (!selected.is_empty()).then_some(selected)
    }

    /// Consume disjoint original units and divide coefficients through the backend.
    pub fn consume(&mut self, selected: &[OperandMatch]) -> i64 {
        let mut counts = vec![0_i64; self.slots.len()];
        for operand in selected {
            if let OperandMatch::Count { index, amount } = operand
                && let Some(total) = counts.get_mut(*index)
            {
                // Selection proves that the combined amount fits its original count.
                *total = total.saturating_add(*amount);
            }
        }
        let repetitions = self
            .slots
            .iter()
            .zip(&counts)
            .filter(|(_, count)| **count > 0)
            .map(|(slot, count)| slot.count.checked_div(*count).unwrap_or(0))
            .min()
            .unwrap_or(1)
            .max(1);
        for (slot, count) in self.slots.iter_mut().zip(counts) {
            if count > 0 {
                // repetitions <= original/count proves both operations fit.
                slot.count = slot.count.saturating_sub(count.saturating_mul(repetitions));
                slot.changed = true;
            }
        }
        for operand in selected {
            if let OperandMatch::Coefficient { index, divisor } = operand
                && let Some(slot) = self.slots.get_mut(*index)
            {
                // A scalar slot is selected once. Its new quotient is not a candidate.
                let (Some(numerator), Some(denominator)) = (slot.base.literal(), divisor.literal())
                else {
                    continue;
                };
                slot.base = Expr::scalar_quotient(numerator, denominator, repetitions);
                slot.changed = true;
            }
        }
        repetitions
    }

    /// Rebuild the unmatched arithmetic remainder and untouched replacements.
    pub fn finish(self, inserted: Vec<Expr>) -> Expr {
        PartialSelection {
            sum: self.sum,
            remainder: self.remainder(),
            inserted,
        }
        .rebuild(&mut Clone::clone)
    }

    /// Normalize repeated replacements in space proportional to their stored factors.
    pub fn repeat(expression: Expr, count: i64, sum: bool) -> Expr {
        if count == 1 {
            return expression;
        }
        if sum {
            if let Some(terms) = expression.additive_operands() {
                return Expr::sum(
                    terms
                        .iter()
                        .map(|term| Expr::product(vec![Expr::number(count), term.clone()]))
                        .collect(),
                );
            }
            return Expr::product(vec![Expr::number(count), expression]);
        }
        let factors = if let Node::Product(factors) = expression.node() {
            factors.as_slice()
        } else {
            from_ref(&expression)
        };
        Expr::product(
            factors
                .iter()
                .map(|factor| {
                    if let Some(value) = factor.literal()
                        && value.value().is_finite()
                        && !value.has_float()
                        && !value.value().is_zero()
                    {
                        let unit = NumericValue::new(Number::one());
                        let reciprocal = Expr::scalar_quotient(&unit, value, 1);
                        return reciprocal.literal().map_or_else(
                            || Expr::number(value.value().to_float()).pow(count),
                            |inverse| Expr::scalar_quotient(&unit, inverse, count),
                        );
                    }
                    if let Some((base, exponent)) = Node::positive_factor(factor)
                        && let Some(total_power) = exponent.checked_mul(count)
                    {
                        base.pow(total_power)
                    } else {
                        factor.pow(count)
                    }
                })
                .collect(),
        )
    }

    pub fn remainder(&self) -> Vec<RemainingOperand> {
        self.slots
            .iter()
            .filter(|slot| {
                if slot.coefficient {
                    !slot.base.is_one_num()
                } else {
                    slot.count > 0
                }
            })
            .map(|slot| {
                if slot.changed
                    && slot.denominator != NonZeroU64::MIN
                    && let Some(exponent) =
                        NonZeroI128::new(i128::from(if slot.side == FactorSide::Denominator {
                            slot.count.saturating_neg()
                        } else {
                            slot.count
                        }))
                {
                    return RemainingOperand::Power {
                        base: slot.base.clone(),
                        exponent,
                        denominator: slot.denominator,
                    };
                }
                if slot.side == FactorSide::Denominator {
                    return if !slot.changed {
                        RemainingOperand::Divisor(slot.original.clone())
                    } else if slot.coefficient {
                        RemainingOperand::FixedDivisor(slot.base.clone())
                    } else {
                        RemainingOperand::DivisorCounted {
                            base: slot.base.clone(),
                            count: slot.count,
                        }
                    };
                }
                if !slot.changed {
                    RemainingOperand::Original(slot.original.clone())
                } else if slot.coefficient {
                    RemainingOperand::Fixed(slot.base.clone())
                } else {
                    RemainingOperand::Counted {
                        base: slot.base.clone(),
                        count: if slot.negative {
                            slot.count.saturating_neg()
                        } else {
                            slot.count
                        },
                    }
                }
            })
            .collect()
    }

    /// Borrow surviving original inputs, excluding generated numeric quotients.
    pub fn remaining_inputs(&self) -> impl Iterator<Item = &Expr> {
        self.slots.iter().filter_map(|slot| {
            if slot.coefficient {
                (!slot.changed && !slot.base.is_one_num()).then_some(&slot.original)
            } else if slot.count > 0 {
                Some(if slot.changed {
                    &slot.base
                } else {
                    &slot.original
                })
            } else {
                None
            }
        })
    }

    /// Select the arithmetic family supported by a concrete source.
    pub fn mode(expression: &Expr) -> Option<bool> {
        match expression.node() {
            Node::Sum(_) | Node::Poly(_) => Some(true),
            Node::Product(_) | Node::Div(..) => Some(false),
            Node::Pow(_, exponent) if Self::integer(exponent).is_some_and(|power| power > 0) => {
                Some(false)
            }
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Pow(..)
            | Node::Derivative { .. } => None,
        }
    }

    /// Classify a concrete term without introducing fractional or negative powers.
    pub fn demand(expression: &Expr, sum: bool) -> OperandDemand {
        match expression.node() {
            Node::Product(factors) if sum => factors
                .first()
                .and_then(Self::integer)
                .and_then(i64::checked_abs)
                .filter(|count| *count > 0)
                .map_or(OperandDemand::Atom, OperandDemand::Copies),
            Node::Pow(_, exponent) if !sum => Self::integer(exponent)
                .filter(|power| *power > 0)
                .map_or(OperandDemand::Whole, OperandDemand::Power),
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
            | Node::Pow(..)
            | Node::Derivative { .. } => OperandDemand::Atom,
        }
    }

    /// Return only a stored real integer; integral floats do not establish exactness.
    pub fn integer(expression: &Expr) -> Option<i64> {
        expression
            .literal()
            .and_then(NumericValue::gaussian)
            .filter(|(_, imaginary)| *imaginary == 0)
            .map(|(real, _)| real)
    }

    /// Read a stored exact real exponent with its positive denominator.
    pub fn ratio(expression: &Expr) -> Option<(i64, NonZeroU64)> {
        let (numerator, denominator) = expression.literal()?.exact_ratio()?;
        Some((numerator, NonZeroU64::new(denominator)?))
    }

    /// Borrow original slots in deterministic canonical order.
    pub fn originals(&self) -> impl ExactSizeIterator<Item = &Expr> {
        self.slots.iter().map(|slot| &slot.original)
    }
}
