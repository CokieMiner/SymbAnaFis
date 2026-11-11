//! Products of principal square roots and sums of principal logarithms.
//!
//! For arguments \(a\) with principal phases \(\alpha\in(-\pi,\pi]\) and signs
//! \(s=\pm1\) (DLMF §4.2), \(\prod\sqrt{a}^{\,s}=(-1)^w\sqrt{\prod a^{s}}\) and
//! \(\sum s\ln a=\ln\prod a^{s}+2\pi i w\), where \(w\) is the integer with
//! \(\sum s\alpha\in((2w-1)\pi,(2w+1)\pi]\). Bounds on each phase decide
//! \(w\); assumption-dependent mode assumes \(w=0\) when the bounds allow it
//! and the phase of a non-literal argument remains in the sum.

use core::{
    cmp::Ordering,
    ops::{Add, Div, Neg},
    slice::from_ref,
};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::core::{BuiltinFunction, Expr, ExprView, FunctionDefinition, Property, Truth};

use super::{Domain, DomainSafety};

/// Namespace for principal square-root product and logarithm sum branch rules.
pub struct Branches;

/// Bounds on a phase or a signed sum of phases, in units of \(\pi\).
#[derive(Clone, Copy)]
struct Phase {
    lower: i64,
    lower_excluded: bool,
    upper: i64,
    upper_excluded: bool,
}

/// The signed phase sum of the joined arguments.
#[derive(Clone)]
struct PhaseSum {
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

impl Branches {
    /// \(\prod\sqrt{a}/\prod\sqrt{b}=(-1)^w\sqrt{\prod a/\prod b}\) over the
    /// `sqrt` factors stored in a product or in the numerator and denominator
    /// of a quotient. A bare or integral power is never read as a product of
    /// roots.
    ///
    /// Roots join in factor order while the bounds fix \(w\), or, in
    /// assumption-dependent mode, admit \(w=0\). A numerator radicand must be
    /// finite and a denominator radicand nonzero.
    pub fn combine_roots(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        let (numerator, denominator) = match expression.view() {
            ExprView::Product(factors) => (factors, &[][..]),
            ExprView::Div(top, bottom) => (Self::operands(top), Self::operands(bottom)),
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Sum(_)
            | ExprView::Pow(..)
            | ExprView::Derivative { .. } => return None,
        };
        numerator
            .iter()
            .chain(denominator)
            .filter_map(Self::radicand)
            .nth(1)?;
        let assumed = safety == DomainSafety::AssumptionDependent;
        let mut sum = PhaseSum::new();
        let (mut top, mut bottom) = (Vec::new(), Vec::new());
        let (mut top_roots, mut bottom_roots) = (Vec::new(), Vec::new());
        for (factors, negative) in [(numerator, false), (denominator, true)] {
            for factor in factors {
                let kept = if negative { &mut bottom } else { &mut top };
                let Some(inner) = Self::radicand(factor) else {
                    kept.push(factor.clone());
                    continue;
                };
                let facts = inner.properties();
                let domain = if negative {
                    Property::Nonzero
                } else {
                    Property::Finite
                };
                let next = sum.join(inner, negative);
                if Domain::admits(facts.get(domain), safety)
                    && (next.winding().is_some() || assumed && next.assumable())
                {
                    sum = next;
                    if negative {
                        bottom_roots.push(inner.clone());
                    } else {
                        top_roots.push(inner.clone());
                    }
                } else {
                    kept.push(factor.clone());
                }
            }
        }
        if top_roots.len().saturating_add(bottom_roots.len()) < 2 {
            return None;
        }
        if sum.winding().is_some_and(|turns| turns.rem_euclid(2) == 1) {
            top.push(Expr::number(-1));
        }
        top.push(Div::div(Expr::product(top_roots), Expr::product(bottom_roots)).sqrt());
        Some(Div::div(Expr::product(top), Expr::product(bottom)))
    }

