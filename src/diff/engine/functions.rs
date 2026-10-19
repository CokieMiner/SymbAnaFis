//! Builtin dispatch and sparse custom-function chain rules.

use core::ops::Mul;

use alloc::vec::Vec;

use crate::core::{Expr, Function, FunctionDefinition};

use super::Differentiation;

impl Differentiation<'_> {
    /// Apply \(D f(u_{0},\ldots,u_{n})=\sum_{i} \partial_{i} f(u) D u_{i}\).
    /// Missing active custom partials preserve the whole derivative. Validation
    /// precedes callback execution, and a zero argument derivative needs no partial.
    pub fn function(function: &Function, args: &[Expr], primes: &[Expr]) -> Option<Expr> {
        if primes.iter().all(Expr::is_zero_num) {
            return Some(Expr::number(0));
        }
        if let Some(custom) = function.definition() {
            let active = primes
                .iter()
                .enumerate()
                .filter(|(_, prime)| !prime.is_zero_num());
            let callbacks = active
                .map(|(index, prime)| custom.partial_fn(index).map(|partial| (partial, prime)))
                .collect::<Option<Vec<_>>>()?;
            let terms = callbacks
                .into_iter()
                .filter_map(|(partial, prime)| {
                    let value = partial(args);
                    (!value.is_zero_num()).then(|| Mul::mul(value, prime))
                })
                .collect();
            return Some(Expr::sum(terms));
        }
        let builtin = FunctionDefinition::get(function)?.builtin();
        if let ([argument], [prime]) = (args, primes)
            && let Some(slope) = Self::elementary(builtin, argument)
        {
            return Some(Mul::mul(slope, prime));
        }
        Self::special(builtin, args, primes)
    }
}
