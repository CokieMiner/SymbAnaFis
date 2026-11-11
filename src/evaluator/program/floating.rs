//! Scalar floating power chains and sparse polynomial evaluation.
//!
//! Horner and Estrin forms preserve the polynomial identity. Their floating
//! rounding and intermediate ranges can differ from a direct monomial sum.

use core::{num::NonZeroU64, slice::from_ref};

use alloc::vec::Vec;

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{ExactIdentity, Expr, ExprView, Scalar, StoredForm},
    errors::EvalError,
};

use super::{Compilation, Operation};

struct Monomial {
    degree: u64,
    coefficient: Option<usize>,
}

impl<Value: Scalar> Compilation<Value> {
    pub fn polynomial(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Expr, usize>,
    ) -> Result<Option<usize>, EvalError> {
        let mut base = None;
        let mut monomials = Vec::new();
        let inputs = expression
            .stored_terms()
            .unwrap_or_else(|| from_ref(expression));
        for term in inputs {
            if let Some(stored) = term.monomials() {
                for (coefficient, mut powers) in stored {
                    let (Some((candidate, degree)), None) = (powers.next(), powers.next()) else {
                        return Ok(None);
                    };
                    if base.as_ref().is_some_and(|current| current != candidate) {
                        return Ok(None);
                    }
                    base = Some(candidate.clone());
                    // A unit coefficient reads the shared unit, which Horner
                    // and Estrin recognize to skip the multiplication.
                    let register = if coefficient.as_number().is_some_and(Number::is_one) {
                        self.coefficient(None)?
                    } else {
                        values[coefficient]
                    };
                    monomials.push(Monomial {
                        degree: u64::from(degree),
                        coefficient: Some(register),
                    });
                }
            } else {
                let Some((candidate, monomial)) = Self::monomial(term, values) else {
                    return Ok(None);
                };
                if let Some(candidate_base) = candidate {
                    if base
                        .as_ref()
                        .is_some_and(|current| current != &candidate_base)
                    {
                        return Ok(None);
                    }
                    base = Some(candidate_base);
                }
                monomials.push(monomial);
            }
        }
        if monomials.len() < 2 {
            return Ok(None);
        }
        let Some(polynomial_base) = base else {
            return Ok(None);
        };
        monomials.sort_unstable_by_key(|term| term.degree);
        if monomials
            .windows(2)
            .any(|pair| matches!(pair, [left, right] if left.degree == right.degree))
        {
            return Ok(None);
        }
        let maximum = monomials.last().ok_or(EvalError::InvalidProgram)?.degree;
        let base_register = values[&polynomial_base];
        if (4..=32).contains(&maximum)
            && u64::try_from(monomials.len()).is_ok_and(|count| {
                count.saturating_mul(4) >= maximum.saturating_add(1).saturating_mul(3)
            })
        {
            self.estrin(base_register, &monomials).map(Some)
        } else {
            self.horner(base_register, monomials).map(Some)
        }
    }