    /// \(c\sum s\,L(a)=c\,L(\prod a^{s})+2\pi i\,c\,w/\ln B\) for
    /// the `ln`, `log2`, and `log10` terms of a sum, grouped by function and
    /// by the magnitude \(c\) of their exact real coefficient; \(B\) is the
    /// base.
    ///
    /// Terms join in order while the bounds fix \(w\), or, in
    /// assumption-dependent mode, admit \(w=0\). An added argument must be
    /// finite and a subtracted argument nonzero.
    pub fn combine_logarithms(sum: &Expr, safety: DomainSafety) -> Option<Expr> {
        let ExprView::Sum(scan) = sum.view() else {
            return None;
        };
        scan.filter(|term| Self::logarithm_term(term).is_some())
            .nth(1)?;
        let ExprView::Sum(children) = sum.view() else {
            return None;
        };
        let terms: Vec<&Expr> = children.collect();
        let classified: Vec<_> = terms.iter().copied().map(Self::logarithm_term).collect();
        let assumed = safety == DomainSafety::AssumptionDependent;
        let mut joined = vec![false; terms.len()];
        let mut combined = Vec::new();
        // Each group of equal function and coefficient magnitude is joined
        // once, from its first term.
        for (first, entry) in classified.iter().enumerate() {
            let Some((builtin, magnitude, _, _)) = entry else {
                continue;
            };
            if classified
                .iter()
                .take(first)
                .flatten()
                .any(|(other, size, _, _)| other == builtin && size == magnitude)
            {
                continue;
            }
            let mut phases = PhaseSum::new();
            let (mut top, mut bottom, mut members) = (Vec::new(), Vec::new(), Vec::new());
            for (position, candidate) in classified.iter().enumerate().skip(first) {
                let Some((other, size, negative, argument)) = candidate else {
                    continue;
                };
                if other != builtin || size != magnitude {
                    continue;
                }
                let facts = argument.properties();
                let domain = if *negative {
                    Property::Nonzero
                } else {
                    Property::Finite
                };
                let next = phases.join(argument, *negative);
                if Domain::admits(facts.get(domain), safety)
                    && (next.winding().is_some() || assumed && next.assumable())
                {
                    phases = next;
                    members.push(position);
                    if *negative {
                        bottom.push((*argument).clone());
                    } else {
                        top.push((*argument).clone());
                    }
                }
            }
            if members.len() < 2 {
                continue;
            }
            for position in members {
                if let Some(flag) = joined.get_mut(position) {
                    *flag = true;
                }
            }
            let scale = Expr::number(magnitude.clone());
            let argument = Div::div(Expr::product(top), Expr::product(bottom));
            // Grouping established the unary builtin logarithm.
            let logarithm = FunctionDefinition::construct(builtin.function(), vec![argument]);
            combined.push(Expr::product(vec![scale.clone(), logarithm]));
            let turns = phases.winding().unwrap_or(0);
            if turns != 0 {
                let turn = Expr::product(vec![
                    Expr::number(turns.saturating_mul(2)),
                    scale,
                    Expr::pi(),
                    Expr::i(),
                ]);
                combined.push(if *builtin == BuiltinFunction::Log2 {
                    Div::div(turn, Expr::number(2).ln())
                } else if *builtin == BuiltinFunction::Log10 {
                    Div::div(turn, Expr::number(10).ln())
                } else {
                    turn
                });
            }
        }
        if combined.is_empty() {
            return None;
        }
        combined.extend(
            terms
                .into_iter()
                .zip(joined)
                .filter(|(_, removed)| !removed)
                .map(|(term, _)| term.clone()),
        );
        Some(Expr::sum(combined))
    }

    /// A unary `ln`, `log2`, or `log10` term with its function, the magnitude
    /// of its exact nonzero real coefficient, whether that coefficient is
    /// negative, and its argument.
    fn logarithm_term(term: &Expr) -> Option<(BuiltinFunction, Number, bool, &Expr)> {
        let (coefficient, call) = if let ExprView::Product([scale, call]) = term.view() {
            (scale.as_number()?.clone(), call)
        } else {
            (Number::from(1), term)
        };
        let (real, imaginary) = coefficient.components();
        if !coefficient.is_exact() || !imaginary.is_zero() || real.is_zero() {
            return None;
        }
        let ExprView::Function {
            name,
            args: [argument],
        } = call.view()
        else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
        matches!(
            builtin,
            BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10
        )
        .then(|| (builtin, coefficient.abs(), real.is_negative(), argument))
    }

    fn operands(operand: &Expr) -> &[Expr] {
        if let ExprView::Product(factors) = operand.view() {
            factors
        } else {
            from_ref(operand)
        }
    }

    fn radicand(factor: &Expr) -> Option<&Expr> {
        if let ExprView::Function { name, args } = factor.view()
            && *name == BuiltinFunction::Sqrt.function()
            && let [inner] = args
        {
            Some(inner)
        } else {
            None
        }
    }
}

impl Add for Phase {
    type Output = Self;

    /// Fewer than \(2^{61}\) arguments are joined, so sums of unit bounds
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

    fn new() -> Self {
        Self {
            cores: Vec::new(),
            literal: Self::EMPTY,
            product: Some(Number::from(1)),
        }
    }

    /// The sum after joining `argument`, subtracted when `negative`.
    ///
    /// The phase of a core lies in \(\[0,0\]\) when nonnegative, \(\[1,1\]\) when
    /// negative, \(\[0,1\]\) when real, and \((-1,1\]\) otherwise; a core counted
    /// \(n\) times contributes \(n\) times its one phase. An exact literal
    /// also updates the exact product, whose principal phase differs from the
    /// literal sum by a multiple of \(2\pi\); the one shift that meets the
    /// summed bounds fixes the literal sum.
    fn join(&self, argument: &Expr, negative: bool) -> Self {
        let mut next = self.clone();
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
            if let Some((_, count, _)) = next.cores.iter_mut().find(|(other, _, _)| *other == core)
            {
                *count = count.saturating_add(step);
                return next;
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
            next.cores.push((core, step, phase));
            return next;
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
        next.literal = refined.unwrap_or(loose);
        next.product = exact.filter(|_| refined.is_some());
        next
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

    /// The winding \(w\) when the bounds place the sum in one interval
    /// \((2w-1,2w+1]\).
    fn winding(&self) -> Option<i64> {
        let total = self.total();
        let turns = total.upper.div_euclid(2);
        let floor = turns.saturating_mul(2).saturating_sub(1);
        (total.lower > floor || (total.lower == floor && total.lower_excluded)).then_some(turns)
    }

    /// Whether the bounds admit \(w=0\) and a core with a nonzero count
    /// remains, so that the caller can supply a hypothesis on its phase.
    fn assumable(&self) -> bool {
        let total = self.total();
        self.cores.iter().any(|(_, count, _)| *count != 0)
            && total.upper > -1
            && (total.lower < 1 || (total.lower == 1 && !total.lower_excluded))
    }
}
