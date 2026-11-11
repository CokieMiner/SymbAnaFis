//! Shared immutable programs and independently owned register workspaces.
//!
//! `Fragile` is a custom scalar over `Number` whose clones panic while
//! `FAIL_CLONES` is set on the current thread.

use core::{
    cell::Cell,
    fmt::{Display, Formatter, Result as FmtResult},
    ops::{Add, Div, Mul, Neg, Sub},
    panic::AssertUnwindSafe,
};
use std::panic::catch_unwind;

use alloc::{sync::Arc, vec::Vec};

use num_anafis::{Number, Numeric};

use crate::core::{Context, EvaluationValue, Expr};

use super::super::{Program, Workspace};

std::thread_local! {
    static FAIL_CLONES: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, PartialEq, PartialOrd, Hash)]
struct Fragile(Number);

impl Clone for Fragile {
    fn clone(&self) -> Self {
        assert!(!FAIL_CLONES.get(), "injected clone failure");
        Self(self.0.clone())
    }
}

impl Display for Fragile {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        Display::fmt(&self.0, formatter)
    }
}

macro_rules! operators {
    ($($operator:ident $method:ident),*) => { $(
        impl $operator for Fragile {
            type Output = Self;
            fn $method(self, other: Self) -> Self {
                Self($operator::$method(self.0, other.0))
            }
        }
        impl $operator<&Self> for Fragile {
            type Output = Self;
            fn $method(self, other: &Self) -> Self {
                Self($operator::$method(self.0, &other.0))
            }
        }
    )* };
}

operators!(Add add, Sub sub, Mul mul, Div div);

impl Neg for Fragile {
    type Output = Self;
    fn neg(self) -> Self {
        Self(Neg::neg(self.0))
    }
}

macro_rules! delegate {
    (unary: $($unary:ident),*; binary: $($binary:ident),*) => {
        $(fn $unary(&self) -> Self { Self(Numeric::$unary(&self.0)) })*
        $(fn $binary(&self, other: &Self) -> Self { Self(Numeric::$binary(&self.0, &other.0)) })*
    };
}

impl Numeric for Fragile {
    delegate!(
        unary: sin, cos, tan, cot, sec, csc, asin, acos, atan, acot, asec, acsc, sinh, cosh,
            tanh, coth, sech, csch, asinh, acosh, atanh, acoth, acsch, asech, exp, exp_m1,
            exp_neg, ln, ln_1p, log2, log10, sqrt, cbrt, abs, signum, floor, ceil, round, trunc,
            fract, negate, sinc, gamma, lgamma, digamma, trigamma, tetragamma, erf, erfc, zeta,
            elliptic_k, elliptic_e;
        binary: atan2, log, pow, polygamma, beta, zeta_deriv, bessel_j, bessel_y, bessel_i,
            bessel_k, hermite, lambert_w
    );

    fn mul_add(&self, multiplier: &Self, addend: &Self) -> Self {
        Self(Numeric::mul_add(&self.0, &multiplier.0, &addend.0))
    }

    fn assoc_legendre(&self, degree: &Self, order: &Self) -> Self {
        Self(Numeric::assoc_legendre(&self.0, &degree.0, &order.0))
    }
}

impl EvaluationValue for Fragile {
    fn from_number(value: &Number) -> Option<Self> {
        Some(Self(value.clone()))
    }
}

#[test]
fn one_workspace_rebinds_across_programs_and_supports_batches() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let first = Program::<f64>::compile(&Add::add(x, 1), &[x]).expect("first program");
    let second = Program::<f64>::compile(&Mul::mul(x, 2), &[x]).expect("second program");
    let mut workspace = Workspace::new();

    assert_eq!(
        first.evaluate(&mut workspace, &[3.0]).expect("first row"),
        4.0
    );
    assert_eq!(
        second
            .evaluate(&mut workspace, &[3.0])
            .expect("rebound row"),
        6.0
    );
    assert_eq!(
        first
            .evaluate(&mut workspace, &[4.0])
            .expect("original row"),
        5.0
    );

    let mut output = [0.0; 3];
    first
        .evaluate_batch(&mut workspace, &[&[1.0, 2.0, 3.0]], &mut output)
        .expect("batch rows");
    assert_eq!(output, [2.0, 3.0, 4.0]);
}

#[test]
fn one_workspace_rebinds_across_parameter_and_constant_shapes() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let function = Program::<f64>::compile(&Add::add(x, 1), &[x]).expect("function program");
    let constant = Program::<f64>::compile(&Expr::number(9), &[]).expect("constant program");
    let mut workspace = Workspace::new();

    assert_eq!(
        function
            .evaluate(&mut workspace, &[2.0])
            .expect("function row"),
        3.0
    );
    assert_eq!(
        constant
            .evaluate(&mut workspace, &[])
            .expect("constant row"),
        9.0
    );
    assert_eq!(
        function
            .evaluate(&mut workspace, &[4.0])
            .expect("rebound function row"),
        5.0
    );
}

#[test]
fn a_panicking_rebind_leaves_an_unbound_workspace() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let first = Program::<Fragile>::compile(&Add::add(x, 1), &[x]).expect("first program");
    let second = Program::<Fragile>::compile(&Mul::mul(x, 2), &[x]).expect("second program");
    let input = [Fragile(Number::from_int(3))];
    let expected = Fragile(Number::from_int(4));
    let mut workspace = first.workspace();
    assert_eq!(
        first.evaluate(&mut workspace, &input).expect("bound row"),
        expected
    );
    FAIL_CLONES.set(true);
    let rebind = catch_unwind(AssertUnwindSafe(|| second.evaluate(&mut workspace, &input)));
    FAIL_CLONES.set(false);
    assert!(rebind.is_err());
    assert!(workspace.program.is_none());
    assert_eq!(
        first.evaluate(&mut workspace, &input).expect("rebuilt row"),
        expected
    );
}

#[cfg(not(miri))]
#[test]
fn shared_programs_execute_with_one_workspace_per_thread() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let program = Arc::new(Program::<f64>::compile(&Add::add(x, 1), &[x]).expect("program"));
    let handles: Vec<_> = [(1.0, 2.0), (2.0, 3.0), (3.0, 4.0), (4.0, 5.0)]
        .into_iter()
        .map(|(input, expected)| {
            let worker_program = Arc::clone(&program);
            std::thread::spawn(move || {
                let mut workspace = worker_program.workspace();
                for _ in 0..64 {
                    assert_eq!(
                        worker_program
                            .evaluate(&mut workspace, &[input])
                            .expect("complete row"),
                        expected
                    );
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("worker thread");
    }
}
