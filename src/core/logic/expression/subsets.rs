//! Exact associative groups selected from original, canonically ordered operands.

use alloc::{borrow::Cow, vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactSources, Expr,
    FactorSide, LinearOperands, Node, NumericValue, OperandSource, PartialSelection,
    RemainingOperand, Selection,
};

impl<'entries> ConcreteMatcher<'entries> {
    /// Prepare source families once for a transformation pass.
    pub fn new(exact: ExactSources<'entries>) -> Self {
        let mut matcher = Self {
            exact,
            groups: Vec::new(),
            arithmetic: 0,
        };
        let mut add = |source: &'entries Expr, destination: &'entries Expr| {
            matcher.arithmetic |=
                AlgebraicOperands::mode(source).map_or(0, |sum| if sum { 1 } else { 2 });
            if LinearOperands::supports(source) {
                matcher.arithmetic |= 1;
            }
            if matches!(source.node(), Node::Pow(..) | Node::Div(..))
                || matches!(source.node(), Node::Function { name, .. } if *name == BuiltinFunction::Sqrt.function())
            {
                matcher.arithmetic |= 16;
            }
            if AlgebraicOperands::mode(source) == Some(false) {
                for (factor, side) in AlgebraicOperands::factors(source) {
                    if factor
                        .literal()
                        .is_some_and(|value| value.value().is_finite() && !value.value().is_zero())
                    {
                        matcher.arithmetic |= match side {
                            FactorSide::Numerator => 4,
                            FactorSide::Denominator => 8,
                        };
                    }
                }
            }
            if matches!(matcher.exact, ExactSources::One(_)) {
                matcher
                    .groups
                    .extend(OperandSource::new(source, destination));
            }
        };
        match matcher.exact {
            ExactSources::One(entry) => add(entry.from(), entry.to()),
            ExactSources::Many(entries) => {
                for (source, destination) in entries.iter() {
                    add(source, destination);
                }
            }
        }
        matcher
    }

    /// Reconstruct one selected node without traversing inserted destinations.
    pub fn replace(&self, expression: &Expr) -> Option<Expr> {
        self.select(expression).map(|selection| {
            if matches!(self.exact, ExactSources::One(entry) if entry.from() == entry.to()) {
                return expression.clone();
            }
            match selection {
                Selection::Whole(value) => value,
                Selection::Partial(partial) => partial.rebuild(&mut Clone::clone),
            }
        })
    }

    /// Select exact parents first, then disjoint groups from original operands.
    pub fn select_explicit(&self, expression: &Expr) -> Option<Selection> {
        let exact = match self.exact {
            ExactSources::One(entry) => (expression == entry.from()).then(|| entry.to().clone()),
            ExactSources::Many(entries) => entries.get(expression).cloned(),
        };
        if let Some(replacement) = exact {
            return Some(Selection::Whole(replacement));
        }
        if self.groups.is_empty() {
            return None;
        }
        let (sum, operands) = OperandSource::operands(expression)?;
        if !self
            .groups
            .iter()
            .any(|group| group.sum == sum && group.operands.len() <= operands.len())
        {
            return None;
        }
        let mut available = vec![true; operands.len()];
        let mut inserted = Vec::new();
        for group in self.groups.iter().filter(|group| group.sum == sum) {
            while let Some(indices) = group.find(&operands, &available) {
                for index in indices {
                    if let Some(slot) = available.get_mut(index) {
                        *slot = false;
                    }
                }
                inserted.push(group.destination.clone());
            }
        }
        if inserted.is_empty() {
            return None;
        }
        let remainder = operands
            .iter()
            .zip(available)
            .filter(|(_, keep)| *keep)
            .map(|(operand, _)| RemainingOperand::Original(operand.clone()))
            .collect();
        Some(Selection::Partial(PartialSelection {
            sum,
            remainder,
            inserted,
        }))
    }
}

