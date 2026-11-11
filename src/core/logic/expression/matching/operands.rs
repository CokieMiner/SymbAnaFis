//! Counted arithmetic matching without expanding powers or collected terms.

#![expect(
    unsafe_code,
    reason = "a selection made by these operands names only their own slots, so applying it needs no bounds checks"
)]

use core::{
    num::{NonZeroI128, NonZeroU64},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactIdentity, ExactSources, Expr,
    FactorSide, LinearOperands, Node, NumericValue, OperandDemand, OperandKind, OperandMatch,
    PartialSelection, PowerProjection, RemainingOperand, Selection,
};

impl ConcreteMatcher<'_> {
    /// Prefer explicit original matches, then consume compressed arithmetic groups.
    pub fn select(&self, expression: &Expr) -> Option<Selection> {
        let explicit = match self.select_explicit(expression) {
            Some(Selection::Whole(replacement)) => return Some(Selection::Whole(replacement)),
            Some(Selection::Partial(partial)) => Some(partial),
            None => None,
        };
        if self.families & Self::POWERS != 0
            && let Some(selected) = self.select_powers(expression)
        {
            return Some(selected);
        }
        let sum = matches!(expression.node(), Node::Sum(_) | Node::Poly(_));
        let units = self.families & (Self::NUMERATOR_UNIT | Self::DENOMINATOR_UNIT);
        let family = if sum {
            Self::ADDITIVE
        } else {
            Self::MULTIPLICATIVE
        };
        if matches!(expression.node(), Node::Number(_) | Node::ImaginaryUnit)
            || self.families & family == 0
            || !sum
                && units == 0
                && !matches!(
                    expression.node(),
                    Node::Product(_) | Node::Pow(..) | Node::Div(..)
                )
        {
            return explicit.map(Selection::Partial);
        }
        let remaining: Option<Vec<Expr>> = explicit.as_ref().map(|partial| {
            partial
                .remainder
                .iter()
                .filter_map(|operand| {
                    if let RemainingOperand::Original(original) = operand {
                        Some(original.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });
        if sum {
            // A remainder of two or more terms is already normalized, so it is
            // projected as a list; one term keeps the shape rules of `new`.
            let linear = match remaining.as_deref() {
                None => LinearOperands::new(expression),
                Some([]) => None,
                Some([single]) => LinearOperands::new(single),
                Some(terms) => Some(LinearOperands::from_terms(terms, None)),
            };
            return self.select_linear(linear, explicit);
        }
        // A remainder of two or more factors is already normalized, so it is
        // projected as a list; one factor keeps the shape rules of `new`.
        let projected = remaining.as_deref().map_or_else(
            || AlgebraicOperands::new(expression, sum, units),
            |factors| {
                let direct = if factors.len() > 1 {
                    AlgebraicOperands::from_operands(factors, sum, units)
                } else {
                    None
                };
                direct.or_else(|| {
                    AlgebraicOperands::new(&Expr::product(factors.to_vec()), sum, units)
                })
            },
        );
        let Some(mut operands) = projected else {
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
        match &self.exact {
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
    /// The number of projected operands a search may select from.
    pub const fn len(&self) -> usize {
        self.slots.len()
    }

    /// Divide a projected expression by a proved finite nonzero scalar.
    pub fn divide_scalar(expression: &Expr, divisor: &Expr) -> Option<Expr> {
        let denominator = divisor.literal()?;
        if !denominator.value().is_finite() || denominator.value().is_zero() {
            return None;
        }
        if let Some(numerator) = expression.literal()
            && numerator.value().is_finite()
        {
            return Some(Node::scalar_quotient(numerator, denominator, 1));
        }
        Some(Expr::product(vec![
            Node::scalar_quotient(&NumericValue::new(Number::one()), denominator, 1),
            expression.clone(),
        ]))
    }

    /// Remove one manifest factor group without dividing by symbolic values.
    /// The owned remainder retains the original inputs for simultaneous traversal.
    pub fn remove_once(expression: &Expr, required: &Expr) -> Option<Self> {
        let mut operands = Self::new(expression, false, 0)?;
        for selected in operands.concrete(required)? {
            // SAFETY: `concrete` selects indices of these operands' slots.
            let slot = unsafe { operands.slots.get_unchecked_mut(selected.index) };
            slot.changed = true;
            match selected.kind {
                // Selection bounds each amount by the count left after earlier ones.
                OperandKind::Count { amount } => {
                    slot.count = slot.count.saturating_sub(amount);
                }
                OperandKind::Coefficient { divisor } => {
                    slot.base = Node::scalar_quotient(slot.base.literal()?, &divisor, 1);
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
            // Every projection of a slot is its base, a counted multiple of
            // that base, a power of it, or its original, and each of those
            // reads the base of `expected` the same way: as the term without
            // its leading integer coefficient, as the factor without its
            // positive integer power, or as the base of its power group. A
            // slot whose base is none of those readings cannot produce
            // `expected`, so the scan rejects it without building a candidate.
            // A coefficient slot projects any finite nonzero literal.
            let base = Self::projection(expected, self.sum).map(|(base, _, _)| base);
            let power = PowerProjection::new(expected).map(|projection| projection.base.clone());
            let literal = expected
                .literal()
                .is_some_and(|value| value.value().is_finite() && !value.value().is_zero());
            let found = self.slots.iter().enumerate().find_map(|(index, slot)| {
                if !(slot.coefficient && literal
                    || slot.base == *expected
                    || base.as_ref() == Some(&slot.base)
                    || power.as_ref() == Some(&slot.base))
                {
                    return None;
                }
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

    /// Consume the selected units and divide the selected coefficients.
    pub fn consume(&mut self, selected: &[OperandMatch]) -> i64 {
        // A match selects at most one unit group per pattern node, so the
        // totals come from `selected` itself. A table over every slot would
        // zero one entry per operand on every match of a wide sum.
        let units = |index: usize| {
            selected
                .iter()
                .filter(|operand| operand.index == index)
                .filter_map(|operand| match operand.kind {
                    OperandKind::Count { amount } => Some(amount),
                    OperandKind::Coefficient { .. } => None,
                })
                .fold(0_i64, i64::saturating_add)
        };
        let repetitions = selected
            .iter()
            .filter_map(|operand| match operand.kind {
                OperandKind::Count { .. } => self
                    .slots
                    .get(operand.index)
                    .map(|slot| slot.count.checked_div(units(operand.index)).unwrap_or(0)),
                OperandKind::Coefficient { .. } => None,
            })
            .min()
            .unwrap_or(1)
            .max(1);
        for (position, operand) in selected.iter().enumerate() {
            if !matches!(operand.kind, OperandKind::Count { .. }) {
                continue;
            }
            // The first occurrence of a slot carries the total of its units.
            if selected
                .iter()
                .take(position)
                .any(|earlier| earlier.index == operand.index)
            {
                continue;
            }
            if let Some(slot) = self.slots.get_mut(operand.index) {
                // repetitions <= original/units proves both operations fit.
                slot.count = slot
                    .count
                    .saturating_sub(units(operand.index).saturating_mul(repetitions));
                slot.changed = true;
            }
        }
        for operand in selected {
            if let OperandKind::Coefficient { divisor } = &operand.kind
                && let Some(slot) = self.slots.get_mut(operand.index)
            {
                // A scalar slot is selected once. Its new quotient is not a candidate.
                let Some(numerator) = slot.base.literal() else {
                    continue;
                };
                slot.base = Node::scalar_quotient(numerator, divisor, repetitions);
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
        .rebuild()
    }

    /// `count` copies of `expression`: $n e$ in a sum, or $e^n$ applied
    /// factor by factor in a product, in space proportional to the stored factors.
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
                    if let Some((base, exponent)) = factor.positive_factor()
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
                    !slot.base.is_exact_one()
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
                .map_or(OperandDemand::Single, OperandDemand::Copies),
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
            | Node::Derivative { .. } => OperandDemand::Single,
        }
    }

    /// A stored exact real integer; an integral float does not qualify.
    pub fn integer(expression: &Expr) -> Option<i64> {
        expression
            .literal()
            .and_then(NumericValue::gaussian)
            .filter(|(_, imaginary)| *imaginary == 0)
            .map(|(real, _)| real)
    }

    /// Read a stored exact real exponent with its positive denominator.
    pub fn ratio(expression: &Expr) -> Option<(i64, NonZeroU64)> {
        let (numerator, denominator) = expression.exact_ratio()?;
        Some((numerator, NonZeroU64::new(denominator)?))
    }

    /// Borrow original slots in deterministic canonical order.
    pub fn originals(&self) -> impl ExactSizeIterator<Item = &Expr> {
        self.slots.iter().map(|slot| &slot.original)
    }
}