    /// Lower compact storage as the sum of its monomials in stored order.
    ///
    /// Generic scalars multiply each coefficient by its powers in stored
    /// order. Native floating scalars multiply the powers first: stored terms
    /// are sorted by their powers, so a term reuses the product of the longest
    /// power prefix it shares with the previous term, and the products of all
    /// terms form a trie. The coefficient multiplies the complete product.
    pub fn compact_polynomial(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Expr, usize>,
    ) -> Result<usize, EvalError> {
        let terms = expression.monomials().ok_or(EvalError::InvalidProgram)?;
        let mut monomials = Vec::with_capacity(terms.len());
        let mut factors = Vec::new();
        let mut prefix: Vec<(&Expr, u32, usize)> = Vec::new();
        for (coefficient, powers) in terms {
            if !Value::OPTIMIZE_FLOATS {
                factors.clear();
                if !coefficient.is_exact_one() {
                    factors.push(values[coefficient]);
                }
                for (base, count) in powers {
                    factors.push(self.stored_power(values[base], count)?);
                }
                monomials.push(self.group(factors.iter().copied(), true)?);
                continue;
            }
            let length = powers.len();
            for (depth, (base, count)) in powers.enumerate() {
                // `prefix` holds the products of this term's first `depth`
                // powers, followed by entries of the previous term.
                if prefix
                    .get(depth)
                    .is_none_or(|&(shared, exponent, _)| shared != base || exponent != count)
                {
                    prefix.truncate(depth);
                    let power = self.stored_power(values[base], count)?;
                    let product = prefix.last().map_or(power, |&(.., previous)| {
                        self.emit(Operation::Multiply([previous, power]))
                    });
                    prefix.push((base, count, product));
                }
            }
            prefix.truncate(length);
            monomials.push(match prefix.last() {
                Some(&(.., product)) if coefficient.is_exact_one() => product,
                Some(&(.., product)) => {
                    self.emit(Operation::Multiply([values[coefficient], product]))
                }
                None => values[coefficient],
            });
        }
        self.group(monomials, false)
    }

    pub fn stored_power(&mut self, base: usize, count: u32) -> Result<usize, EvalError> {
        if count == 1 {
            return Ok(base);
        }
        if let Some(result) = self.powers.get(&(base, u64::from(count))) {
            return Ok(*result);
        }
        if Value::OPTIMIZE_FLOATS {
            let positive = NonZeroU64::new(u64::from(count)).ok_or(EvalError::InvalidProgram)?;
            return Ok(self.integer_power(base, positive));
        }
        let number = Number::from_int(i64::from(count));
        let value =
            Value::literal(&number).ok_or(EvalError::UnsupportedLiteral { value: number })?;
        let source = self.constant(value);
        let exponent = self.sources.len();
        self.sources.push(source);
        let result = self.emit(Operation::Power([base, exponent]));
        self.powers.insert((base, u64::from(count)), result);
        Ok(result)
    }

    pub fn integer_power(&mut self, base: usize, count: NonZeroU64) -> usize {
        if count.get() == 1 {
            return base;
        }
        if let Some(result) = self.powers.get(&(base, count.get())) {
            return *result;
        }
        let mut result = base;
        let mut prefix = 1_u64;
        for bit in (0..count.ilog2()).rev() {
            // Each binary prefix is at most the complete positive exponent.
            prefix = prefix.saturating_mul(2);
            result = self
                .powers
                .get(&(base, prefix))
                .copied()
                .unwrap_or_else(|| {
                    let square = self.emit(Operation::Multiply([result, result]));
                    self.powers.insert((base, prefix), square);
                    square
                });
            if count.get() & (1_u64 << bit) != 0 {
                prefix = prefix.saturating_add(1);
                result = self
                    .powers
                    .get(&(base, prefix))
                    .copied()
                    .unwrap_or_else(|| {
                        let product = self.emit(Operation::Multiply([result, base]));
                        self.powers.insert((base, prefix), product);
                        product
                    });
            }
        }
        result
    }

    fn horner(&mut self, base: usize, mut terms: Vec<Monomial>) -> Result<usize, EvalError> {
        let highest = terms.pop().ok_or(EvalError::InvalidProgram)?;
        let mut degree = highest.degree;
        let mut result = self.coefficient(highest.coefficient)?;
        while let Some(term) = terms.pop() {
            let gap = degree
                .checked_sub(term.degree)
                .and_then(NonZeroU64::new)
                .ok_or(EvalError::InvalidProgram)?;
            let power = self.integer_power(base, gap);
            let coefficient = self.coefficient(term.coefficient)?;
            result = self.multiply_add(result, power, coefficient);
            degree = term.degree;
        }
        if let Some(count) = NonZeroU64::new(degree) {
            let power = self.integer_power(base, count);
            result = if self.unit == Some(result) {
                power
            } else {
                self.emit(Operation::Multiply([result, power]))
            };
        }
        Ok(result)
    }