impl PartialSelection {
    /// Transform unmatched original operands, then insert untouched destinations.
    pub fn rebuild(self, mapped: &mut impl FnMut(&Expr) -> Expr) -> Expr {
        let mut operands = Vec::new();
        let mut divisors = Vec::new();
        for operand in &self.remainder {
            let value = match operand {
                RemainingOperand::Original(original) => mapped(original),
                RemainingOperand::Counted { base, count } => {
                    let transformed = mapped(base);
                    if *count == 1 {
                        transformed
                    } else if self.sum {
                        Expr::product(vec![Expr::number(*count), transformed])
                    } else {
                        transformed.pow(*count)
                    }
                }
                RemainingOperand::Fixed(value) => value.clone(),
                RemainingOperand::Scaled { base, coefficient } => {
                    Expr::product(vec![coefficient.clone(), mapped(base)])
                }
                RemainingOperand::Power {
                    base,
                    exponent,
                    denominator,
                } => {
                    let value = mapped(base);
                    if denominator.get() == 1 {
                        Expr::integer_power(&value, exponent.get())
                    } else if let Ok(numerator) = i64::try_from(exponent.get()) {
                        value.pow(Number::from_rational(RationalType::from_parts(
                            numerator,
                            denominator.get(),
                        )))
                    } else {
                        let root = value.pow(Number::from_rational(RationalType::from_parts(
                            1,
                            denominator.get(),
                        )));
                        Expr::integer_power(&root, exponent.get())
                    }
                }
                RemainingOperand::Divisor(original) => {
                    divisors.push(mapped(original));
                    continue;
                }
                RemainingOperand::FixedDivisor(value) => {
                    if let Some(denominator) = value.literal()
                        && denominator.value().is_finite()
                        && !denominator.value().is_zero()
                    {
                        operands.push(Expr::scalar_quotient(
                            &NumericValue::new(Number::one()),
                            denominator,
                            1,
                        ));
                        continue;
                    }
                    divisors.push(value.clone());
                    continue;
                }
                RemainingOperand::DivisorCounted { base, count } => {
                    let value = mapped(base);
                    divisors.push(if *count == 1 {
                        value
                    } else {
                        value.pow(*count)
                    });
                    continue;
                }
                RemainingOperand::Factored { factor, inserted } => Self {
                    sum: false,
                    remainder: factor.remainder(),
                    inserted: vec![inserted.clone()],
                }
                .rebuild(mapped),
            };
            operands.push(value);
        }
        operands.extend(self.inserted);
        let result = if self.sum {
            Expr::sum(operands)
        } else {
            Expr::product(operands)
        };
        if divisors.is_empty() {
            result
        } else {
            Node::normalize_quotient(result, Expr::product(divisors))
        }
    }
}

impl<'entries> OperandSource<'entries> {
    pub fn new(from: &Expr, destination: &'entries Expr) -> Option<Self> {
        let (sum, operands) = Self::operands(from)?;
        (!operands.is_empty()).then_some(Self {
            sum,
            operands: operands.into_owned(),
            destination,
        })
    }

    fn find(&self, operands: &[Expr], available: &[bool]) -> Option<Vec<usize>> {
        let mut required = self.operands.iter();
        let mut next = required.next();
        let mut selected = Vec::with_capacity(self.operands.len());
        for (index, (candidate, keep)) in operands.iter().zip(available).enumerate() {
            let Some(expected) = next else {
                return Some(selected);
            };
            if !keep {
                continue;
            }
            if candidate == expected {
                selected.push(index);
                next = required.next();
            } else if Node::compare(candidate, expected).is_gt() {
                return None;
            }
        }
        next.is_none().then_some(selected)
    }

    fn operands(expression: &Expr) -> Option<(bool, Cow<'_, [Expr]>)> {
        match expression.node() {
            Node::Sum(_) | Node::Poly(_) => {
                expression.additive_operands().map(|terms| (true, terms))
            }
            Node::Product(factors) => Some((false, Cow::Borrowed(factors))),
            Node::Number(_)
            | Node::ImaginaryUnit
            | Node::Pi
            | Node::Euler
            | Node::Symbol(_)
            | Node::Function { .. }
            | Node::Div(..)
            | Node::Pow(..)
            | Node::Derivative { .. } => None,
        }
    }
}
