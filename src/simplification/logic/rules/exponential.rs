//! Exponential products and principal logarithms on proved domains.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn exponential_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let b = Self::capture("b", Property::Finite);
        let real = Self::capture("r", Property::Real);
        let nonzero = Self::capture("n", Property::Nonzero);
        let positive = Self::capture("p", Property::Positive);
        let other = Self::capture("q", Property::Positive);
        let base = Self::conditional("base", |node| {
            Self::proved(node, Property::Positive)
                && Self::proved(&Sub::sub(node, 1), Property::Nonzero)
        });
        vec![
            (nonzero.ln().exp(), nonzero.clone()),
            (real.exp().ln(), real.clone()),
            (Mul::mul(&a, nonzero.ln()).exp(), nonzero.pow(&a)),
            (Mul::mul(a.exp(), b.exp()), Add::add(&a, &b).exp()),
            (Div::div(a.exp(), b.exp()), Sub::sub(&a, &b).exp()),
            (positive.pow(&real).ln(), Mul::mul(&real, positive.ln())),
            (positive.pow(&real).log2(), Mul::mul(&real, positive.log2())),
            (
                positive.pow(&real).log10(),
                Mul::mul(&real, positive.log10()),
            ),
            (
                Add::add(positive.ln(), other.ln()),
                Mul::mul(&positive, &other).ln(),
            ),
            (
                Sub::sub(positive.ln(), other.ln()),
                Div::div(&positive, &other).ln(),
            ),
            (
                Add::add(positive.log2(), other.log2()),
                Mul::mul(&positive, &other).log2(),
            ),
            (
                Add::add(positive.log10(), other.log10()),
                Mul::mul(&positive, &other).log10(),
            ),
            (base.log(&base), Pattern::exact(1)),
        ]
    }
}
