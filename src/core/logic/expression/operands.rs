//! Counted arithmetic matching without expanding powers or collected terms.

use core::slice::from_ref;

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
        if sum && explicit.is_some() && matches!(self.exact, ExactSources::One(_)) {
            // Canonical addition has one collected coefficient per base. An
            // explicit match consumes every required base for this sole source.
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
                candidate.same_node(expected).then_some(consumed)
            })?;
            selected.push(found);
        }
        (!selected.is_empty()).then_some(selected)
    }

    /// Construct one requested virtual operand while proving disjoint consumption.
    pub fn candidate<'used>(
        &self,
        index: usize,
        demand: OperandDemand,
        literal: Option<&Expr>,
        used: impl Iterator<Item = &'used OperandMatch>,
        denominator: bool,
    ) -> Option<(Expr, OperandMatch)> {
        let slot = self.slots.get(index).filter(|slot| {
            if (slot.side == FactorSide::Denominator) != denominator {
                return false;
            }
            if slot.coefficient {
                !slot.changed
            } else {
                slot.count > 0
            }
        })?;
        if slot.coefficient {
            if !matches!(demand, OperandDemand::Atom | OperandDemand::Whole)
                || used.into_iter().any(|selected| match selected {
                    OperandMatch::Count {
                        index: selected_index,
                        ..
                    }
                    | OperandMatch::Coefficient {
                        index: selected_index,
                        ..
                    } => *selected_index == index,
                })
            {
                return None;
            }
            let projected = literal
                .filter(|value| {
                    value.literal().is_some_and(|number| {
                        number.value().is_finite() && !number.value().is_zero()
                    })
                })
                .unwrap_or(&slot.base);
            return Some((
                projected.clone(),
                OperandMatch::Coefficient {
                    index,
                    divisor: projected.clone(),
                },
            ));
        }
        let mut remaining = slot.count;
        for selected in used {
            if let OperandMatch::Count {
                index: selected_index,
                amount,
            } = selected
                && *selected_index == index
            {
                remaining = remaining.checked_sub(*amount)?;
            }
        }
        let count = match demand {
            OperandDemand::Atom => 1,
            OperandDemand::Split(count) if count > 0 && count < remaining => count,
            OperandDemand::Whole => remaining,
            OperandDemand::Power(count) if !self.sum => count,
            OperandDemand::Copies(count) if self.sum => count,
            OperandDemand::Root { exponent, block } if !self.sum => exponent.checked_mul(block)?,
            OperandDemand::Split(_)
            | OperandDemand::Power(_)
            | OperandDemand::Copies(_)
            | OperandDemand::Root { .. } => {
                return None;
            }
        };
        if count <= 0 || count > remaining {
            return None;
        }
        let projected = if let OperandDemand::Root { exponent, block } = demand
            && block > 1
        {
            slot.base.pow(block).pow(exponent)
        } else if count == slot.count && !slot.changed {
            slot.original.clone()
        } else if count == 1 && !slot.negative {
            slot.base.clone()
        } else if self.sum {
            let coefficient = if slot.negative {
                count.saturating_neg()
            } else {
                count
            };
            Expr::product(vec![Expr::number(coefficient), slot.base.clone()])
        } else {
            slot.base.pow(count)
        };
        Some((
            projected,
            OperandMatch::Count {
                index,
                amount: count,
            },
        ))
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
        .rebuild(Clone::clone)
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

    fn remainder(&self) -> Vec<RemainingOperand> {
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

    /// Borrow original slots in deterministic canonical order.
    pub fn originals(&self) -> impl ExactSizeIterator<Item = &Expr> {
        self.slots.iter().map(|slot| &slot.original)
    }
}
