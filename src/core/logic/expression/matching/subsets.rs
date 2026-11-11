//! Exact associative groups selected from original, canonically ordered operands.

use alloc::{borrow::Cow, vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactSources, Expr,
    FactorSide, LinearOperands, Node, NumericValue, PartialSelection, RemainingOperand, Selection,
};

#[derive(Debug)]
pub struct OperandSource {
    pub sum: bool,
    pub operands: Vec<Expr>,
    pub destination: Expr,
}

impl<'entries> ConcreteMatcher<'entries> {
    /// The source family of sums: counted addends and common additive factors.
    pub const ADDITIVE: u8 = 1;
    /// The source family of products, quotients, and positive integer powers.
    pub const MULTIPLICATIVE: u8 = 2;
    /// A multiplicative source with a finite nonzero numerator coefficient, which
    /// a unit slot supplies to an input that has none.
    pub const NUMERATOR_UNIT: u8 = 4;
    /// A multiplicative source with a finite nonzero denominator coefficient.
    pub const DENOMINATOR_UNIT: u8 = 8;
    /// The source family of powers, quotients, and square roots, matched as
    /// exact rational power groups on one principal base.
    pub const POWERS: u8 = 16;

    /// Prepare source families once for a transformation pass.
    pub fn new(exact: ExactSources<'entries>) -> Self {
        let mut families = 0;
        let mut groups = Vec::new();
        let mut add = |source: &Expr, single: Option<&Expr>| {
            match AlgebraicOperands::mode(source) {
                Some(true) => families |= Self::ADDITIVE,
                Some(false) => {
                    families |= Self::MULTIPLICATIVE;
                    for (factor, side) in AlgebraicOperands::factors(source) {
                        if factor.literal().is_some_and(|value| {
                            value.value().is_finite() && !value.value().is_zero()
                        }) {
                            families |= match side {
                                FactorSide::Numerator => Self::NUMERATOR_UNIT,
                                FactorSide::Denominator => Self::DENOMINATOR_UNIT,
                            };
                        }
                    }
                }
                None => {}
            }
            if LinearOperands::supports(source) {
                families |= Self::ADDITIVE;
            }
            if matches!(source.node(), Node::Pow(..) | Node::Div(..))
                || matches!(source.node(), Node::Function { name, .. } if *name == BuiltinFunction::Sqrt.function())
            {
                families |= Self::POWERS;
            }
            if let Some(destination) = single {
                groups.extend(OperandSource::new(source, destination.clone()));
            }
        };
        match &exact {
            ExactSources::One(entry) => add(entry.from(), Some(entry.to())),
            ExactSources::Many(entries) => {
                for (source, _) in entries.iter() {
                    add(source, None);
                }
            }
        }
        Self {
            exact,
            groups,
            families,
        }
    }

    /// Reconstruct one selected node without traversing inserted destinations.
    pub fn replace(&self, expression: &Expr) -> Option<Expr> {
        self.select(expression).map(|selection| {
            if matches!(&self.exact, ExactSources::One(entry) if entry.from() == entry.to()) {
                return expression.clone();
            }
            match selection {
                Selection::Whole(value) => value,
                Selection::Partial(partial) => partial.rebuild(),
            }
        })
    }

    /// Select exact parents first, then disjoint groups from original operands.
    pub fn select_explicit(&self, expression: &Expr) -> Option<Selection> {
        let exact = match &self.exact {
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
            while group.claim(&operands, &mut available) {
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
    /// Visit the expressions that [`Self::rebuild`] takes from the remainder,
    /// in the order it takes them, so that a caller can replace each by its
    /// transformed value. The visits depend only on the remainder's shape.
    pub fn for_each_input(&mut self, visit: &mut impl FnMut(&mut Expr)) {
        for operand in &mut self.remainder {
            match operand {
                RemainingOperand::Original(input)
                | RemainingOperand::Divisor(input)
                | RemainingOperand::Counted { base: input, .. }
                | RemainingOperand::Scaled { base: input, .. }
                | RemainingOperand::Power { base: input, .. }
                | RemainingOperand::DivisorCounted { base: input, .. } => visit(input),
                RemainingOperand::Factored(factored) => factored.for_each_input(visit),
                RemainingOperand::Fixed(_) | RemainingOperand::FixedDivisor(_) => {}
            }
        }
    }

    /// Combine the remainder with the untouched destinations.
    pub fn rebuild(self) -> Expr {
        let mut operands = Vec::new();
        let mut divisors = Vec::new();
        for operand in self.remainder {
            let value = match operand {
                RemainingOperand::Original(original) | RemainingOperand::Fixed(original) => {
                    original
                }
                RemainingOperand::Counted { base, count } => {
                    if count == 1 {
                        base
                    } else if self.sum {
                        Expr::product(vec![Expr::number(count), base])
                    } else {
                        base.pow(count)
                    }
                }
                RemainingOperand::Scaled { base, coefficient } => {
                    Expr::product(vec![coefficient, base])
                }
                RemainingOperand::Power {
                    base,
                    exponent,
                    denominator,
                } => {
                    if denominator.get() == 1 {
                        Expr::integer_power(&base, exponent.get())
                    } else if let Ok(numerator) = i64::try_from(exponent.get()) {
                        base.pow(Number::from_rational(RationalType::from_parts(
                            numerator,
                            denominator.get(),
                        )))
                    } else {
                        let root = base.pow(Number::from_rational(RationalType::from_parts(
                            1,
                            denominator.get(),
                        )));
                        Expr::integer_power(&root, exponent.get())
                    }
                }
                RemainingOperand::Divisor(original) => {
                    divisors.push(original);
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
                    divisors.push(value);
                    continue;
                }
                RemainingOperand::DivisorCounted { base, count } => {
                    divisors.push(if count == 1 { base } else { base.pow(count) });
                    continue;
                }
                RemainingOperand::Factored(factored) => factored.rebuild(),
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

impl OperandSource {
    pub fn new(from: &Expr, destination: Expr) -> Option<Self> {
        let (sum, operands) = Self::operands(from)?;
        (!operands.is_empty()).then_some(Self {
            sum,
            operands: operands.into_owned(),
            destination,
        })
    }

    /// Claim this group's operands among the available ones, in canonical order.
    /// Availability changes only when every required operand is found.
    fn claim(&self, operands: &[Expr], available: &mut [bool]) -> bool {
        let mut required = self.operands.iter();
        let mut next = required.next();
        let mut claimed = Vec::with_capacity(self.operands.len());
        for (candidate, keep) in operands.iter().zip(available) {
            let Some(expected) = next else {
                break;
            };
            if !*keep {
                continue;
            }
            if candidate == expected {
                claimed.push(keep);
                next = required.next();
            } else if Node::compare(candidate, expected).is_gt() {
                return false;
            }
        }
        if next.is_some() {
            return false;
        }
        for keep in claimed {
            *keep = false;
        }
        true
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
