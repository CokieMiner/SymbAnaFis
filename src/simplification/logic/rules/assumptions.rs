//! Identities with caller-supplied zero, realness, or principal-range hypotheses.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::{DomainSafety, PreparedStep, SimplificationStep};

impl PreparedStep<'_> {
    pub fn assumed_rules(step: &SimplificationStep) -> Vec<(Pattern, Pattern)> {
        let a = Self::assumed_capture("a", Property::Finite);
        let b = Self::assumed_capture("b", Property::Finite);
        let real = Self::assumed_capture("r", Property::Real);
        let nonzero = Self::assumed_capture("n", Property::Nonzero);
        let one = Pattern::exact(1);
        match step {
            SimplificationStep::Algebraic => vec![
                (real.abs().pow(2), real.pow(2)),
                (real.signum().signum(), real.signum()),
                (nonzero.abs().signum(), one),
                (Mul::mul(real.abs(), real.signum()), real),
            ],
            SimplificationStep::Exponential => vec![
                (nonzero.ln().exp(), nonzero.clone()),
                (real.exp().ln(), real),
                (Mul::mul(&a, nonzero.ln()).exp(), nonzero.pow(&a)),
                (Add::add(a.ln(), b.ln()), Mul::mul(&a, &b).ln()),
                (Sub::sub(a.ln(), b.ln()), Div::div(&a, &b).ln()),
            ],
            SimplificationStep::Trigonometric => vec![
                (real.sin().asin(), real.clone()),
                (real.cos().acos(), real.clone()),
                (real.tan().atan(), real),
                (a.atan().tan(), a.clone()),
                (Div::div(&one, a.tan()), a.cot()),
                (Sub::sub(a.sec().pow(2), a.tan().pow(2)), one),
            ],
            SimplificationStep::Hyperbolic => vec![
                (real.sinh().asinh(), real.clone()),
                (real.tanh().atanh(), real.clone()),
                (real.cosh().acosh(), real),
                (a.atanh().tanh(), a.clone()),
                (Div::div(&one, a.tanh()), a.coth()),
                (Add::add(a.tanh().pow(2), a.sech().pow(2)), one),
            ],
            SimplificationStep::Roots => vec![
                (Mul::mul(a.sqrt(), b.sqrt()), Mul::mul(&a, &b).sqrt()),
                (
                    Div::div(a.sqrt(), nonzero.sqrt()),
                    Div::div(&a, &nonzero).sqrt(),
                ),
                (
                    Mul::mul(real.pow(2), &a).sqrt(),
                    Mul::mul(real.abs(), a.sqrt()),
                ),
            ],
            SimplificationStep::Compilation => vec![
                (Div::div(nonzero.sin(), &nonzero), nonzero.sinc()),
                (real.exp_m1().ln_1p(), real),
                (a.ln_1p().exp_m1(), a),
            ],
            SimplificationStep::Expand
            | SimplificationStep::Factor
            | SimplificationStep::Rewrite(_)
            | SimplificationStep::Map(_) => Vec::new(),
        }
    }

    fn assumed_capture(name: &str, property: Property) -> Pattern {
        Self::conditional(name, move |node| {
            Self::permitted(node, property, DomainSafety::AssumptionDependent)
        })
    }
}
