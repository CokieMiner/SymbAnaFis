//! Common scalar multiples of original additive terms.

use core::ops::Sub;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use super::{
    AdditiveOperands, ConcreteMatcher, ExactSources, Expr, LinearMatch, LinearOperands, LinearTerm,
    Node, NumericValue, PartialSelection, RemainingOperand, Selection,
};

impl ConcreteMatcher<'_> {
    /// Replace common scalar multiples while retaining original unmatched bases.
    pub fn select_linear(
        &self,
        expression: &Expr,
        explicit: Option<PartialSelection>,
    ) -> Option<Selection> {
        let Some(mut operands) = LinearOperands::new(expression) else {
            return explicit.map(Selection::Partial);
        };
        let mut inserted = Vec::new();
        let mut apply = |source: &Expr, destination: &Expr| {
            if let Some(selected) = operands.select(source) {
                inserted.push(operands.consume(selected, destination));
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
            .collect();
        Some(Selection::Partial(PartialSelection {
            sum: true,
            remainder,
            inserted,
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

    fn select(&self, source: &Expr) -> Option<LinearMatch> {
        let source_terms = Self::new(source)?;
        let explicit: Option<Vec<_>> = source_terms
            .terms
            .iter()
            .map(|required| {
                self.terms
                    .iter()
                    .position(|term| !term.changed && term.original.same_node(&required.original))
                    .map(|index| (index, required.coefficient.clone()))
            })
            .collect();
        if let Some(terms) = explicit {
            return Some(LinearMatch {
                terms,
                scale: Number::one(),
            });
        }
        let mut scale: Option<Number> = None;
        let mut selected = Vec::new();
        for required in source_terms.terms {
            let coefficient = required.coefficient.literal()?;
            if !coefficient.value().is_finite() || coefficient.value().is_zero() {
                return None;
            }
            let (index, term) =
                self.terms.iter().enumerate().find(|(_, term)| {
                    term.base == required.base && !term.coefficient.is_zero_num()
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
            selected.push((index, required.coefficient));
        }
        Some(LinearMatch {
            terms: selected,
            scale: scale?,
        })
    }

    fn consume(&mut self, selected: LinearMatch, destination: &Expr) -> Expr {
        let coefficient = Expr::number(selected.scale);
        for (index, required) in selected.terms {
            if let Some(term) = self.terms.get_mut(index) {
                let consumed = Expr::product(vec![required, coefficient.clone()]);
                term.coefficient = Sub::sub(&term.coefficient, consumed);
                term.changed = true;
            }
        }
        if coefficient.is_one_num() {
            return destination.clone();
        }
        if let Some(terms) = destination.additive_operands() {
            Expr::sum(
                terms
                    .iter()
                    .map(|term| Expr::product(vec![coefficient.clone(), term.clone()]))
                    .collect(),
            )
        } else {
            Expr::product(vec![coefficient, destination.clone()])
        }
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
