//! Iterative primitive polynomial remainder sequences over exact rationals.

use core::ops::Div;

use alloc::vec::{IntoIter, Vec};

use crate::{
    core::{Expr, ExprView, Property},
    errors::SimplifyError,
};

use super::{DomainSafety, PolynomialInput, PolynomialWork, PreparedStep, SparsePolynomial};

struct PolynomialGcd {
    pending: Vec<Continuation>,
}

/// A completed child is carried by `Return`, so a continuation cannot consume
/// an absent result or confuse results belonging to different children.
enum Evaluation {
    Gcd {
        left: SparsePolynomial,
        right: SparsePolynomial,
        variable: usize,
    },
    Content {
        value: SparsePolynomial,
        variable: usize,
    },
    Remainder {
        left: SparsePolynomial,
        right: SparsePolynomial,
        content: SparsePolynomial,
        variable: usize,
    },
    Return(SparsePolynomial),
}

enum Continuation {
    FoldContent {
        remaining: IntoIter<SparsePolynomial>,
        variable: usize,
    },
    LeftContent {
        left: SparsePolynomial,
        right: SparsePolynomial,
        variable: usize,
    },
    RightContent {
        left: SparsePolynomial,
        right: SparsePolynomial,
        content: SparsePolynomial,
        variable: usize,
    },
    Primitive {
        left: SparsePolynomial,
        right: SparsePolynomial,
        variable: usize,
    },
    RemainderContent {
        divisor: SparsePolynomial,
        remainder: SparsePolynomial,
        content: SparsePolynomial,
        variable: usize,
    },
}

