//! Bounds on the principal phases of joined arguments and their signed sum.

use core::{
    cmp::Ordering,
    ops::{Add, Neg},
};

use alloc::vec::Vec;

use num_anafis::Number;

use crate::core::{Expr, ExprView, Property, Truth};

/// Bounds on a phase or a signed sum of phases, in units of $\pi$.
#[derive(Clone, Copy)]
pub struct Phase {
    lower: i64,
    lower_excluded: bool,
    upper: i64,
    upper_excluded: bool,
}

/// The state change one join made, so that a rejected candidate is undone
/// without copying the accumulator.
pub enum Undo {
    /// The count of the core at this position was replaced.
    Count { index: usize, previous: i64 },
    /// A core was appended.
    Pushed,
    /// The literal bounds and the exact product were replaced.
    Literal {
        literal: Phase,
        product: Option<Number>,
    },
}

/// The signed phase sum of the joined arguments.
pub struct PhaseSum {
    /// Each core of the arguments that are not literals, with its net signed
    /// count and the bounds on its phase. A positive factor does not change a
    /// phase, so arguments that differ only by positive factors share a core,
    /// and their phases cancel exactly when the count returns to zero.
    cores: Vec<(Expr, i64, Phase)>,
    /// Bounds of the literal arguments. While `product` is known they fix the
    /// sum to a point or an open unit interval.
    literal: Phase,
    /// The exact product of the literal arguments raised to their signs.
    product: Option<Number>,
}

impl Add for Phase {
    type Output = Self;

    /// Fewer than $2^{61}$ arguments are joined, so sums of unit bounds
    /// never saturate.
    fn add(self, other: Self) -> Self {
        Self {
            lower: self.lower.saturating_add(other.lower),
            lower_excluded: self.lower_excluded || other.lower_excluded,
            upper: self.upper.saturating_add(other.upper),
            upper_excluded: self.upper_excluded || other.upper_excluded,
        }
    }
}

impl Neg for Phase {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            lower: self.upper.saturating_neg(),
            lower_excluded: self.upper_excluded,
            upper: self.lower.saturating_neg(),
            upper_excluded: self.lower_excluded,
        }
    }
}

impl Phase {
    /// The principal phase of a literal from the signs of its parts: a point
    /// on the real axis or an open half-plane.
    fn literal(value: &Number) -> Self {
        let (real, imaginary) = value.components();
        let (lower, lower_excluded, upper, upper_excluded) = if imaginary.is_positive() {
            (0, true, 1, true)
        } else if imaginary.is_negative() {
            (-1, true, 0, true)
        } else if real.is_negative() {
            (1, false, 1, false)
        } else {
            (0, false, 0, false)
        };
        Self {
            lower,
            lower_excluded,
            upper,
            upper_excluded,
        }
    }
}

impl PhaseSum {
    const EMPTY: Phase = Phase {
        lower: 0,
        lower_excluded: false,
        upper: 0,
        upper_excluded: false,
    };

    pub fn new() -> Self {
        Self {
            cores: Vec::new(),
            literal: Self::EMPTY,
            product: Some(Number::from(1)),
        }
    }

