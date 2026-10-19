//! Principal square-root scaling and real cube-root identities.

use core::ops::{Div, Mul};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn root_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Nonnegative);
        let b = Self::capture("b", Property::Nonnegative);
        let positive = Self::capture("p", Property::Positive);
        let real = Self::capture("r", Property::Real);
        let finite = Self::capture("z", Property::Finite);
        vec![
            (Mul::mul(a.sqrt(), b.sqrt()), Mul::mul(&a, &b).sqrt()),
            (
                Div::div(a.sqrt(), positive.sqrt()),
                Div::div(&a, &positive).sqrt(),
            ),
            (
                Mul::mul(real.pow(2), &finite).sqrt(),
                Mul::mul(real.abs(), finite.sqrt()),
            ),
            (real.pow(3).cbrt(), real),
            (finite.cbrt().pow(3), finite),
        ]
    }
}
