//! Canonical sums and commuting scalar products.
//!
//! Expressions are real or complex scalars. Multiplication is commutative,
//! and the imaginary unit is central.

use core::slice::from_ref;

use alloc::{vec, vec::Vec};

use hashbrown::hash_map::Entry;
use num_anafis::Number;

use crate::collections::HashMap;

use super::{Expr, Node, NumericValue, ProductFrame};

struct SumGroup {
    first: Expr,
    repeated: Vec<Expr>,
}

impl Node {
    /// Normalize addition while retaining zero coefficients of unknown bases.
    pub fn normalize_sum(mut terms: Vec<Expr>) -> Expr {
        if terms.len() <= 1 {
            return terms.pop().unwrap_or_else(|| Expr::number(Number::zero()));
        }
        let mut normalized = Self::flatten(terms, true);
        Self::fold_number_prefix(&mut normalized, true);
        if Self::distinct_atomic_tail(&normalized) {
            if normalized.len() > 1 {
                normalized.retain(|term| !term.is_zero_num());
            }
            return Self::finish(normalized, true);
        }
        if let [first, second] = normalized.as_slice() {
            let (first_coefficient, first_factors) = Self::coefficient_parts(first);
            let (second_coefficient, second_factors) = Self::coefficient_parts(second);
            // Equal factor slices collect directly without a grouping table.
            if first_factors == second_factors {
                let unit = NumericValue::new(Number::one());
                let mut coefficients = [
                    first_coefficient.unwrap_or(&unit),
                    second_coefficient.unwrap_or(&unit),
                ];
                coefficients.sort_unstable_by(|left, right| left.compare(right));
                if let Some(value) = Self::finite_fold(coefficients.into_iter(), true) {
                    let base = if let [single] = first_factors {
                        single.clone()
                    } else {
                        Expr::product(first_factors.to_vec())
                    };
                    normalized.clear();
                    normalized.push(Expr::number(value));
                    normalized.push(base);
                    return Self::normalize_product(normalized);
                }
            }
        }
        let mut groups: HashMap<Expr, SumGroup> = HashMap::new();
        for term in normalized.extract_if(.., |term| {
            term.literal()
                .is_none_or(|value| !value.value().is_finite())
        }) {
            let (_, factors) = Self::coefficient_parts(&term);
            let base = if let [base] = factors {
                base.clone()
            } else {
                Expr::product(factors.to_vec())
            };
            match groups.entry(base) {
                Entry::Vacant(entry) => {
                    entry.insert(SumGroup {
                        first: term,
                        repeated: Vec::new(),
                    });
                }
                Entry::Occupied(mut entry) => entry.get_mut().repeated.push(term),
            }
        }
        for (base, mut group) in groups {
            if group.repeated.is_empty() {
                normalized.push(group.first);
                continue;
            }
            group.repeated.push(group.first);
            let unit = NumericValue::new(Number::one());
            let mut coefficients: Vec<_> = group
                .repeated
                .iter()
                .map(|term| Self::coefficient_parts(term).0.unwrap_or(&unit))
                .collect();
            coefficients.sort_unstable_by(|left, right| left.compare(right));
            if let Some(value) = Self::finite_fold(coefficients.into_iter(), true) {
                normalized.push(Expr::product(vec![Expr::number(value), base]));
            } else {
                normalized.extend(group.repeated);
            }
        }
        if normalized.len() > 1 {
            normalized.retain(|term| !term.is_zero_num());
        }
        normalized = Self::collect_polynomials(normalized);
        Self::sort_operands(&mut normalized);
        Self::finish(normalized, true)
    }

