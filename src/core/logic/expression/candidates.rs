//! Virtual operand candidates with disjoint counted consumption.

use alloc::vec;

use num_anafis::{Number, RationalType};

use super::{AlgebraicOperands, Expr, FactorSide, OperandDemand, OperandMatch, WeightedOperand};

impl AlgebraicOperands {
    /// Construct one virtual operand within its remaining original count.
    pub fn candidate<'used>(
        &self,
        index: usize,
        demand: OperandDemand,
        literal: Option<&Expr>,
        used: impl Iterator<Item = &'used OperandMatch>,
        inverse: bool,
    ) -> Option<(Expr, OperandMatch)> {
        let slot = self.slots.get(index).filter(|slot| {
            if (slot.side == FactorSide::Denominator) != inverse {
                return false;
            }
            if slot.coefficient {
                !slot.changed
            } else {
                slot.count > 0
            }
        })?;
        if slot.coefficient {
            return slot.scalar_candidate(index, demand, literal, used);
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
            OperandDemand::Atom => i64::try_from(slot.denominator.get()).ok()?,
            OperandDemand::Split(count) if count > 0 && count < remaining => count,
            OperandDemand::Whole => remaining,
            OperandDemand::Power(count) if !self.sum => {
                count.checked_mul(i64::try_from(slot.denominator.get()).ok()?)?
            }
            OperandDemand::Copies(count) if self.sum => count,
            OperandDemand::Root { exponent, block } if !self.sum => {
                return slot.root_candidate(index, remaining, exponent, block);
            }
            OperandDemand::Rational {
                numerator,
                denominator,
            } if !self.sum => {
                let scale = slot.denominator.get().checked_div(denominator.get())?;
                if slot.denominator.get().checked_rem(denominator.get())? != 0 {
                    return None;
                }
                numerator.checked_mul(i64::try_from(scale).ok()?)?
            }
            OperandDemand::Split(_)
            | OperandDemand::Power(_)
            | OperandDemand::Copies(_)
            | OperandDemand::Root { .. }
            | OperandDemand::Rational { .. } => {
                return None;
            }
        };
        if count <= 0 || count > remaining {
            return None;
        }
        let projected = if matches!(demand, OperandDemand::Rational { .. }) {
            slot.base.clone()
        } else if count == slot.count && !slot.changed {
            slot.original.clone()
        } else if u64::try_from(count).ok() == Some(slot.denominator.get()) && !slot.negative {
            slot.base.clone()
        } else if self.sum {
            let coefficient = if slot.negative {
                count.saturating_neg()
            } else {
                count
            };
            Expr::product(vec![Expr::number(coefficient), slot.base.clone()])
        } else {
            slot.base
                .pow(Number::from_rational(RationalType::from_parts(
                    count,
                    slot.denominator.get(),
                )))
        };
        Some((
            projected,
            OperandMatch::Count {
                index,
                amount: count,
            },
        ))
    }
}

impl WeightedOperand {
    /// Try integral bases before fractional blocks already present in the input.
    /// For integer \(n>0\), \((b^r)^n=b^{rn}\) uses the same principal logarithm.
    fn root_candidate(
        &self,
        index: usize,
        remaining: i64,
        exponent: i64,
        ordinal: i64,
    ) -> Option<(Expr, OperandMatch)> {
        let maximum = u64::try_from(remaining.checked_div(exponent)?).ok()?;
        let whole = maximum.checked_div(self.denominator.get())?;
        let rank = u64::try_from(ordinal).ok()?;
        let units = if rank <= whole {
            rank.checked_mul(self.denominator.get())?
        } else {
            let fractional = rank.checked_sub(whole)?;
            fractional.checked_add(
                fractional
                    .checked_sub(1)?
                    .checked_div(self.denominator.get().checked_sub(1)?)?,
            )?
        };
        if units == 0 || units > maximum {
            return None;
        }
        let amount = exponent.checked_mul(i64::try_from(units).ok()?)?;
        let base = if units == self.denominator.get() {
            self.base.clone()
        } else {
            self.base
                .pow(Number::from_rational(RationalType::from_parts(
                    i64::try_from(units).ok()?,
                    self.denominator.get(),
                )))
        };
        Some((base, OperandMatch::Count { index, amount }))
    }

    /// Select a finite coefficient once, excluding its generated quotient.
    fn scalar_candidate<'used>(
        &self,
        index: usize,
        demand: OperandDemand,
        literal: Option<&Expr>,
        used: impl Iterator<Item = &'used OperandMatch>,
    ) -> Option<(Expr, OperandMatch)> {
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
                value
                    .literal()
                    .is_some_and(|number| number.value().is_finite() && !number.value().is_zero())
            })
            .unwrap_or(&self.base);
        Some((
            projected.clone(),
            OperandMatch::Coefficient {
                index,
                divisor: projected.clone(),
            },
        ))
    }
}