impl PolynomialGcd {
    /// Gauss's lemma reduces multivariate GCD to contents and primitive
    /// pseudo-remainder sequences in successive variables. Explicit continuations
    /// keep the native stack independent of the expression and variable counts.
    fn compute(
        left: &SparsePolynomial,
        right: &SparsePolynomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<SparsePolynomial>, SimplifyError> {
        let mut engine = Self {
            pending: Vec::new(),
        };
        engine.run(
            Evaluation::Gcd {
                left: left.clone(),
                right: right.clone(),
                variable: 0,
            },
            work,
        )
    }

    fn run(
        &mut self,
        mut active: Evaluation,
        work: &mut PolynomialWork,
    ) -> Result<Option<SparsePolynomial>, SimplifyError> {
        loop {
            work.spend(1)?;
            active = match active {
                Evaluation::Gcd {
                    left,
                    right,
                    variable,
                } => self.start(left, right, variable, work)?,
                Evaluation::Content { value, variable } => {
                    let mut coefficients = value.coefficients(variable, work)?.into_iter();
                    let Some(first) = coefficients.next() else {
                        // The coefficient content of the zero polynomial is zero.
                        active = Evaluation::Return(value);
                        continue;
                    };
                    self.pending.push(Continuation::FoldContent {
                        remaining: coefficients,
                        // Content is requested only below `dimensions`; a Vec
                        // length is at most isize::MAX, so this addition fits.
                        variable: variable.saturating_add(1),
                    });
                    Evaluation::Return(first.monic(work)?)
                }
                Evaluation::Remainder {
                    left,
                    right,
                    content,
                    variable,
                } => {
                    if right.is_zero() {
                        Evaluation::Return(left.mul(&content, work)?.monic(work)?)
                    } else {
                        let remainder = left.pseudo_remainder(&right, variable, work)?;
                        if remainder.is_zero() {
                            Evaluation::Return(right.mul(&content, work)?.monic(work)?)
                        } else {
                            self.pending.push(Continuation::RemainderContent {
                                divisor: right,
                                remainder: remainder.clone(),
                                content,
                                variable,
                            });
                            Evaluation::Content {
                                value: remainder,
                                variable,
                            }
                        }
                    }
                }
                Evaluation::Return(value) => {
                    let Some(continuation) = self.pending.pop() else {
                        return Ok(Some(value));
                    };
                    let Some(next) = self.resume(continuation, value, work)? else {
                        return Ok(None);
                    };
                    next
                }
            };
        }
    }

    fn start(
        &mut self,
        left: SparsePolynomial,
        right: SparsePolynomial,
        variable: usize,
        work: &mut PolynomialWork,
    ) -> Result<Evaluation, SimplifyError> {
        if right.is_zero() || left == right {
            Ok(Evaluation::Return(left.monic(work)?))
        } else if left.is_zero() {
            Ok(Evaluation::Return(right.monic(work)?))
        } else if variable >= left.dimensions() || left.is_constant() || right.is_constant() {
            Ok(Evaluation::Return(SparsePolynomial::one(
                left.dimensions(),
                work,
            )?))
        } else {
            self.pending.push(Continuation::LeftContent {
                left: left.clone(),
                right,
                variable,
            });
            Ok(Evaluation::Content {
                value: left,
                variable,
            })
        }
    }

    /// Resume exactly one completed child. Exact division certifies each
    /// content removal; a failed certificate discards the whole GCD candidate.
    fn resume(
        &mut self,
        continuation: Continuation,
        value: SparsePolynomial,
        work: &mut PolynomialWork,
    ) -> Result<Option<Evaluation>, SimplifyError> {
        let next = match continuation {
            Continuation::FoldContent {
                mut remaining,
                variable,
            } => {
                if value.is_constant() {
                    Evaluation::Return(value)
                } else if let Some(next) = remaining.next() {
                    self.pending.push(Continuation::FoldContent {
                        remaining,
                        variable,
                    });
                    Evaluation::Gcd {
                        left: value,
                        right: next,
                        variable,
                    }
                } else {
                    Evaluation::Return(value)
                }
            }
            Continuation::LeftContent {
                left,
                right,
                variable,
            } => {
                let Some(primitive) = left.exact_div(&value, work)? else {
                    return Ok(None);
                };
                self.pending.push(Continuation::RightContent {
                    left: primitive,
                    right: right.clone(),
                    content: value,
                    variable,
                });
                Evaluation::Content {
                    value: right,
                    variable,
                }
            }
            Continuation::RightContent {
                left,
                right,
                content,
                variable,
            } => {
                let Some(primitive) = right.exact_div(&value, work)? else {
                    return Ok(None);
                };
                self.pending.push(Continuation::Primitive {
                    left,
                    right: primitive,
                    variable,
                });
                Evaluation::Gcd {
                    left: content,
                    right: value,
                    variable: variable.saturating_add(1),
                }
            }
            Continuation::Primitive {
                left,
                right,
                variable,
            } => Evaluation::Remainder {
                left,
                right,
                content: value,
                variable,
            },
            Continuation::RemainderContent {
                divisor,
                remainder,
                content,
                variable,
            } => {
                let Some(primitive) = remainder.exact_div(&value, work)? else {
                    return Ok(None);
                };
                Evaluation::Remainder {
                    left: divisor,
                    right: primitive,
                    content,
                    variable,
                }
            }
        };
        Ok(Some(next))
    }
}

impl PreparedStep<'_> {
    /// Cancel a verified exact polynomial GCD. Preserving mode additionally
    /// proves that the removed expression is nonzero, retaining every pole.
    pub fn polynomial_cancellation(
        expression: &Expr,
        safety: DomainSafety,
        work: &mut PolynomialWork,
    ) -> Result<Option<Expr>, SimplifyError> {
        let ExprView::Div(numerator, denominator) = expression.view() else {
            return Ok(None);
        };
        if !Self::polynomial_sum(numerator) && !Self::polynomial_sum(denominator) {
            return Ok(None);
        }
        if !Self::permitted(numerator, Property::Finite, safety)
            || !Self::permitted(denominator, Property::Finite, safety)
        {
            return Ok(None);
        }
        let Some((top, bottom, bases)) = PolynomialInput::pair(numerator, denominator, work)?
        else {
            return Ok(None);
        };
        if top.is_zero() || bottom.is_zero() || top.is_constant() || bottom.is_constant() {
            return Ok(None);
        }
        let Some(candidate) = PolynomialGcd::compute(&top, &bottom, work)? else {
            return Ok(None);
        };
        let common = candidate.primitive_integer(work)?;
        if common.is_constant() {
            return Ok(None);
        }
        if !Self::permitted(&common.to_expr(&bases), Property::Nonzero, safety) {
            return Ok(None);
        }
        let Some(reduced_top) = top.exact_div(&common, work)? else {
            return Ok(None);
        };
        let Some(reduced_bottom) = bottom.exact_div(&common, work)? else {
            return Ok(None);
        };
        let output = Div::div(reduced_top.to_expr(&bases), reduced_bottom.to_expr(&bases));
        Ok((output != *expression).then_some(output))
    }

    fn polynomial_sum(expression: &Expr) -> bool {
        match expression.view() {
            ExprView::Sum(_) => true,
            ExprView::Pow(base, _) => matches!(base.view(), ExprView::Sum(_)),
            ExprView::Product(factors) => factors
                .iter()
                .any(|factor| matches!(factor.view(), ExprView::Sum(_))),
            ExprView::Number(_)
            | ExprView::ImaginaryUnit
            | ExprView::Pi
            | ExprView::Euler
            | ExprView::Symbol(_)
            | ExprView::Function { .. }
            | ExprView::Div(..)
            | ExprView::Derivative { .. } => false,
        }
    }
}
