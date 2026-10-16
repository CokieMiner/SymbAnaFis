//! Canonical sums and commuting scalar products.
//!
//! Every active expression is a real or complex scalar. Scalar i is central.
//! Clifford generators are outside this contract: an extension must introduce
//! an ordered multiplication boundary before it can use these constructors.

use core::slice::from_ref;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::collections::HashMap;

use super::{Expr, Node, NumericValue};

impl Node {
    /// Normalize addition while retaining zero coefficients of unknown bases.
    pub fn normalize_sum(mut terms: Vec<Expr>) -> Expr {
        if terms.len() <= 1 {
            return terms.pop().unwrap_or_else(|| Expr::number(Number::zero()));
        }
        let flat = Self::flatten(terms, true);
        let (numbers, symbolic) = Self::partition_numbers(flat);
        let mut normalized = Self::fold_numbers(numbers, true);
        if Self::distinct_symbol_tail(&symbolic) {
            if normalized.is_empty() {
                return Self::finish(symbolic, true);
            }
            normalized.extend(symbolic);
            if normalized.len() > 1 {
                normalized.retain(|term| !term.is_zero_num());
            }
            return Self::finish(normalized, true);
        }
        let mut groups: HashMap<Expr, Vec<(NumericValue, Expr)>> = HashMap::new();
        for term in symbolic {
            let (coefficient, base) = Self::split_coefficient(&term);
            groups.entry(base).or_default().push((coefficient, term));
        }
        for (base, original) in groups {
            // A singleton group already has a canonical coefficient and base.
            // Reuse its input instead of allocating coefficients and rebuilding 1*base.
            if original.len() == 1 {
                normalized.extend(original.into_iter().map(|(_, term)| term));
                continue;
            }
            let mut coefficients: Vec<_> = original
                .iter()
                .map(|(coefficient, _)| coefficient.clone())
                .collect();
            coefficients.sort_by(NumericValue::compare);
            if let Some(value) = Self::finite_fold(&coefficients, true) {
                normalized.push(Expr::product(vec![Expr::number(value), base]));
            } else {
                normalized.extend(original.into_iter().map(|(_, term)| term));
            }
        }
        if normalized.len() > 1 {
            normalized.retain(|term| !term.is_zero_num());
        }
        normalized = Self::collect_polynomials(normalized);
        Self::sort_operands(&mut normalized);
        Self::finish(normalized, true)
    }

    /// Normalize multiplication of commuting scalars, preserving unknown factors.
    pub fn normalize_product(mut factors: Vec<Expr>) -> Expr {
        if factors.len() <= 1 {
            return factors.pop().unwrap_or_else(|| Expr::number(Number::one()));
        }
        let flat = Self::flatten(factors, false);
        // This order is valid for central complex scalars, including i. It must
        // not be extended to noncommuting generators by sorting their names.
        let (numbers, symbolic) = Self::partition_numbers(flat);
        let (collected, regrouped) = Self::collect_positive_factors(symbolic);
        let mut normalized = Self::fold_numbers(numbers, false);
        normalized.extend(collected);
        if normalized.len() > 1 {
            normalized.retain(|factor| !factor.is_one_num());
        }
        // Numeric folding only changes the literal prefix. The symbolic tail
        // retains the canonical order established by flattening or linear merge.
        let prefix = Self::numeric_prefix(&normalized);
        if regrouped {
            Self::sort_operands(&mut normalized);
        } else if let Some(literals) = normalized.get_mut(..prefix) {
            Self::sort_operands(literals);
        }
        Self::finish(normalized, false)
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

    fn partition_numbers(mut inputs: Vec<Expr>) -> (Vec<NumericValue>, Vec<Expr>) {
        let prefix = Self::numeric_prefix(&inputs);
        let numbers = inputs
            .extract_if(..prefix, |input| {
                input
                    .literal()
                    .is_some_and(|value| value.value().is_finite())
            })
            .filter_map(|input| input.literal().cloned())
            .collect();
        (numbers, inputs)
    }

    fn numeric_prefix(inputs: &[Expr]) -> usize {
        // Node rank places every Number, followed by central i, before all
        // symbolic kinds. Finiteness is checked within this prefix, not assumed.
        inputs
            .partition_point(|input| matches!(input.node(), Self::Number(_) | Self::ImaginaryUnit))
    }

    fn distinct_symbol_tail(terms: &[Expr]) -> bool {
        let mut previous = None;
        terms.iter().all(|term| {
            if !matches!(term.node(), Self::Symbol(_)) {
                return false;
            }
            let distinct = previous.is_none_or(|prior: &Expr| !prior.same_node(term));
            previous = Some(term);
            distinct
        })
    }

    fn fold_numbers(numbers: Vec<NumericValue>, sum: bool) -> Vec<Expr> {
        if numbers.is_empty() {
            return Vec::new();
        }
        Self::finite_fold(&numbers, sum).map_or_else(
            || {
                numbers
                    .into_iter()
                    .map(|value| Expr::number(value.value().clone()))
                    .collect()
            },
            |value| vec![Expr::number(value)],
        )
    }

    fn split_coefficient(term: &Expr) -> (NumericValue, Expr) {
        if let Self::Product(factors) = term.node()
            && let Some((first, rest)) = factors.split_first()
            && let Some(coefficient) = first.literal()
            && coefficient.value().is_finite()
        {
            return (coefficient.clone(), Expr::product(rest.to_vec()));
        }
        (NumericValue::new(Number::one()), term.clone())
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

    fn finite_fold(values: &[NumericValue], sum: bool) -> Option<Number> {
        let (first, rest) = values.split_first()?;
        rest.iter()
            .try_fold(first.value().clone(), |accumulated, value| {
                let left = NumericValue::new(accumulated);
                if sum {
                    left.add(value)
                } else {
                    left.mul(value)
                }
            })
    }
}
