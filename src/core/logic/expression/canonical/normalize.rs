//! Canonical sums and commuting scalar products.
//!
//! Expressions are real or complex scalars. Multiplication is commutative,
//! and the imaginary unit is central.

#![expect(
    unsafe_code,
    reason = "the literal prefix and its compaction indices are bounded by the operand slice length, so indexing needs no bounds check"
)]

use core::slice::from_ref;

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use super::{Expr, MonomialStatus, Node, NumericValue};

/// A product with quotient factors, normalized in parts on an explicit stack.
enum ProductFrame {
    /// The numerators are being normalized; the denominators follow.
    Numerator { denominators: Vec<Expr> },
    /// A denominator is being normalized under its normalized numerator.
    Denominator { numerator: Expr },
}

impl Node {
    /// Allocation-free fast paths for binary addition on exact zero identities
    /// and foldable finite literals, falling back to canonical sum construction.
    pub fn add_binary(left: Expr, right: Expr) -> Expr {
        if let (Some(a), Some(b)) = (left.literal(), right.literal())
            && a.value().is_finite()
            && b.value().is_finite()
            && let Some(sum) = a.add(b)
        {
            return Expr::number(sum);
        }
        if left.is_exact_zero() {
            return right;
        }
        if right.is_exact_zero() {
            return left;
        }
        Expr::sum(vec![left, right])
    }

    /// Allocation-free fast paths for binary subtraction on exact zero
    /// identities and foldable finite literals, avoiding intermediate negation
    /// nodes when literals fold directly.
    pub fn sub_binary(left: Expr, right: Expr) -> Expr {
        if let (Some(a), Some(b)) = (left.literal(), right.literal())
            && a.value().is_finite()
            && b.value().is_finite()
            && let Some(negated) = NumericValue::new(Number::from_int(-1)).mul(b)
            && let Some(sum) = a.add(&NumericValue::new(negated))
        {
            return Expr::number(sum);
        }
        if right.is_exact_zero() {
            return left;
        }
        if left.is_exact_zero() {
            return Self::negate(right);
        }
        Expr::sum(vec![left, Self::negate(right)])
    }

    /// Allocation-free fast paths for binary multiplication on exact unit
    /// identities and foldable finite literals, falling back to canonical
    /// product construction.
    pub fn mul_binary(left: Expr, right: Expr) -> Expr {
        if let (Some(a), Some(b)) = (left.literal(), right.literal())
            && a.value().is_finite()
            && b.value().is_finite()
            && let Some(product) = a.mul(b)
        {
            return Expr::number(product);
        }
        if left.is_exact_one() {
            return right;
        }
        if right.is_exact_one() {
            return left;
        }
        Expr::product(vec![left, right])
    }

    /// Allocation-free fast paths for unary negation on finite literals and
    /// products with a leading finite coefficient, including `-(-x) = x`.
    pub fn negate(expression: Expr) -> Expr {
        if let Some(value) = expression.literal()
            && value.value().is_finite()
            && let Some(folded) = NumericValue::new(Number::from_int(-1)).mul(value)
        {
            return Expr::number(folded);
        }
        if let Self::Product(factors) = expression.node()
            && let Some((first, rest)) = factors.split_first()
            && let Some(coefficient) = first.literal()
            && coefficient.value().is_finite()
            && rest
                .first()
                .is_some_and(|second| second.literal().is_none())
            && let Some(folded) = NumericValue::new(Number::from_int(-1)).mul(coefficient)
        {
            if folded.is_one() && folded.is_exact() {
                return if let [single] = rest {
                    single.clone()
                } else {
                    Expr::intern(Self::Product(rest.to_vec()))
                };
            }
            let mut negated = Vec::with_capacity(factors.len());
            negated.push(Expr::number(folded));
            negated.extend_from_slice(rest);
            return Expr::intern(Self::Product(negated));
        }
        Expr::product(vec![Expr::number(-1), expression])
    }