    /// Evaluate dense `terms`, sorted by strictly increasing degree, by
    /// Estrin's scheme: each level pairs adjacent coefficients \(c_{2i} +
    /// c_{2i+1} x^{2^k}\) and squares the power of the base.
    ///
    /// Reference: G. Estrin, "Organization of computer systems: the fixed plus
    /// variable structure computer", Proc. Western Joint Computer Conference
    /// (1960), 33-40.
    fn estrin(&mut self, base: usize, terms: &[Monomial]) -> Result<usize, EvalError> {
        let mut coefficients = Vec::new();
        for term in terms {
            let degree =
                usize::try_from(term.degree).map_err(|_overflow| EvalError::ProgramTooLarge)?;
            // Degrees increase strictly, so padding to the degree places the
            // coefficient at its position.
            coefficients.resize(degree, None);
            coefficients.push(Some(self.coefficient(term.coefficient)?));
        }
        let mut power = base;
        while coefficients.len() > 1 {
            let (pairs, rest) = coefficients.as_chunks::<2>();
            let mut level = Vec::with_capacity(coefficients.len().div_ceil(2));
            for pair in pairs {
                level.push(match *pair {
                    [Some(value), Some(factor)] => Some(self.multiply_add(factor, power, value)),
                    [None, Some(factor)] => Some(if self.unit == Some(factor) {
                        power
                    } else {
                        self.emit(Operation::Multiply([factor, power]))
                    }),
                    [value, None] => value,
                });
            }
            level.extend_from_slice(rest);
            coefficients = level;
            if coefficients.len() > 1 {
                power = self.emit(Operation::Multiply([power, power]));
            }
        }
        coefficients
            .into_iter()
            .next()
            .flatten()
            .ok_or(EvalError::InvalidProgram)
    }

    /// The source of `coefficient`, or of the shared unit constant when it is
    /// `None`; the unit is created on first use.
    ///
    /// # Errors
    /// Returns [`EvalError::UnsupportedLiteral`] if the scalar cannot represent one.
    pub fn coefficient(&mut self, coefficient: Option<usize>) -> Result<usize, EvalError> {
        if let Some(value) = coefficient.or(self.unit) {
            return Ok(value);
        }
        let one = Number::one();
        let value = Value::literal(&one).ok_or(EvalError::UnsupportedLiteral { value: one })?;
        let source = self.constant(value);
        let output = self.sources.len();
        self.sources.push(source);
        self.unit = Some(output);
        Ok(output)
    }

    fn multiply_add(&mut self, left: usize, right: usize, addend: usize) -> usize {
        if self.unit == Some(left) {
            return self.emit(Operation::Add([right, addend]));
        }
        if Value::FUSE {
            self.emit(Operation::Fused([left, right, addend]))
        } else {
            let product = self.emit(Operation::Multiply([left, right]));
            self.emit(Operation::Add([product, addend]))
        }
    }

    fn monomial(term: &Expr, values: &FixedMap<Expr, usize>) -> Option<(Option<Expr>, Monomial)> {
        if let Some(number) = term.as_number() {
            return number.is_finite().then(|| {
                (
                    None,
                    Monomial {
                        degree: 0,
                        coefficient: Some(values[term]),
                    },
                )
            });
        }
        let (factor, coefficient) = if let ExprView::Product(factors) = term.view() {
            let [coefficient, factor] = factors else {
                return None;
            };
            if !coefficient.as_number()?.is_finite() {
                return None;
            }
            (factor, Some(values[coefficient]))
        } else {
            (term, None)
        };
        if let ExprView::Pow(base, exponent) = factor.view() {
            Some((
                Some(base.clone()),
                Monomial {
                    degree: exponent.positive_integer()?.get(),
                    coefficient,
                },
            ))
        } else {
            Some((
                Some(factor.clone()),
                Monomial {
                    degree: 1,
                    coefficient,
                },
            ))
        }
    }
}
