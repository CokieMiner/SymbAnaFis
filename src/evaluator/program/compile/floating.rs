//! Scalar floating power chains and sparse polynomial evaluation.
//!
//! Horner and Estrin forms preserve the polynomial identity. Their floating
//! rounding and intermediate ranges can differ from a direct monomial sum.

use core::{mem::swap, num::NonZeroU64, slice::from_ref};

use alloc::{borrow::Cow, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::FixedMap,
    core::{Expr, ExprView, StoredForm},
    errors::EvalError,
};

use super::{Compilation, EvaluationValue, Fused, Operation, ProgramData, Source};

struct Monomial {
    degree: u64,
    coefficient: Option<u32>,
}

impl<Value: EvaluationValue> Compilation<Value> {
    pub fn polynomial(
        &mut self,
        expression: &Expr,
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Result<Option<u32>, EvalError> {
        let mut base: Option<&Expr> = None;
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
                    if base.is_some_and(|current| current != candidate) {
                        return Ok(None);
                    }
                    base = Some(candidate);
                    let register = if coefficient.is_exact_one() {
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
                    if base.is_some_and(|current| current != candidate_base) {
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
        if maximum < 2 {
            return Ok(None);
        }
        let base_register = values[polynomial_base];
        if (4..=32).contains(&maximum)
            && u64::try_from(monomials.len()).is_ok_and(|count| {
                count.saturating_mul(4) >= maximum.saturating_add(1).saturating_mul(3)
            })
        {
            self.estrin(base_register, &monomials).map(Some)
        } else if inputs.len() == 2 && inputs.iter().any(|term| term.monomials().is_some()) {
            Ok(None)
        } else {
            self.horner(base_register, monomials).map(Some)
        }
    }

    pub fn stored_power(&mut self, base: u32, count: u32) -> Result<u32, EvalError> {
        if count == 1 {
            return Ok(base);
        }
        if let Some(result) = self.powers.get(&(base, u64::from(count))) {
            return Ok(*result);
        }
        if Value::OPTIMIZE_FLOATS {
            let positive = NonZeroU64::new(u64::from(count)).ok_or(EvalError::InvalidProgram)?;
            return self.integer_power(base, positive);
        }
        let exponent = if let Some(&existing) = self.exponent_constants.get(&count) {
            existing
        } else {
            let number = Number::from_int(i64::from(count));
            let value = Value::from_number(&number)
                .ok_or(EvalError::UnsupportedLiteral { value: number })?;
            let created = self.constant(value)?;
            self.exponent_constants.insert(count, created);
            created
        };
        let result = self.push_source(Source::Computed(Operation::Power([base, exponent])))?;
        self.powers.insert((base, u64::from(count)), result);
        Ok(result)
    }

    pub fn integer_power(&mut self, base: u32, count: NonZeroU64) -> Result<u32, EvalError> {
        if count.get() == 1 {
            return Ok(base);
        }
        if let Some(result) = self.powers.get(&(base, count.get())) {
            return Ok(*result);
        }
        let mut result = base;
        let mut prefix = 1_u64;
        for bit in (0..count.ilog2()).rev() {
            prefix = prefix.saturating_mul(2);
            result = if let Some(&existing) = self.powers.get(&(base, prefix)) {
                existing
            } else {
                let square = self.emit_binary(true, result, result)?;
                self.powers.insert((base, prefix), square);
                square
            };
            if count.get() & (1_u64 << bit) != 0 {
                prefix = prefix.saturating_add(1);
                result = if let Some(&existing) = self.powers.get(&(base, prefix)) {
                    existing
                } else {
                    let product = self.emit_binary(true, result, base)?;
                    self.powers.insert((base, prefix), product);
                    product
                };
            }
        }
        Ok(result)
    }

    fn horner(&mut self, base: u32, mut terms: Vec<Monomial>) -> Result<u32, EvalError> {
        let highest = terms.pop().ok_or(EvalError::InvalidProgram)?;
        let mut degree = highest.degree;
        let mut result = self.coefficient(highest.coefficient)?;
        while let Some(term) = terms.pop() {
            let gap = degree
                .checked_sub(term.degree)
                .and_then(NonZeroU64::new)
                .ok_or(EvalError::InvalidProgram)?;
            let power = self.integer_power(base, gap)?;
            let coefficient = self.coefficient(term.coefficient)?;
            result = self.multiply_add(result, power, coefficient)?;
            degree = term.degree;
        }
        if let Some(count) = NonZeroU64::new(degree) {
            let power = self.integer_power(base, count)?;
            result = if self.unit == Some(result) {
                power
            } else {
                self.emit_binary(true, result, power)?
            };
        }
        Ok(result)
    }

    /// Evaluate dense `terms`, sorted by strictly increasing degree, by
    /// Estrin's scheme: each level pairs adjacent coefficients $c_{2i} +
    /// c_{2i+1} x^{2^k}$ and squares the power of the base.
    ///
    /// Reference: G. Estrin, "Organization of computer systems: the fixed plus
    /// variable structure computer", Proc. Western Joint Computer Conference
    /// (1960), 33-40.
    fn estrin(&mut self, base: u32, terms: &[Monomial]) -> Result<u32, EvalError> {
        let capacity = terms
            .last()
            .and_then(|term| usize::try_from(term.degree).ok())
            .map_or(0, |max| max.saturating_add(1));
        let mut coefficients = Vec::with_capacity(capacity);
        let mut level = Vec::with_capacity(capacity.div_ceil(2));
        for term in terms {
            let degree =
                usize::try_from(term.degree).map_err(|_overflow| EvalError::ProgramTooLarge)?;
            coefficients.resize(degree, None);
            coefficients.push(Some(self.coefficient(term.coefficient)?));
        }
        let mut power = base;
        let mut exponent = 1_u64;
        while coefficients.len() > 1 {
            let (pairs, rest) = coefficients.as_chunks::<2>();
            level.clear();
            for pair in pairs {
                level.push(match *pair {
                    [Some(value), Some(factor)] => Some(self.multiply_add(factor, power, value)?),
                    [None, Some(factor)] => Some(if self.unit == Some(factor) {
                        power
                    } else {
                        self.emit_binary(true, factor, power)?
                    }),
                    [value, None] => value,
                });
            }
            level.extend_from_slice(rest);
            swap(&mut coefficients, &mut level);
            if coefficients.len() > 1 {
                exponent = exponent.saturating_mul(2);
                power = if let Some(&existing) = self.powers.get(&(base, exponent)) {
                    existing
                } else {
                    let square = self.emit_binary(true, power, power)?;
                    self.powers.insert((base, exponent), square);
                    square
                };
            }
        }
        coefficients
            .into_iter()
            .next()
            .flatten()
            .ok_or(EvalError::InvalidProgram)
    }

    pub fn coefficient(&mut self, coefficient: Option<u32>) -> Result<u32, EvalError> {
        if let Some(value) = coefficient.or(self.unit) {
            return Ok(value);
        }
        let one = Number::one();
        let value = Value::from_number(&one).ok_or(EvalError::UnsupportedLiteral { value: one })?;
        let output = self.constant(value)?;
        self.unit = Some(output);
        Ok(output)
    }

    fn multiply_add(&mut self, left: u32, right: u32, addend: u32) -> Result<u32, EvalError> {
        if self.unit == Some(left) {
            return self.emit_binary(false, right, addend);
        }
        if ProgramData::<Value>::fuse() {
            self.emit_fused(
                Fused {
                    negate_product: false,
                    negate_addend: false,
                },
                [left, right, addend],
            )
        } else {
            let product = self.emit_binary(true, left, right)?;
            self.emit_binary(false, product, addend)
        }
    }

    fn monomial<'expr>(
        term: &'expr Expr,
        values: &FixedMap<Cow<'_, Expr>, u32>,
    ) -> Option<(Option<&'expr Expr>, Monomial)> {
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
        let (base, degree) = if let ExprView::Pow(base, exponent) = factor.view() {
            (base, exponent.positive_integer()?.get())
        } else {
            (factor, 1)
        };
        Some((
            Some(base),
            Monomial {
                degree,
                coefficient,
            },
        ))
    }
}