    /// Normalize addition while retaining zero coefficients of unknown bases.
    pub fn normalize_sum(mut terms: Vec<Expr>) -> Expr {
        if terms.len() <= 1 {
            return terms.pop().unwrap_or_else(|| Expr::number(Number::zero()));
        }
        let status = Self::try_monomial_polynomial(&terms);
        if let MonomialStatus::Packed(polynomial) = status {
            return polynomial;
        }
        let operands = match Self::keep_compact_group(terms) {
            Ok(sum) => return sum,
            Err(operands) => operands,
        };
        let mut normalized = Self::flatten(operands, true);
        Self::fold_number_prefix(&mut normalized, true);
        if Self::distinct_atomic_tail(&normalized) {
            if normalized.len() > 1 {
                normalized.retain(|term| !term.is_exact_zero());
            }
            return Self::finish(normalized, true);
        }
        let combined = if let [first, second] = normalized.as_slice() {
            let first_factors = Self::like_factors(first);
            let second_factors = Self::like_factors(second);
            // Equal non-literal factor slices collect directly without a grouping table.
            if first_factors == second_factors
                && first
                    .literal()
                    .is_none_or(|value| !value.value().is_finite())
                && second
                    .literal()
                    .is_none_or(|value| !value.value().is_finite())
            {
                let unit = NumericValue::new(Number::one());
                let mut coefficients = [
                    Self::coefficient_parts(first).0.unwrap_or(&unit),
                    Self::coefficient_parts(second).0.unwrap_or(&unit),
                ];
                coefficients.sort_unstable_by(|left, right| left.compare(right));
                if let Some(value) = Self::finite_fold(coefficients.into_iter(), true) {
                    let mut collected = Vec::with_capacity(first_factors.len().saturating_add(1));
                    collected.push(Expr::number(value));
                    collected.extend_from_slice(first_factors);
                    return Self::normalize_product(collected);
                }
            }
            false
        } else {
            Self::collect_like_terms(&mut normalized)
        };
        if normalized.len() > 1 {
            normalized.retain(|term| !term.is_exact_zero());
        }
        if !matches!(status, MonomialStatus::NoDistinctPowers) || combined {
            normalized = Self::collect_polynomials(normalized);
        }
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
                    let (literals, rest) = Self::split_literals(&mut flat);
                    // A finite scalar prefix and one quotient already have
                    // canonical coefficient placement.
                    let weighted_quotient = matches!(rest, [quotient] if matches!(quotient.node(), Self::Div(..)))
                        && literals.iter().all(|factor| {
                            factor
                                .literal()
                                .is_some_and(|value| value.value().is_finite())
                        });
                    if weighted_quotient && let [coefficient, quotient] = flat.as_slice() {
                        break 'product if coefficient.is_exact_one() {
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
                    normalized.retain(|factor| !factor.is_exact_one());
                }
                if rebuild {
                    factors = normalized;
                    continue 'normalize;
                }
                Self::sort_operands(Self::split_literals(&mut normalized).0);
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
            && inputs.iter().any(|input| {
                matches!(
                    (input.node(), sum),
                    (Self::Sum(_), true) | (Self::Product(_), false)
                )
            })
        {
            let merged = Self::merge_operands(
                Self::operands(left, sum).iter(),
                Self::operands(right, sum).iter(),
            );
            return if sum {
                Self::unpack_polynomials(merged)
            } else {
                merged
            };
        }
        if !inputs.iter().any(|input| {
            matches!(
                (input.node(), sum),
                (Self::Sum(_) | Self::Poly(_), true) | (Self::Product(_), false)
            )
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
                result.extend(polynomial.projected().iter().cloned());
            } else {
                result.push(term);
            }
        }
        Self::sort_operands(&mut result);
        result
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

    /// Merge two operand sequences, each in structural order, into one.
    pub fn merge_operands<'operand>(
        left: impl Iterator<Item = &'operand Expr>,
        right: impl Iterator<Item = &'operand Expr>,
    ) -> Vec<Expr> {
        // The sequences read slices of pointer-sized handles, which bound
        // their lengths; two of them cannot reach `usize::MAX` together.
        let mut merged = Vec::with_capacity(
            left.size_hint()
                .1
                .unwrap_or_default()
                .saturating_add(right.size_hint().1.unwrap_or_default()),
        );
        let mut first = left.peekable();
        let mut second = right.peekable();
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

    /// Fold finite prefix literals in place, folding as far as checked
    /// arithmetic permits while retaining singleton and unfolded literals.
    fn fold_number_prefix(inputs: &mut Vec<Expr>, sum: bool) {
        let (literals, _) = Self::split_literals(inputs);
        let prefix = literals.len();
        let finite_count = literals
            .iter()
            .filter(|input| {
                input
                    .literal()
                    .is_some_and(|value| value.value().is_finite())
            })
            .count();
        if finite_count < 2 {
            return;
        }
        if let Some(combined) = Self::finite_fold(
            literals
                .iter()
                .filter_map(|input| input.literal().filter(|value| value.value().is_finite())),
            sum,
        ) {
            if finite_count == prefix {
                if let Some(first) = literals.first_mut() {
                    *first = Expr::number(combined);
                }
                inputs.drain(1..prefix);
                return;
            }
            let mut folded = Some(combined);
            let write = Self::compact_finite_prefix(literals, || folded.take());
            inputs.drain(write..prefix);
            return;
        }
        let buckets = Self::fold_partial_literals(
            literals
                .iter()
                .filter_map(|input| input.literal().filter(|value| value.value().is_finite())),
            sum,
        );
        if buckets.len() == finite_count {
            return;
        }
        let mut replacements = buckets.into_iter();
        let write = Self::compact_finite_prefix(literals, || replacements.next());
        inputs.drain(write..prefix);
    }

    fn compact_finite_prefix(
        literals: &mut [Expr],
        mut next_replacement: impl FnMut() -> Option<Number>,
    ) -> usize {
        let mut write = 0_usize;
        for read in 0..literals.len() {
            // SAFETY: `read < literals.len()`.
            let keep = unsafe { literals.get_unchecked(read) }
                .literal()
                .is_none_or(|value| !value.value().is_finite());
            if keep {
                if write != read {
                    literals.swap(write, read);
                }
                // `write <= read < literals.len() <= usize::MAX`.
                write = write.saturating_add(1);
            } else if let Some(value) = next_replacement() {
                // SAFETY: `write <= read < literals.len()`.
                *unsafe { literals.get_unchecked_mut(write) } = Expr::number(value);
                write = write.saturating_add(1);
            }
        }
        // SAFETY: `write` advances at most once per iteration, so `write <= literals.len()`.
        Self::sort_operands(unsafe { literals.get_unchecked_mut(..write) });
        write
    }

    /// Split operands after their leading literals. Node rank places every
    /// Number, followed by central i, before all symbolic kinds; finiteness is
    /// checked by the callers, not assumed.
    fn split_literals(inputs: &mut [Expr]) -> (&mut [Expr], &mut [Expr]) {
        let prefix = inputs
            .iter()
            .take_while(|input| matches!(input.node(), Self::Number(_) | Self::ImaginaryUnit))
            .count();
        // SAFETY: a `take_while` count over `inputs` is at most its length.
        unsafe { inputs.split_at_mut_unchecked(prefix) }
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

    pub fn finish(mut operands: Vec<Expr>, sum: bool) -> Expr {
        if operands.len() <= 1 {
            return operands
                .pop()
                .unwrap_or_else(|| Expr::number(if sum { Number::zero() } else { Number::one() }));
        }
        Expr::intern(if sum {
            Self::Sum(operands)
        } else {
            Self::Product(operands)
        })
    }

    pub fn finite_fold<'value>(
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

    /// Fold sorted finite literals greedily to a fixed point so partial sums
    /// and products remain invariant under operand permutations and combine
    /// cancellable extreme values without overflow or underflow.
    pub fn fold_partial_literals<'value>(
        values: impl Iterator<Item = &'value NumericValue>,
        sum: bool,
    ) -> Vec<Number> {
        let mut buckets: Vec<NumericValue> = Vec::new();
        for value in values {
            let mut pending = Some(value.clone());
            while let Some(candidate) = pending.take() {
                let mut combined = None;
                for (index, existing) in buckets.iter().enumerate() {
                    let result = if sum {
                        existing.add(&candidate)
                    } else {
                        existing.mul(&candidate)
                    };
                    if let Some(folded) = result {
                        combined = Some((index, NumericValue::new(folded)));
                        break;
                    }
                }
                if let Some((index, folded)) = combined {
                    buckets.swap_remove(index);
                    pending = Some(folded);
                } else {
                    buckets.push(candidate);
                }
            }
        }
        buckets.sort_unstable_by(NumericValue::compare);
        buckets.into_iter().map(NumericValue::into_value).collect()
    }
}
