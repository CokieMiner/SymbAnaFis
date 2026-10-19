//! Absolute values and signs with explicit scalar-domain predicates.

use core::ops::Mul;

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn algebraic_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let real = Self::capture("r", Property::Real);
        let nonzero = Self::capture("n", Property::Nonzero);
        vec![
            (a.abs().abs(), a.abs()),
            (a.pow(2).abs(), a.abs().pow(2)),
            (real.abs().pow(2), real.pow(2)),
            (real.signum().signum(), real.signum()),
            (nonzero.abs().signum(), Pattern::exact(1)),
            (Mul::mul(real.abs(), real.signum()), real),
        ]
    }
}
