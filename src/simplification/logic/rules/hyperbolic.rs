//! Hyperbolic angle identities and guarded exponential representations.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn hyperbolic_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let b = Self::capture("b", Property::Finite);
        let real = Self::capture("r", Property::Real);
        let nonnegative = Self::capture("n", Property::Nonnegative);
        let atanh_domain = Self::conditional("d", |node| {
            Self::proved(node, Property::Finite)
                && Self::proved(&Sub::sub(1_i32, node.pow(2)), Property::Nonzero)
        });
        let cosh_nonzero = Self::conditional("c", |node| {
            Self::proved(node, Property::Finite) && Self::proved(&node.cosh(), Property::Nonzero)
        });
        let one = Pattern::exact(1);
        let exp = a.exp();
        let inverse = Neg::neg(&a).exp();
        let double = Mul::mul(&a, 2);
        let triple = Mul::mul(&a, 3);
        let twice = double.exp();
        let exp_sum = Add::add(&exp, &inverse);
        let exp_difference = Sub::sub(&exp, &inverse);
        let twice_sum = Add::add(&twice, &one);
        let twice_difference = Sub::sub(&twice, &one);
        vec![
            (Sub::sub(a.cosh().pow(2), a.sinh().pow(2)), one.clone()),
            (Add::add(&one, a.sinh().pow(2)), a.cosh().pow(2)),
            (Sub::sub(a.cosh().pow(2), &one), a.sinh().pow(2)),
            (Sub::sub(&one, a.tanh().pow(2)), a.sech().pow(2)),
            (Sub::sub(a.coth().pow(2), &one), a.csch().pow(2)),
            (
                Add::add(cosh_nonzero.tanh().pow(2), cosh_nonzero.sech().pow(2)),
                one.clone(),
            ),
            (
                Pattern::product(vec![Pattern::exact(2), a.sinh(), a.cosh()]),
                double.sinh(),
            ),
            (Add::add(a.cosh().pow(2), a.sinh().pow(2)), double.cosh()),
            (
                Add::add(Mul::mul(a.sinh(), b.cosh()), Mul::mul(a.cosh(), b.sinh())),
                Add::add(&a, &b).sinh(),
            ),
            (
                Sub::sub(Mul::mul(a.sinh(), b.cosh()), Mul::mul(a.cosh(), b.sinh())),
                Sub::sub(&a, &b).sinh(),
            ),
            (
                Add::add(Mul::mul(a.cosh(), b.cosh()), Mul::mul(a.sinh(), b.sinh())),
                Add::add(&a, &b).cosh(),
            ),
            (
                Sub::sub(Mul::mul(a.cosh(), b.cosh()), Mul::mul(a.sinh(), b.sinh())),
                Sub::sub(&a, &b).cosh(),
            ),
            (
                Add::add(Mul::mul(a.sinh(), 3), Mul::mul(a.sinh().pow(3), 4)),
                triple.sinh(),
            ),
            (
                Sub::sub(Mul::mul(a.cosh().pow(3), 4), Mul::mul(a.cosh(), 3)),
                triple.cosh(),
            ),
            (Div::div(a.sinh(), a.cosh()), a.tanh()),
            (Div::div(a.cosh(), a.sinh()), a.coth()),
            (Div::div(&one, a.cosh()), a.sech()),
            (Div::div(&one, a.sinh()), a.csch()),
            (Div::div(&one, cosh_nonzero.tanh()), cosh_nonzero.coth()),
            (a.asinh().sinh(), a.clone()),
            (a.acosh().cosh(), a.clone()),
            (real.sinh().asinh(), real.clone()),
            (real.tanh().atanh(), real),
            (nonnegative.cosh().acosh(), nonnegative),
            (atanh_domain.atanh().tanh(), atanh_domain),
            (Div::div(&exp_difference, 2), a.sinh()),
            (Div::div(&exp_sum, 2), a.cosh()),
            (Div::div(&exp_difference, &exp_sum), a.tanh()),
            (Div::div(&exp_sum, &exp_difference), a.coth()),
            (Div::div(2_i32, &exp_sum), a.sech()),
            (Div::div(2_i32, &exp_difference), a.csch()),
            (Div::div(&twice_difference, &twice_sum), a.tanh()),
            (Div::div(&twice_sum, &twice_difference), a.coth()),
            (Div::div(Mul::mul(&exp, 2), &twice_sum), a.sech()),
            (Div::div(Mul::mul(&exp, 2), &twice_difference), a.csch()),
            (Div::div(&twice_sum, Mul::mul(&exp, 2)), a.cosh()),
            (Div::div(&twice_difference, Mul::mul(&exp, 2)), a.sinh()),
        ]
    }
}