    /// Normalize scalar products with coefficients outside shared quotients.
    /// Unknown factors and every denominator retain their domain exclusions.
    pub fn normalize_product(mut factors: Vec<Expr>) -> Expr {
        let mut pending = Vec::new();
        'normalize: loop {
            let mut value = 'product: {
                if factors.len() <= 1 {
                    break 'product factors.pop().unwrap_or_else(|| Expr::number(Number::one()));
                }
                let mut flat = Self::flatten(factors, false);
                // Only powers and derivatives sort after quotients.
                if flat
                    .iter()
                    .rev()
                    .find(|factor| {
                        !matches!(factor.node(), Self::Pow(..) | Self::Derivative { .. })
                    })
                    .is_some_and(|factor| matches!(factor.node(), Self::Div(..)))
                {
                    let prefix = Self::numeric_prefix(&flat);
                    // A finite scalar prefix and one quotient already have
                    // canonical coefficient placement.
                    let weighted_quotient = matches!(flat.get(prefix..), Some([quotient])
                        if matches!(quotient.node(), Self::Div(..)))
                        && flat.iter().take(prefix).all(|factor| {
                            factor
                                .literal()
                                .is_some_and(|value| value.value().is_finite())
                        });
                    if weighted_quotient && let [coefficient, quotient] = flat.as_slice() {
                        break 'product if coefficient.is_one_num() {
                            quotient.clone()
                        } else {
                            Self::finish(flat, false)
                        };
                    }
                    if !weighted_quotient {
                        let mut numerators = Vec::with_capacity(flat.len());
                        let mut denominators = Vec::new();
                        for factor in flat {
                            if let Self::Div(numerator, denominator) = factor.node() {
                                numerators.push(numerator.clone());
                                denominators.push(denominator.clone());
                            } else {
                                numerators.push(factor);
                            }
                        }
                        pending.push(ProductFrame::Numerator { denominators });
                        factors = numerators;
                        continue 'normalize;
                    }
                }
                Self::fold_number_prefix(&mut flat, false);
                let (mut normalized, rebuild) = Self::collect_positive_factors(flat);
                if normalized.len() > 1 {
                    normalized.retain(|factor| !factor.is_one_num());
                }
                if rebuild {
                    factors = normalized;
                    continue 'normalize;
                }
                let prefix = Self::numeric_prefix(&normalized);
                if let Some(literals) = normalized.get_mut(..prefix) {
                    Self::sort_operands(literals);
                }
                Self::finish(normalized, false)
            };
            while let Some(frame) = pending.pop() {
                match frame {
                    ProductFrame::Numerator { denominators } => {
                        pending.push(ProductFrame::Denominator { numerator: value });
                        factors = denominators;
                        continue 'normalize;
                    }
                    ProductFrame::Denominator { numerator } => {
                        if let Some((top, bottom)) = Self::split_quotient(&numerator) {
                            pending.push(ProductFrame::Denominator { numerator: top });
                            factors = vec![bottom, value];
                            continue 'normalize;
                        }
                        value = Self::normalize_quotient(numerator, value);
                    }
                }
            }
            return value;
        }
    }

    fn flatten(mut inputs: Vec<Expr>, sum: bool) -> Vec<Expr> {
        if let [left, right] = inputs.as_slice()
            && (Self::is_nary(left, sum) || Self::is_nary(right, sum))
        {
            let merged =
                Self::merge_operands(Self::operands(left, sum), Self::operands(right, sum));
            return if sum {
                Self::unpack_polynomials(merged)
            } else {
                merged
            };
        }
        if !inputs.iter().any(|input| {
            Self::is_nary(input, sum) || (sum && matches!(input.node(), Self::Poly(_)))
        }) {
            Self::sort_operands(&mut inputs);
            return inputs;
        }
        let mut flat = Vec::with_capacity(inputs.len());
        for input in inputs {
            match input.node() {
                Self::Sum(children) if sum => flat.extend(children.iter().cloned()),
                Self::Product(children) if !sum => flat.extend(children.iter().cloned()),
                Self::Number(_)
                | Self::ImaginaryUnit
                | Self::Pi
                | Self::Euler
                | Self::Symbol(_)
                | Self::Function { .. }
                | Self::Sum(_)
                | Self::Poly(_)
                | Self::Product(_)
                | Self::Div(..)
                | Self::Pow(..)
                | Self::Derivative { .. } => flat.push(input),
            }
        }
        if sum {
            flat = Self::unpack_polynomials(flat);
        }
        Self::sort_operands(&mut flat);
        flat
    }

    fn unpack_polynomials(terms: Vec<Expr>) -> Vec<Expr> {
        if !terms
            .iter()
            .any(|term| matches!(term.node(), Self::Poly(_)))
        {
            return terms;
        }
        let mut result = Vec::with_capacity(terms.len());
        for term in terms {
            if let Self::Poly(polynomial) = term.node() {
                result.extend(Self::mapped_polynomial(polynomial, Clone::clone));
            } else {
                result.push(term);
            }
        }
        Self::sort_operands(&mut result);
        result
    }

    fn is_nary(expression: &Expr, sum: bool) -> bool {
        matches!(expression.node(), Self::Sum(_) if sum)
            || matches!(expression.node(), Self::Product(_) if !sum)
    }

    fn operands(expression: &Expr, sum: bool) -> &[Expr] {
        match expression.node() {
            Self::Sum(children) if sum => children,
            Self::Product(children) if !sum => children,
            Self::Number(_)
            | Self::ImaginaryUnit
            | Self::Pi
            | Self::Euler
            | Self::Symbol(_)
            | Self::Function { .. }
            | Self::Sum(_)
            | Self::Poly(_)
            | Self::Product(_)
            | Self::Div(..)
            | Self::Pow(..)
            | Self::Derivative { .. } => from_ref(expression),
        }
    }

    fn merge_operands(left: &[Expr], right: &[Expr]) -> Vec<Expr> {
        let mut merged = Vec::new();
        if let Some(capacity) = left.len().checked_add(right.len()) {
            merged.reserve_exact(capacity);
        }
        let mut first = left.iter().peekable();
        let mut second = right.iter().peekable();
        while let (Some(a), Some(b)) = (first.peek(), second.peek()) {
            if Self::compare(a, b).is_gt() {
                merged.extend(second.next().cloned());
            } else {
                merged.extend(first.next().cloned());
            }
        }
        merged.extend(first.cloned());
        merged.extend(second.cloned());
        merged
    }

    pub fn sort_operands(operands: &mut [Expr]) {
        if !operands.is_sorted_by(|left, right| Self::compare(left, right).is_le()) {
            operands.sort_unstable_by(Self::compare);
        }
    }

    /// Fold finite prefix literals in place, retaining singleton and failed folds.
    fn fold_number_prefix(inputs: &mut Vec<Expr>, sum: bool) {
        let prefix = Self::numeric_prefix(inputs);
        let numbers = inputs
            .iter()
            .take(prefix)
            .filter_map(|input| input.literal().filter(|value| value.value().is_finite()));
        if numbers.clone().nth(1).is_none() {
            return;
        }
        let Some(combined) = Self::finite_fold(numbers, sum) else {
            return;
        };
        let mut folded = Some(Expr::number(combined));
        inputs.retain_mut(|input| {
            if input
                .literal()
                .is_none_or(|value| !value.value().is_finite())
            {
                return true;
            }
            folded.take().is_some_and(|replacement| {
                *input = replacement;
                true
            })
        });
    }

    fn numeric_prefix(inputs: &[Expr]) -> usize {
        // Node rank places every Number, followed by central i, before all
        // symbolic kinds. Finiteness is checked within this prefix, not assumed.
        inputs
            .iter()
            .take_while(|input| matches!(input.node(), Self::Number(_) | Self::ImaginaryUnit))
            .count()
    }

    fn distinct_atomic_tail(terms: &[Expr]) -> bool {
        let mut previous = None;
        terms
            .iter()
            .skip_while(|term| {
                term.literal()
                    .is_some_and(|value| value.value().is_finite())
            })
            .all(|term| {
                if !matches!(
                    term.node(),
                    Self::Symbol(_)
                        | Self::Pi
                        | Self::Euler
                        | Self::Function { .. }
                        | Self::Div(..)
                        | Self::Derivative { .. }
                ) {
                    return false;
                }
                let distinct = previous.is_none_or(|prior: &Expr| prior != term);
                previous = Some(term);
                distinct
            })
    }

    /// Borrow a finite scalar coefficient and the remaining multiplicative factors.
    pub fn coefficient_parts(term: &Expr) -> (Option<&NumericValue>, &[Expr]) {
        if let Some(value) = term.literal()
            && value.value().is_finite()
        {
            return (Some(value), &[]);
        }
        if let Self::Product(factors) = term.node()
            && let Some((first, rest)) = factors.split_first()
            && let Some(coefficient) = first.literal()
            && coefficient.value().is_finite()
        {
            return (Some(coefficient), rest);
        }
        (None, from_ref(term))
    }

    fn finish(mut operands: Vec<Expr>, sum: bool) -> Expr {
        match operands.len() {
            0 => Expr::number(if sum { Number::zero() } else { Number::one() }),
            1 => operands.remove(0),
            _ => Expr::intern(if sum {
                Self::Sum(operands)
            } else {
                Self::Product(operands)
            }),
        }
    }

    fn finite_fold<'value>(
        mut values: impl Iterator<Item = &'value NumericValue>,
        sum: bool,
    ) -> Option<Number> {
        let first = values.next()?;
        values.try_fold(first.value().clone(), |accumulated, value| {
            let left = NumericValue::new(accumulated);
            if sum {
                left.add(value)
            } else {
                left.mul(value)
            }
        })
    }
}
