//! Native evaluation forms retaining the original principal domains.

use core::ops::{Add, Div, Neg, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn compilation_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let real = Self::capture("r", Property::Real);
        let nonzero = Self::capture("n", Property::Nonzero);
        let log_domain = Self::conditional("d", |node| {
            Self::proved(node, Property::Finite)
                && Self::proved(&Add::add(node, 1), Property::Nonzero)
        });
        vec![
            (Sub::sub(a.exp(), 1), a.exp_m1()),
            (Sub::sub(1_i32, a.exp()), Neg::neg(a.exp_m1())),
            (Add::add(&a, 1).ln(), a.ln_1p()),
            (Sub::sub(1_i32, a.erf()), a.erfc()),
            (Div::div(nonzero.sin(), &nonzero), nonzero.sinc()),
            (Add::add(a.exp_m1(), 1), a.exp()),
            (real.exp_m1().ln_1p(), real),
            (log_domain.ln_1p().exp_m1(), log_domain),
        ]
    }
}
