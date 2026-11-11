//! Common numeric and symbolic factors of original additive terms.

#![expect(
    unsafe_code,
    reason = "a linear match selected from these operands names only their own terms, so consuming it needs no bounds checks"
)]

use core::ops::Sub;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::collections::{FixedMap, FixedState};

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactSources, Expr, Node, NumericValue,
    PartialSelection, RemainingOperand, Selection,
};

pub struct LinearOperands {
    terms: Vec<LinearTerm>,
    by_base: Option<FixedMap<Option<Expr>, Vec<usize>>>,
}

pub struct LinearTerm {
    original: Expr,
    base: Option<Expr>,
    coefficient: Expr,
    changed: bool,
}

/// One multiple of a linear source among these operands: the selected term
/// indices with the source coefficients they lose, the common numeric scale,
/// and the common symbolic factor, if any.
struct LinearMatch {
    terms: Vec<(usize, Expr)>,
    scale: Number,
    factor: Option<AlgebraicOperands>,
}

impl ConcreteMatcher<'_> {
    /// Replace common factors while retaining original unmatched bases.
    pub fn select_linear(
        &self,
        projected: Option<LinearOperands>,
        explicit: Option<PartialSelection>,
    ) -> Option<Selection> {
        let Some(mut operands) = projected else {
            return explicit.map(Selection::Partial);
        };
        let mut replacements = Vec::new();
        let mut apply = |source: &Expr, destination: &Expr| {
            let Some(required) = LinearOperands::new(source) else {
                return;
            };
            while let Some((replacement, exhausted_term)) = operands.take(&required, destination) {
                replacements.push(replacement);
                if !exhausted_term {
                    // Approximate division may leave every coefficient nonzero.
                    // Retain that residual without repeatedly rounding it away.
                    break;
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
        if replacements.is_empty() {
            return explicit.map(Selection::Partial);
        }
        let remainder = operands
            .terms
            .into_iter()
            .filter_map(|term| {
                if !term.changed {
                    return Some(RemainingOperand::Original(term.original));
                }
                if term.coefficient.is_zero_num() {
                    return None;
                }
                Some(match term.base {
                    Some(base) => RemainingOperand::Scaled {
                        base,
                        coefficient: term.coefficient,
                    },
                    None => RemainingOperand::Fixed(term.coefficient),
                })
            })
            .chain(replacements)
            .collect();
        Some(Selection::Partial(PartialSelection {
            sum: true,
            remainder,
            inserted: explicit.map_or_else(Vec::new, |partial| partial.inserted),
        }))
    }
}

impl LinearOperands {
    /// Project a sum, or a finite multiple of one, into its additive terms.
    pub fn new(expression: &Expr) -> Option<Self> {
        let (base, scale) = match expression.node() {
            Node::Product(factors) => {
                let [coefficient, base] = factors.as_slice() else {
                    return None;
                };
                if !coefficient
                    .literal()
                    .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
                {
                    return None;
                }
                (base, Some(coefficient.clone()))
            }
            Node::Div(base, divisor) => {
                let denominator = divisor.literal()?;
                if !denominator.value().is_finite() || denominator.value().is_zero() {
                    return None;
                }
                let scale =
                    Node::scalar_quotient(&NumericValue::new(Number::one()), denominator, 1);
                (base, Some(scale))
            }
            Node::Sum(_) | Node::Poly(_) => (expression, None),
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Pow(..)
            | Node::Derivative { .. } => return None,
        };
        let terms = base.additive_operands()?;
        Some(Self::from_terms(&terms, scale.as_ref()))
    }

    /// Project an already normalized additive operand list, scaled by a common
    /// finite coefficient when the source carried one.
    pub fn from_terms(terms: &[Expr], scale: Option<&Expr>) -> Self {
        let unit = Expr::number(1);
        let built: Vec<LinearTerm> = terms
            .iter()
            .map(|term| {
                scale.map_or_else(
                    || LinearTerm::new(term, &unit),
                    |coefficient| {
                        LinearTerm::new(
                            &Expr::product(vec![coefficient.clone(), term.clone()]),
                            &unit,
                        )
                    },
                )
            })
            .collect();
        let by_base = (built.len() > 8).then(|| {
            let mut index_map =
                FixedMap::with_capacity_and_hasher(built.len(), FixedState::default());
            for (index, term) in built.iter().enumerate() {
                index_map
                    .entry(term.base.clone())
                    .or_insert_with(Vec::new)
                    .push(index);
            }
            index_map
        });
        Self {
            terms: built,
            by_base,
        }
    }

    fn select(&self, source: &Self) -> Option<LinearMatch> {
        if self.terms.len() < source.terms.len() {
            return None;
        }
        let explicit: Option<Vec<_>> = source
            .terms
            .iter()
            .map(|required| {
                self.terms
                    .iter()
                    .position(|term| !term.changed && term.original == required.original)
                    .map(|index| (index, required.coefficient.clone()))
            })
            .collect();
        if let Some(terms) = explicit {
            return Some(LinearMatch {
                terms,
                scale: Number::one(),
                factor: None,
            });
        }
        if let Some(selected) = self.select_scaled(source, None) {
            return Some(selected);
        }
        let anchor = source.terms.first()?;
        for term in self
            .terms
            .iter()
            .filter(|term| !term.coefficient.is_zero_num())
        {
            let Some(base) = &term.base else {
                continue;
            };
            let Some(factor) = anchor.base.as_ref().map_or_else(
                || AlgebraicOperands::new(base, false, 0),
                |required| AlgebraicOperands::remove_once(base, required),
            ) else {
                continue;
            };
            if factor.slots.iter().any(|slot| {
                slot.base
                    .literal()
                    .is_some_and(|value| !value.value().is_finite())
            }) {
                continue;
            }
            let multiplier = PartialSelection {
                sum: false,
                remainder: factor.remainder(),
                inserted: Vec::new(),
            }
            .rebuild();
            if !multiplier.is_exact_one()
                && let Some(mut selected) = self.select_scaled(source, Some(&multiplier))
            {
                selected.factor = Some(factor);
                return Some(selected);
            }
        }
        None
    }

    /// Select a common factor using $c F (a b + d e)$.
    /// Numeric ratios choose a common signed scale without symbolic division.
    fn select_scaled(&self, source: &Self, factor: Option<&Expr>) -> Option<LinearMatch> {
        let mut scale: Option<Number> = None;
        let mut selected: Vec<(usize, Expr)> = Vec::with_capacity(source.terms.len());
        for required in &source.terms {
            let coefficient = required.coefficient.literal()?;
            if !coefficient.value().is_finite() || coefficient.value().is_zero() {
                return None;
            }
            let base = factor.map_or_else(
                || required.base.clone(),
                |multiplier| {
                    Some(required.base.as_ref().map_or_else(
                        || multiplier.clone(),
                        |base| Expr::product(vec![multiplier.clone(), base.clone()]),
                    ))
                },
            );
            let available = |index: usize, term: &LinearTerm| {
                !term.coefficient.is_zero_num()
                    && !selected.iter().any(|(previous, _)| *previous == index)
            };
            let (index, term) = if let Some(index_map) = &self.by_base {
                let candidates = index_map.get(&base)?;
                candidates.iter().find_map(|&idx| {
                    self.terms
                        .get(idx)
                        .filter(|term| available(idx, term))
                        .map(|term| (idx, term))
                })?
            } else {
                self.terms
                    .iter()
                    .enumerate()
                    .find(|(idx, term)| term.base == base && available(*idx, term))?
            };
            let actual = term.coefficient.literal()?;
            if !actual.value().is_finite() {
                return None;
            }
            let quotient = Node::scalar_quotient(actual, coefficient, 1);
            let ratio = quotient.literal()?.value();
            if !ratio.is_finite() || ratio.is_zero() {
                return None;
            }
            if let Some(current) = &mut scale {
                if current.is_positive() && ratio.is_positive() {
                    if ratio < current {
                        *current = ratio.clone();
                    }
                } else if current.is_negative() && ratio.is_negative() {
                    if ratio > current {
                        *current = ratio.clone();
                    }
                } else if ratio != current {
                    return None;
                }
            } else {
                scale = Some(ratio.clone());
            }
            selected.push((index, required.coefficient.clone()));
        }
        Some(LinearMatch {
            terms: selected,
            scale: scale?,
            factor: None,
        })
    }

    /// Select one multiple of `source` and subtract it from the matched terms.
    /// Also reports whether a matched term was exhausted.
    fn take(&mut self, source: &Self, destination: &Expr) -> Option<(RemainingOperand, bool)> {
        let selected = self.select(source)?;
        let coefficient = Expr::number(selected.scale);
        let mut exhausted_term = false;
        for (index, required) in selected.terms {
            // SAFETY: `select` names positions of `self.terms`, which it borrowed
            // unchanged just above.
            let term = unsafe { self.terms.get_unchecked_mut(index) };
            let consumed = Expr::product(vec![required, coefficient.clone()]);
            term.coefficient = Sub::sub(&term.coefficient, consumed);
            term.changed = true;
            exhausted_term |= term.coefficient.is_zero_num();
        }
        let inserted = if coefficient.is_exact_one() {
            destination.clone()
        } else if let Some(terms) = destination.additive_operands() {
            Expr::sum(
                terms
                    .iter()
                    .map(|term| Expr::product(vec![coefficient.clone(), term.clone()]))
                    .collect(),
            )
        } else {
            Expr::product(vec![coefficient, destination.clone()])
        };
        let replacement = match selected.factor {
            Some(factor) => RemainingOperand::Factored(PartialSelection {
                sum: false,
                remainder: factor.remainder(),
                inserted: vec![inserted],
            }),
            None => RemainingOperand::Fixed(inserted),
        };
        Some((replacement, exhausted_term))
    }

    /// Recognize additive shapes and numeric multiples without reconstructing terms.
    pub fn supports(expression: &Expr) -> bool {
        match expression.node() {
            Node::Sum(_) | Node::Poly(_) => true,
            Node::Product(factors) => {
                matches!(factors.as_slice(),[coefficient,base] if coefficient.literal().is_some_and(|value|value.value().is_finite() && !value.value().is_zero()) && matches!(base.node(),Node::Sum(_) | Node::Poly(_)))
            }
            Node::Div(base, coefficient) => {
                matches!(base.node(), Node::Sum(_) | Node::Poly(_))
                    && coefficient
                        .literal()
                        .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
            }
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Pow(..)
            | Node::Derivative { .. } => false,
        }
    }
}

impl LinearTerm {
    fn new(original: &Expr, unit: &Expr) -> Self {
        let (numerator, denominator) = if let Node::Div(numerator, denominator) = original.node()
            && denominator
                .literal()
                .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
        {
            (numerator, Some(denominator))
        } else {
            (original, None)
        };
        let (mut coefficient, base) = match numerator.node() {
            Node::Number(value) if value.value().is_finite() => (numerator.clone(), None),
            Node::ImaginaryUnit => (numerator.clone(), None),
            Node::Product(factors)
                if factors.first().is_some_and(|first| {
                    first
                        .literal()
                        .is_some_and(|value| value.value().is_finite())
                }) =>
            {
                if let Some((first, rest)) = factors.split_first() {
                    (first.clone(), Some(Expr::product(rest.to_vec())))
                } else {
                    (unit.clone(), Some(numerator.clone()))
                }
            }
            Node::Number(_)
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Sum(_)
            | Node::Poly(_)
            | Node::Product(_)
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => (unit.clone(), Some(numerator.clone())),
        };
        if let Some(divisor) = denominator
            && let (Some(left), Some(right)) = (coefficient.literal(), divisor.literal())
        {
            coefficient = Node::scalar_quotient(left, right, 1);
        }
        Self {
            original: original.clone(),
            coefficient,
            base,
            changed: false,
        }
    }
}