    /// Join `argument`, subtracted when `negative`, and return the change so
    /// that a rejected candidate is undone by [`Self::undo`].
    ///
    /// The phase of a core lies in $[0,0]$ when nonnegative, $[1,1]$ when
    /// negative, $[0,1]$ when real, and $(-1,1]$ otherwise; a core counted
    /// $n$ times contributes $n$ times its one phase. An exact literal
    /// also updates the exact product, whose principal phase differs from the
    /// literal sum by a multiple of $2\pi$; the one shift that meets the
    /// summed bounds fixes the literal sum.
    pub fn join(&mut self, argument: &Expr, negative: bool) -> Undo {
        let value = argument
            .as_number()
            .cloned()
            .or_else(|| matches!(argument.view(), ExprView::ImaginaryUnit).then(Number::i));
        let Some(number) = value else {
            let core = if let ExprView::Product(factors) = argument.view() {
                Expr::product(
                    factors
                        .iter()
                        .filter(|factor| factor.properties().get(Property::Positive) != Truth::True)
                        .cloned()
                        .collect(),
                )
            } else {
                argument.clone()
            };
            let step = if negative { -1 } else { 1 };
            if let Some((index, (_, count, _))) = self
                .cores
                .iter_mut()
                .enumerate()
                .find(|(_, (other, _, _))| *other == core)
            {
                let previous = *count;
                *count = count.saturating_add(step);
                return Undo::Count { index, previous };
            }
            let facts = core.properties();
            let (lower, lower_excluded, upper, upper_excluded) =
                if facts.get(Property::Nonnegative) == Truth::True {
                    (0, false, 0, false)
                } else if facts.get(Property::Negative) == Truth::True {
                    (1, false, 1, false)
                } else if facts.get(Property::Real) == Truth::True {
                    (0, false, 1, false)
                } else {
                    (-1, true, 1, false)
                };
            let phase = Phase {
                lower,
                lower_excluded,
                upper,
                upper_excluded,
            };
            self.cores.push((core, step, phase));
            return Undo::Pushed;
        };
        let phase = Phase::literal(&number);
        let loose = Add::add(self.literal, if negative { Neg::neg(phase) } else { phase });
        let exact = self
            .product
            .as_ref()
            .and_then(|product| {
                if negative {
                    product.checked_div(&number)
                } else {
                    product.checked_mul(&number)
                }
            })
            .filter(Number::is_exact);
        let refined = exact.as_ref().and_then(|product| {
            let principal = Phase::literal(product);
            // The exact sum has width at most two units, so one of two
            // consecutive shifts meets it.
            let turns = loose.lower.saturating_sub(principal.lower).div_euclid(2);
            [turns, turns.saturating_add(1)]
                .into_iter()
                .map(|shift| {
                    let offset = shift.saturating_mul(2);
                    Phase {
                        lower: principal.lower.saturating_add(offset),
                        upper: principal.upper.saturating_add(offset),
                        ..principal
                    }
                })
                .find(|shifted| {
                    let (lower, lower_excluded) = match shifted.lower.cmp(&loose.lower) {
                        Ordering::Greater => (shifted.lower, shifted.lower_excluded),
                        Ordering::Less => (loose.lower, loose.lower_excluded),
                        Ordering::Equal => {
                            (loose.lower, shifted.lower_excluded || loose.lower_excluded)
                        }
                    };
                    let (upper, upper_excluded) = match shifted.upper.cmp(&loose.upper) {
                        Ordering::Less => (shifted.upper, shifted.upper_excluded),
                        Ordering::Greater => (loose.upper, loose.upper_excluded),
                        Ordering::Equal => {
                            (loose.upper, shifted.upper_excluded || loose.upper_excluded)
                        }
                    };
                    lower < upper || (lower == upper && !lower_excluded && !upper_excluded)
                })
        });
        let undo = Undo::Literal {
            literal: self.literal,
            product: self.product.take(),
        };
        self.literal = refined.unwrap_or(loose);
        self.product = exact.filter(|_| refined.is_some());
        undo
    }

    /// Restore the state before the join that returned `undo`.
    pub fn undo(&mut self, undo: Undo) {
        match undo {
            Undo::Count { index, previous } => {
                if let Some((_, count, _)) = self.cores.get_mut(index) {
                    *count = previous;
                }
            }
            Undo::Pushed => {
                self.cores.pop();
            }
            Undo::Literal { literal, product } => {
                self.literal = literal;
                self.product = product;
            }
        }
    }

    /// Bounds on the whole sum: each core's phase scaled by its count, plus
    /// the literal bounds.
    fn total(&self) -> Phase {
        self.cores
            .iter()
            .fold(self.literal, |sum, (_, count, phase)| {
                let magnitude = count.saturating_abs();
                let scaled = if magnitude == 0 {
                    Self::EMPTY
                } else {
                    Phase {
                        lower: phase.lower.saturating_mul(magnitude),
                        upper: phase.upper.saturating_mul(magnitude),
                        ..*phase
                    }
                };
                Add::add(sum, if *count < 0 { Neg::neg(scaled) } else { scaled })
            })
    }

    /// The winding $w$ when the bounds place the sum in one interval
    /// $(2w-1,2w+1]$.
    pub fn winding(&self) -> Option<i64> {
        let total = self.total();
        let turns = total.upper.div_euclid(2);
        let floor = turns.saturating_mul(2).saturating_sub(1);
        (total.lower > floor || (total.lower == floor && total.lower_excluded)).then_some(turns)
    }

    /// Whether the bounds admit $w=0$ and a core with a nonzero count
    /// remains, so that the caller can supply a hypothesis on its phase.
    pub fn assumable(&self) -> bool {
        let total = self.total();
        self.cores.iter().any(|(_, count, _)| *count != 0)
            && total.upper > -1
            && (total.lower < 1 || (total.lower == 1 && !total.lower_excluded))
    }
}
