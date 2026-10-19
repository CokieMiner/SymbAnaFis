//! Finite polynomial identities for squares, cubes, and quadratic differences.

use core::ops::{Add, Mul, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn factoring_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let b = Self::capture("b", Property::Finite);
        let cross = Pattern::product(vec![Pattern::exact(2), a.clone(), b.clone()]);
        let cubic_first = Pattern::product(vec![Pattern::exact(3), a.pow(2), b.clone()]);
        let cubic_second = Pattern::product(vec![Pattern::exact(3), a.clone(), b.pow(2)]);
        let linear = Mul::mul(&a, 2);
        let square_term = Mul::mul(a.pow(2), 3);
        let cubic_linear = Mul::mul(&a, 3);
        vec![
            (
                Add::add(Add::add(a.pow(2), &linear), 1),
                Add::add(&a, 1).pow(2),
            ),
            (
                Add::add(Sub::sub(a.pow(2), &linear), 1),
                Sub::sub(&a, 1).pow(2),
            ),
            (
                Pattern::sum(vec![
                    a.pow(3),
                    square_term.clone(),
                    cubic_linear.clone(),
                    Pattern::exact(1),
                ]),
                Add::add(&a, 1).pow(3),
            ),
            (
                Sub::sub(Add::add(Sub::sub(a.pow(3), square_term), cubic_linear), 1),
                Sub::sub(&a, 1).pow(3),
            ),
            (
                Pattern::sum(vec![a.pow(2), cross.clone(), b.pow(2)]),
                Add::add(&a, &b).pow(2),
            ),
            (
                Add::add(Sub::sub(a.pow(2), cross), b.pow(2)),
                Sub::sub(&a, &b).pow(2),
            ),
            (
                Pattern::sum(vec![
                    a.pow(3),
                    cubic_first.clone(),
                    cubic_second.clone(),
                    b.pow(3),
                ]),
                Add::add(&a, &b).pow(3),
            ),
            (
                Sub::sub(
                    Add::add(Sub::sub(a.pow(3), cubic_first), cubic_second),
                    b.pow(3),
                ),
                Sub::sub(&a, &b).pow(3),
            ),
            (
                Sub::sub(a.pow(2), b.pow(2)),
                Mul::mul(Sub::sub(&a, &b), Add::add(&a, &b)),
            ),
        ]
    }
}
