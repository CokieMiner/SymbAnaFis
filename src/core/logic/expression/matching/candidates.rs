//! Virtual operand candidates with disjoint counted consumption.

use alloc::vec;

use num_anafis::{Number, RationalType};

use super::{
    AlgebraicOperands, Expr, FactorSide, OperandDemand, OperandKind, OperandMatch, WeightedOperand,
};

impl AlgebraicOperands {
    /// Whether the value `candidate` builds at `index` for `demand` is a
    /// coefficient times a base, and therefore a product.
    ///
    /// Conservative: `false` whenever the value might not be a product, so a
    /// caller that skips a product candidate skips only products.
    #[inline]
    pub fn builds_product(&self, index: usize, demand: OperandDemand) -> bool {
        let Some(slot) = self.slots.get(index) else {
            return false;
        };
        if slot.coefficient || !self.sum || !slot.negative {
            return false;
        }
        // A negative slot reaches the product branch exactly when the count is
        // neither the whole original nor one unit of a positive base, and the
        // second is impossible here.
        let unit = i64::try_from(slot.denominator.get()).unwrap_or(0);
        match demand {
            OperandDemand::Single => slot.count != unit,
            OperandDemand::Copies(count) => count != slot.count,
            OperandDemand::Split(_)
            | OperandDemand::Whole
            | OperandDemand::Power(_)
            | OperandDemand::Root { .. }
            | OperandDemand::Rational { .. } => false,
        }
    }

    /// Construct one virtual operand within its remaining original count.
    #[inline]
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
            if selected.index == index
                && let OperandKind::Count { amount } = selected.kind
            {
                remaining = remaining.checked_sub(amount)?;
            }
        }
        let count = match demand {
            OperandDemand::Single => i64::try_from(slot.denominator.get()).ok()?,
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
            OperandMatch {
                index,
                kind: OperandKind::Count { amount: count },
            },
        ))
    }
}

impl WeightedOperand {
    /// Select a finite coefficient once, excluding its generated quotient.
    #[inline]
    fn scalar_candidate<'used>(
        &self,
        index: usize,
        demand: OperandDemand,
        literal: Option<&Expr>,
        used: impl Iterator<Item = &'used OperandMatch>,
    ) -> Option<(Expr, OperandMatch)> {
        if !matches!(demand, OperandDemand::Single | OperandDemand::Whole)
            || used.into_iter().any(|selected| selected.index == index)
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
        let divisor = projected.literal()?.clone();
        Some((
            projected.clone(),
            OperandMatch {
                index,
                kind: OperandKind::Coefficient { divisor },
            },
        ))
    }

    /// Try integral bases before fractional blocks already present in the input.
    /// For integer $n>0$, $(b^r)^n=b^{rn}$ uses the same principal logarithm.
    #[inline]
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
        Some((
            base,
            OperandMatch {
                index,
                kind: OperandKind::Count { amount },
            },
        ))
    }
}
