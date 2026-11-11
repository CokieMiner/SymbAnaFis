//! Products of principal square roots and sums of principal logarithms.
//!
//! For arguments $a$ with principal phases $\alpha\in(-\pi,\pi]$ and signs
//! $s=\pm1$ (DLMF §4.2), $\prod\sqrt{a}^{ s}=(-1)^w\sqrt{\prod a^{s}}$ and
//! $\sum s\ln a=\ln\prod a^{s}+2\pi i w$, where $w$ is the integer with
//! $\sum s\alpha\in((2w-1)\pi,(2w+1)\pi]$. Bounds on each phase decide
//! $w$; assumption-dependent mode assumes $w=0$ when the bounds allow it
//! and the phase of a non-literal argument remains in the sum.

use core::{ops::Div, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{BuiltinFunction, Expr, ExprView, FunctionDefinition, Property, StoredForm},
};

use super::{Domain, DomainSafety, PhaseSum};

/// Namespace for principal square-root product and logarithm sum branch rules.
pub struct Branches;

impl Branches {
    /// $\prod\sqrt{a}/\prod\sqrt{b}=(-1)^w\sqrt{\prod a/\prod b}$ over the
    /// `sqrt` factors stored in a product or in the numerator and denominator
    /// of a quotient. A bare or integral power is never read as a product of
    /// roots.
    ///
    /// Roots join in factor order while the bounds fix $w$, or, in
    /// assumption-dependent mode, admit $w=0$. A numerator radicand must be
    /// finite and a denominator radicand nonzero.
    pub fn combine_roots(expression: &Expr, safety: DomainSafety) -> Option<Expr> {
        let (numerator, denominator) = match expression.view() {
            ExprView::Product(factors) => (factors, &[][..]),
            ExprView::Div(top, bottom) => {
                let top_factors = if let ExprView::Product(factors) = top.view() {
                    factors
                } else {
                    from_ref(top)
                };
                let bottom_factors = if let ExprView::Product(factors) = bottom.view() {
                    factors
                } else {
                    from_ref(bottom)
                };
                (top_factors, bottom_factors)
            }
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
        // The empty sum has winding zero.
        let mut turns = Some(0);
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
                if !Domain::admits(facts.get(domain), safety) {
                    kept.push(factor.clone());
                    continue;
                }
                let undo = sum.join(inner, negative);
                let winding = sum.winding();
                if winding.is_some() || assumed && sum.assumable() {
                    turns = winding;
                    if negative {
                        bottom_roots.push(inner.clone());
                    } else {
                        top_roots.push(inner.clone());
                    }
                } else {
                    sum.undo(undo);
                    kept.push(factor.clone());
                }
            }
        }
        if top_roots.len().saturating_add(bottom_roots.len()) < 2 {
            return None;
        }
        if turns.is_some_and(|winding| winding.rem_euclid(2) == 1) {
            top.push(Expr::number(-1));
        }
        top.push(Div::div(Expr::product(top_roots), Expr::product(bottom_roots)).sqrt());
        Some(Div::div(Expr::product(top), Expr::product(bottom)))
    }

    /// $c\sum s L(a)=c L(\prod a^{s})+2\pi i c w/\ln B$ for
    /// the `ln`, `log2`, and `log10` terms of a sum, grouped by function and
    /// by the magnitude $c$ of their exact real coefficient; $B$ is the
    /// base.
    ///
    /// Terms join in order while the bounds fix $w$, or, in
    /// assumption-dependent mode, admit $w=0$. An added argument must be
    /// finite and a subtracted argument nonzero.
    pub fn combine_logarithms(sum: &Expr, safety: DomainSafety) -> Option<Expr> {
        if sum.simple_bases() == Some(true) {
            return None;
        }
        let ExprView::Sum(children) = sum.view() else {
            return None;
        };
        children
            .clone()
            .filter(|term| Self::logarithm_term(term).is_some())
            .nth(1)?;
        let classified: Vec<_> = children.clone().map(Self::logarithm_term).collect();
        // One pass groups the terms by function and coefficient magnitude, in
        // the order of each group's first term. The interned magnitude is the
        // key because equal exact values share one node while `Number` itself
        // is not `Eq`.
        let mut slots: FixedMap<(BuiltinFunction, Expr), usize> = FixedMap::default();
        let mut groups: Vec<((BuiltinFunction, Expr), Vec<usize>)> = Vec::new();
        for (position, entry) in classified.iter().enumerate() {
            let Some((builtin, magnitude, _, _)) = entry else {
                continue;
            };
            let key = (*builtin, Expr::number(magnitude.clone()));
            let fresh = groups.len();
            let slot = *slots.entry(key.clone()).or_insert(fresh);
            if slot == fresh {
                groups.push((key, Vec::new()));
            }
            if let Some((_, members)) = groups.get_mut(slot) {
                members.push(position);
            }
        }
        let assumed = safety == DomainSafety::AssumptionDependent;
        let mut joined = vec![false; children.len()];
        let mut combined = Vec::new();
        for ((builtin, magnitude), members) in groups {
            let mut phases = PhaseSum::new();
            let (mut top, mut bottom, mut accepted) = (Vec::new(), Vec::new(), Vec::new());
            for position in members {
                let Some((_, _, negative, argument)) =
                    classified.get(position).and_then(Option::as_ref)
                else {
                    continue;
                };
                let facts = argument.properties();
                let domain = if *negative {
                    Property::Nonzero
                } else {
                    Property::Finite
                };
                if !Domain::admits(facts.get(domain), safety) {
                    continue;
                }
                let undo = phases.join(argument, *negative);
                if phases.winding().is_some() || assumed && phases.assumable() {
                    accepted.push(position);
                    if *negative {
                        bottom.push((*argument).clone());
                    } else {
                        top.push((*argument).clone());
                    }
                } else {
                    phases.undo(undo);
                }
            }
            if accepted.len() < 2 {
                continue;
            }
            for position in accepted {
                if let Some(flag) = joined.get_mut(position) {
                    *flag = true;
                }
            }
            let scale = magnitude;
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
                combined.push(if builtin == BuiltinFunction::Log2 {
                    Div::div(turn, Expr::number(2).ln())
                } else if builtin == BuiltinFunction::Log10 {
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
            children
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
        let (scale, call) = if let ExprView::Product([scale, call]) = term.view() {
            (Some(scale), call)
        } else {
            (None, term)
        };
        let ExprView::Function {
            name,
            args: [argument],
        } = call.view()
        else {
            return None;
        };
        let builtin = BuiltinFunction::of(name)?;
        if !matches!(
            builtin,
            BuiltinFunction::Ln | BuiltinFunction::Log2 | BuiltinFunction::Log10
        ) {
            return None;
        }
        let coefficient = match scale {
            Some(factor) => factor.as_number()?.clone(),
            None => Number::from(1),
        };
        let (real, imaginary) = coefficient.components();
        if !coefficient.is_exact() || !imaginary.is_zero() || real.is_zero() {
            return None;
        }
        Some((builtin, coefficient.abs(), real.is_negative(), argument))
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
