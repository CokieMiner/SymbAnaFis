//! Scalar floating power chains and sparse polynomial evaluation.
//!
//! Horner and Estrin forms preserve the polynomial identity. Their floating
//! rounding and intermediate ranges can differ from a direct monomial sum.

use core::{num::NonZeroU64, slice::from_ref};

use alloc::{vec, vec::Vec};

use num_anafis::Number;

use crate::{
    collections::HashMap,
    core::{EvaluationView, Expr, ExprView, Scalar},
    errors::EvalError,
};

use super::{Compilation, Operation, Source};

struct Monomial {
    degree: u64,
    coefficient: Option<usize>,
}

impl<Value: Scalar> Compilation<Value> {
    pub fn polynomial(
        &mut self,
        expression: &Expr,
        values: &HashMap<Expr, usize>,
    ) -> Result<Option<usize>, EvalError> {
        let mut base = None;
        let mut monomials = Vec::new();
        let inputs = EvaluationView::new(expression)
            .sum()
            .unwrap_or_else(|| from_ref(expression));
        for term in inputs {
            if let Some(stored) = EvaluationView::new(term).monomials() {
                for (coefficient, powers) in stored {
                    let [(candidate, degree)] = powers else {
                        return Ok(None);
                    };
                    if base.as_ref().is_some_and(|current| current != candidate) {
                        return Ok(None);
                    }
                    base = Some(candidate.clone());
                    if coefficient.as_number().is_some_and(Number::is_one) && self.unit.is_none() {
                        self.unit = Some(values[coefficient]);
                    }
                    monomials.push(Monomial {
                        degree: u64::from(*degree),
                        coefficient: Some(values[coefficient]),
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
        let maximum = monomials.last().expect("at least two monomials").degree;
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

    pub fn compact_polynomial(
        &mut self,
        expression: &Expr,
        values: &HashMap<Expr, usize>,
    ) -> Result<usize, EvalError> {
        let terms = EvaluationView::new(expression)
            .monomials()
            .expect("the compiler selects compact polynomial storage before this kernel");
        let mut monomials = Vec::new();
        for (coefficient, powers) in terms {
            let mut product = if EvaluationView::exact_one(coefficient) {
                None
            } else {
                Some(values[coefficient])
            };
            for (base, count) in powers {
                let factor = self.stored_power(values[base], *count)?;
                product = Some(product.map_or(factor, |current| {
                    self.emit(Operation::Multiply([current, factor]))
                }));
            }
            monomials.push(product.expect("stored monomials contain at least one positive power"));
        }
        let operation = self.group(monomials.into_iter(), false);
        Ok(self.emit(operation))
    }

    pub fn stored_power(&mut self, base: usize, count: u32) -> Result<usize, EvalError> {
        if count == 1 {
            return Ok(base);
        }
        if let Some(result) = self.powers.get(&(base, u64::from(count))) {
            return Ok(*result);
        }
        if Value::OPTIMIZE_FLOATS {
            let positive =
                NonZeroU64::new(u64::from(count)).expect("stored powers are strictly positive");
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
        let highest = terms
            .pop()
            .expect("polynomial selection requires at least two terms");
        let mut degree = highest.degree;
        let mut result = self.coefficient(highest.coefficient)?;
        while let Some(term) = terms.pop() {
            let gap = degree
                .checked_sub(term.degree)
                .and_then(NonZeroU64::new)
                .expect("normalized univariate monomials have distinct sorted degrees");
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

    #[expect(
        clippy::indexing_slicing,
        reason = "dense Estrin degrees are at most thirty-two and every coefficient position is bounded by the selected maximum degree"
    )]
    #[expect(
        clippy::unwrap_in_result,
        reason = "polynomial selection proves that the nonempty sorted degrees are at most thirty-two and fit every supported pointer width"
    )]
    fn estrin(&mut self, base: usize, terms: &[Monomial]) -> Result<usize, EvalError> {
        let degree = usize::try_from(terms.last().expect("nonempty polynomial").degree)
            .expect("dense degrees fit every supported pointer width");
        let mut coefficients = vec![None; degree.saturating_add(1)];
        for term in terms {
            let index = usize::try_from(term.degree).expect("dense degrees are at most thirty-two");
            coefficients[index] = Some(self.coefficient(term.coefficient)?);
        }
        let mut power = base;
        while coefficients.len() > 1 {
            let length = coefficients.len();
            for (output, input) in (0..length).step_by(2).enumerate() {
                let left = coefficients[input];
                let right = coefficients.get(input.saturating_add(1)).copied().flatten();
                coefficients[output] = match (left, right) {
                    (Some(value), Some(factor)) => Some(self.multiply_add(factor, power, value)),
                    (None, Some(factor)) => Some(if self.unit == Some(factor) {
                        power
                    } else {
                        self.emit(Operation::Multiply([factor, power]))
                    }),
                    (value, None) => value,
                };
            }
            coefficients.truncate(length.div_ceil(2));
            if coefficients.len() > 1 {
                power = self.emit(Operation::Multiply([power, power]));
            }
        }
        Ok(coefficients
            .into_iter()
            .next()
            .flatten()
            .expect("grouping a nonempty polynomial retains a coefficient"))
    }

    fn coefficient(&mut self, coefficient: Option<usize>) -> Result<usize, EvalError> {
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

    fn emit(&mut self, operation: Operation<Value>) -> usize {
        let output = self.sources.len();
        self.sources.push(Source::Computed(operation));
        output
    }

    fn monomial(term: &Expr, values: &HashMap<Expr, usize>) -> Option<(Option<Expr>, Monomial)> {
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
                    degree: EvaluationView::positive_integer(exponent)?.get(),
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
