//! Exact associative groups selected from original, canonically ordered operands.

use alloc::{borrow::Cow, vec, vec::Vec};

use num_anafis::{Number, RationalType};

use super::{
    AdditiveOperands, AlgebraicOperands, BuiltinFunction, ConcreteMatcher, ExactSources, Expr,
    FactorSide, LinearOperands, Node, NumericValue, PartialSelection, PowerProjection,
    RemainingOperand, Selection,
};

/// Filter describing which monomials of a `simple_bases` polynomial can
/// possibly match any source in a [`ConcreteMatcher`].
pub enum SimplePolyFilter {
    /// No source can match any `simple_bases` monomial or its descendants.
    None,
    /// A source can match a `simple_bases` monomial only if all `(base, min_exponent)`
    /// requirements of at least one entry hold in that monomial.
    Sources(Vec<Vec<(Expr, u32)>>),
    /// Conservative fallback: a literal source may match inside any monomial.
    Any,
}

/// Structural source summary allowing a substitution pass to skip projecting
/// unaffected compact polynomials.
pub struct MatcherHints {
    pub has_additive: bool,
    pub has_monomial: bool,
    pub poly_filter: SimplePolyFilter,
}

enum SourceClassification {
    Literal,
    Impossible,
    Requires(Vec<(Expr, u32)>),
}

fn classify_simple_source(source: &Expr) -> SourceClassification {
    let simple_leaf = |expr: &Expr| matches!(expr.node(), Node::Symbol(_) | Node::Pi | Node::Euler);
    let min_exp_of = |projection: &PowerProjection<'_>| -> Option<u32> {
        if !projection.nonzero && projection.denominator.get() == 1 && projection.exponent.get() > 0
        {
            u32::try_from(projection.exponent.get()).ok()
        } else {
            Some(1)
        }
    };
    if source.literal().is_some() {
        return SourceClassification::Literal;
    }
    if simple_leaf(source) {
        return SourceClassification::Requires(vec![(source.clone(), 1)]);
    }
    if let Some(projection) = PowerProjection::new(source) {
        if !simple_leaf(projection.base) {
            return SourceClassification::Impossible;
        }
        return min_exp_of(&projection).map_or(SourceClassification::Impossible, |min_exp| {
            SourceClassification::Requires(vec![(projection.base.clone(), min_exp)])
        });
    }
    if matches!(source.node(), Node::Product(_) | Node::Div(..)) {
        let mut reqs: Vec<(Expr, u32)> = Vec::new();
        for (factor, _) in AlgebraicOperands::factors(source) {
            if factor.literal().is_some() {
                continue;
            }
            let (base, min_exp) = if simple_leaf(factor) {
                (factor.clone(), 1)
            } else if let Some(projection) = PowerProjection::new(factor)
                && simple_leaf(projection.base)
                && let Some(min_exp) = min_exp_of(&projection)
            {
                (projection.base.clone(), min_exp)
            } else {
                return SourceClassification::Impossible;
            };
            if let Some(existing) = reqs.iter_mut().find(|(known, _)| *known == base) {
                existing.1 = existing.1.max(min_exp);
            } else {
                reqs.push((base, min_exp));
            }
        }
        return if reqs.is_empty() {
            SourceClassification::Literal
        } else {
            SourceClassification::Requires(reqs)
        };
    }
    SourceClassification::Impossible
}

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

    /// Build structural source hints for skipping unaffected compact polynomials.
    pub fn hints(&self) -> MatcherHints {
        let has_additive =
            (self.families & Self::ADDITIVE) != 0 || self.groups.iter().any(|group| group.sum);
        let has_monomial = (self.families & (Self::MULTIPLICATIVE | Self::POWERS)) != 0
            || self.groups.iter().any(|group| !group.sum);
        let poly_filter = if has_additive {
            SimplePolyFilter::Any
        } else {
            let mut requirements = Vec::new();
            let mut inspect = |source: &Expr| -> bool {
                match classify_simple_source(source) {
                    SourceClassification::Literal => false,
                    SourceClassification::Impossible => true,
                    SourceClassification::Requires(reqs) => {
                        requirements.push(reqs);
                        true
                    }
                }
            };
            let non_literal = match &self.exact {
                ExactSources::One(entry) => inspect(entry.from()),
                ExactSources::Many(entries) => entries.iter().all(|(source, _)| inspect(source)),
            };
            if !non_literal {
                SimplePolyFilter::Any
            } else if requirements.is_empty() {
                SimplePolyFilter::None
            } else {
                SimplePolyFilter::Sources(requirements)
            }
        };
        MatcherHints {
            has_additive,
            has_monomial,
            poly_filter,
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
        let mut claimed = Vec::new();
        let mut inserted = Vec::new();
        for group in self.groups.iter().filter(|group| group.sum == sum) {
            while group.claim(&operands, &mut available, &mut claimed) {
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
                        Node::integer_power(&base, exponent.get())
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
                        Node::integer_power(&root, exponent.get())
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
                        operands.push(Node::scalar_quotient(
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
    /// Availability changes only when every required operand is found, so the
    /// positions are recorded in the caller's buffer and applied at the end.
    fn claim(&self, operands: &[Expr], available: &mut [bool], claimed: &mut Vec<usize>) -> bool {
        claimed.clear();
        let mut required = self.operands.iter();
        let mut next = required.next();
        for (index, (candidate, keep)) in operands.iter().zip(available.iter()).enumerate() {
            let Some(expected) = next else {
                break;
            };
            if !*keep {
                continue;
            }
            if candidate == expected {
                claimed.push(index);
                next = required.next();
            } else if Node::compare(candidate, expected).is_gt() {
                return false;
            }
        }
        if next.is_some() {
            return false;
        }
        for index in claimed.iter() {
            if let Some(keep) = available.get_mut(*index) {
                *keep = false;
            }
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
