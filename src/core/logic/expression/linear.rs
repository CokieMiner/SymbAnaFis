//! Common numeric and symbolic factors of original additive terms.

use core::ops::Sub;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use super::{
    AdditiveOperands, AlgebraicOperands, ConcreteMatcher, ExactSources, Expr, LinearMatch,
    LinearOperands, LinearTerm, Node, NumericValue, PartialSelection, RemainingOperand, Selection,
};

impl ConcreteMatcher<'_> {
    /// Replace common factors while retaining original unmatched bases.
    pub fn select_linear(
        &self,
        expression: &Expr,
        explicit: Option<PartialSelection>,
    ) -> Option<Selection> {
        let Some(mut operands) = LinearOperands::new(expression) else {
            return explicit.map(Selection::Partial);
        };
        let mut replacements = Vec::new();
        let mut apply = |source: &Expr, destination: &Expr| {
            let Some(required) = LinearOperands::new(source) else {
                return;
            };
            while let Some(selected) = operands.select(&required) {
                let (replacement, exhausted_term) = operands.consume(selected, destination);
                replacements.push(replacement);
                if !exhausted_term {
                    // Approximate division may leave every coefficient nonzero.
                    // Retain that residual without repeatedly rounding it away.
                    break;
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
    fn new(expression: &Expr) -> Option<Self> {
        let unit = Expr::number(1);
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
                    Expr::scalar_quotient(&NumericValue::new(Number::one()), denominator, 1);
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
        Some(Self {
            terms: terms
                .iter()
                .map(|term| {
                    scale.as_ref().map_or_else(
                        || LinearTerm::new(term, &unit),
                        |coefficient| {
                            LinearTerm::new(
                                &Expr::product(vec![coefficient.clone(), term.clone()]),
                                &unit,
                            )
                        },
                    )
                })
                .collect(),
        })
    }

    fn select(&self, source: &Self) -> Option<LinearMatch> {
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
            .rebuild(&mut Clone::clone);
            if !multiplier.is_one_num()
                && let Some(mut selected) = self.select_scaled(source, Some(&multiplier))
            {
                selected.factor = Some(factor);
                return Some(selected);
            }
        }
        None
    }

    /// Select a common factor using \(c F (a b + d e)\).
    /// Numeric ratios choose a common signed scale without symbolic division.
    fn select_scaled(&self, source: &Self, factor: Option<&Expr>) -> Option<LinearMatch> {
        let mut scale: Option<Number> = None;
        let mut selected: Vec<(usize, Expr)> = Vec::new();
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
            let direct = self
                .terms
                .iter()
                .enumerate()
                .find(|(index, term)| term.base == base && available(*index, term));
            let (index, term) = direct.or_else(|| {
                let multiplier = factor?;
                let required_base = required.base.as_ref()?;
                self.terms.iter().enumerate().find(|(index, term)| {
                    if !available(*index, term) {
                        return false;
                    }
                    let Some(actual_base) = &term.base else {
                        return false;
                    };
                    let Some(remaining) =
                        AlgebraicOperands::remove_once(actual_base, required_base)
                    else {
                        return false;
                    };
                    PartialSelection {
                        sum: false,
                        remainder: remaining.remainder(),
                        inserted: Vec::new(),
                    }
                    .rebuild(&mut Clone::clone)
                        == *multiplier
                })
            })?;
            let actual = term.coefficient.literal()?;
            if !actual.value().is_finite() {
                return None;
            }
            let quotient = Expr::scalar_quotient(actual, coefficient, 1);
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

    fn consume(&mut self, selected: LinearMatch, destination: &Expr) -> (RemainingOperand, bool) {
        let coefficient = Expr::number(selected.scale);
        let mut exhausted_term = false;
        for (index, required) in selected.terms {
            if let Some(term) = self.terms.get_mut(index) {
                let consumed = Expr::product(vec![required, coefficient.clone()]);
                term.coefficient = Sub::sub(&term.coefficient, consumed);
                term.changed = true;
                exhausted_term |= term.coefficient.is_zero_num();
            }
        }
        let inserted = if coefficient.is_one_num() {
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
            Some(factor) => RemainingOperand::Factored { factor, inserted },
            None => RemainingOperand::Fixed(inserted),
        };
        (replacement, exhausted_term)
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
            coefficient = Expr::scalar_quotient(left, right, 1);
        }
        Self {
            original: original.clone(),
            coefficient,
            base,
            changed: false,
        }
    }
}
