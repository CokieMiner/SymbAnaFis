//! Entire angle identities and quotient rules retaining the same poles.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::{vec, vec::Vec};

use crate::core::{Expr, Pattern, Property};

use super::PreparedStep;

impl PreparedStep<'_> {
    pub fn trigonometric_rules() -> Vec<(Pattern, Pattern)> {
        let a = Self::capture("a", Property::Finite);
        let b = Self::capture("b", Property::Finite);
        let atan_domain = Self::conditional("d", |node| {
            Self::proved(node, Property::Finite)
                && Self::proved(&Add::add(node.pow(2), 1), Property::Nonzero)
        });
        let cosine_nonzero = Self::conditional("c", |node| {
            Self::proved(node, Property::Finite) && Self::proved(&node.cos(), Property::Nonzero)
        });
        let one = Pattern::exact(1);
        let half = Pattern::exact(Div::div(Expr::pi(), 2));
        let double = Mul::mul(&a, 2);
        let triple = Mul::mul(&a, 3);
        vec![
            (Add::add(a.sin().pow(2), a.cos().pow(2)), one.clone()),
            (Sub::sub(&one, a.sin().pow(2)), a.cos().pow(2)),
            (Sub::sub(&one, a.cos().pow(2)), a.sin().pow(2)),
            (Add::add(&one, a.tan().pow(2)), a.sec().pow(2)),
            (Add::add(&one, a.cot().pow(2)), a.csc().pow(2)),
            (Sub::sub(a.sec().pow(2), &one), a.tan().pow(2)),
            (Sub::sub(a.csc().pow(2), &one), a.cot().pow(2)),
            (
                Sub::sub(cosine_nonzero.sec().pow(2), cosine_nonzero.tan().pow(2)),
                one.clone(),
            ),
            (Sub::sub(a.cos().pow(2), a.sin().pow(2)), double.cos()),
            (
                Sub::sub(a.sin().pow(2), a.cos().pow(2)),
                Neg::neg(double.cos()),
            ),
            (
                Mul::mul(Sub::sub(a.cos(), a.sin()), Add::add(a.cos(), a.sin())),
                double.cos(),
            ),
            (
                Pattern::product(vec![Pattern::exact(2), a.sin(), a.cos()]),
                double.sin(),
            ),
            (Mul::mul(a.sin(), a.cos()), Div::div(double.sin(), 2)),
            (
                Add::add(Mul::mul(a.sin(), b.cos()), Mul::mul(a.cos(), b.sin())),
                Add::add(&a, &b).sin(),
            ),
            (
                Sub::sub(Mul::mul(a.sin(), b.cos()), Mul::mul(a.cos(), b.sin())),
                Sub::sub(&a, &b).sin(),
            ),
            (
                Sub::sub(Mul::mul(a.cos(), b.cos()), Mul::mul(a.sin(), b.sin())),
                Add::add(&a, &b).cos(),
            ),
            (
                Add::add(Mul::mul(a.cos(), b.cos()), Mul::mul(a.sin(), b.sin())),
                Sub::sub(&a, &b).cos(),
            ),
            (
                Sub::sub(Mul::mul(a.sin(), 3), Mul::mul(a.sin().pow(3), 4)),
                triple.sin(),
            ),
            (
                Sub::sub(Mul::mul(a.cos().pow(3), 4), Mul::mul(a.cos(), 3)),
                triple.cos(),
            ),
            (Div::div(a.sin(), a.cos()), a.tan()),
            (Div::div(a.cos(), a.sin()), a.cot()),
            (Div::div(&one, a.cos()), a.sec()),
            (Div::div(&one, a.sin()), a.csc()),
            (Div::div(&one, cosine_nonzero.tan()), cosine_nonzero.cot()),
            (a.asin().sin(), a.clone()),
            (a.acos().cos(), a.clone()),
            (atan_domain.atan().tan(), atan_domain),
            (Sub::sub(&half, &a).sin(), a.cos()),
            (Sub::sub(&half, &a).cos(), a.sin()),
            (Sub::sub(&half, &a).tan(), a.cot()),
            (Sub::sub(&half, &a).cot(), a.tan()),
            (Sub::sub(&half, &a).sec(), a.csc()),
            (Sub::sub(&half, &a).csc(), a.sec()),
        ]
    }
}
