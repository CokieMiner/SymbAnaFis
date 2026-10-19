//! Factored product, quotient, and principal-power derivative kernels.

use core::ops::{Div, Mul, Neg, Sub};

use alloc::{vec, vec::Vec};

use crate::core::Expr;

use super::Differentiation;

impl Differentiation<'_> {
    /// Apply the product rule without dividing by any input factor.
    /// Constant factors occur once outside a balanced derivative expression.
    /// For active groups, \((AB)'=A'B+AB'\) bounds intermediate storage by
    /// \(O(n\log n)\), rather than materializing \(n\) flat products of length \(n\).
    pub fn product(factors: &[Expr], primes: &[Expr]) -> Expr {
        let mut constants = Vec::new();
        let mut active = Vec::new();
        for (factor, prime) in factors.iter().zip(primes) {
            if prime.is_zero_num() {
                constants.push(factor.clone());
            } else {
                active.push((factor.clone(), prime.clone()));
            }
        }
        if active.is_empty() {
            return Expr::number(0);
        }
        while active.len() > 1 {
            let mut next = Vec::new();
            let mut pairs = active.into_iter();
            while let Some((left, left_prime)) = pairs.next() {
                if let Some((right, right_prime)) = pairs.next() {
                    let prime = Expr::sum(vec![
                        Mul::mul(left_prime, &right),
                        Mul::mul(&left, right_prime),
                    ]);
                    next.push((Mul::mul(left, right), prime));
                } else {
                    next.push((left, left_prime));
                }
            }
            active = next;
        }
        if let Some((_, derivative)) = active.pop() {
            constants.push(derivative);
        }
        Expr::product(constants)
    }

    /// Apply \((u/v)'=(u'v-uv')/v^2\) on the quotient's regular domain.
    /// A constant denominator uses \(u'/v\), retaining its original pole.
    pub fn quotient(
        numerator: &Expr,
        denominator: &Expr,
        numerator_prime: Expr,
        denominator_prime: Expr,
    ) -> Expr {
        if numerator_prime.is_zero_num() && denominator_prime.is_zero_num() {
            return Expr::number(0);
        }
        if denominator_prime.is_zero_num() {
            return Div::div(numerator_prime, denominator);
        }
        let negative = Neg::neg(Mul::mul(numerator, denominator_prime));
        let top = if numerator_prime.is_zero_num() {
            negative
        } else {
            Expr::sum(vec![Mul::mul(numerator_prime, denominator), negative])
        };
        Div::div(top, denominator.pow(2))
    }

    /// Apply \((u^v)'=v u^{v-1}u'+u^v\log(u)v'\) on regular principal branches.
    /// Constant exponents omit the logarithm; exponent one omits the zero power.
    pub fn power(
        base: &Expr,
        exponent: &Expr,
        base_prime: Expr,
        exponent_prime: Expr,
    ) -> Option<Expr> {
        if base_prime.is_zero_num() && exponent_prime.is_zero_num() {
            return Some(Expr::number(0));
        }
        if base.is_zero_num() && !exponent_prime.is_zero_num() {
            return None;
        }
        let mut terms = Vec::new();
        if !base_prime.is_zero_num() && !exponent.is_zero_num() {
            terms.push(if exponent.is_one_num() {
                base_prime
            } else {
                Expr::product(vec![
                    exponent.clone(),
                    base.pow(Sub::sub(exponent, 1)),
                    base_prime,
                ])
            });
        }
        if !exponent_prime.is_zero_num() {
            terms.push(Expr::product(vec![
                base.pow(exponent),
                base.ln(),
                exponent_prime,
            ]));
        }
        Some(Expr::sum(terms))
    }
}
