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

use alloc::{sync::Arc, vec, vec::Vec};

use num_anafis::{Number, Numeric};
#[cfg(feature = "rayon")]
use rayon::ThreadPoolBuilder;

use crate::core::{Context, Expr};
#[cfg(feature = "rayon")]
use crate::errors::EvalError;

use super::super::{EvaluationValue, Program, Workspace};

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
    fn from_number(value: &Number) -> Option<Self> {
        Some(Self(value.clone()))
    }

    delegate!(
        unary: sin, cos, tan, cot, sec, csc, asin, acos, atan, acot, asec, acsc, sinh, cosh,
            tanh, coth, sech, csch, asinh, acosh, atanh, acoth, acsch, asech, exp, exp2, exp10,
            exp_m1, ln, ln_1p, log2, log10, sqrt, rsqrt, recip, cbrt, abs, signum, re, im, conj,
            heaviside, floor, ceil, round, trunc, fract, sinc, sin_pi, cos_pi, gamma, lgamma,
            digamma, trigamma, tetragamma, erf, erfc, erfcx, erfi, dawson, faddeeva, zeta,
            elliptic_k, elliptic_e;
        binary: atan2, log, pow, hypot, max, min, modulo, rem, polygamma, beta, zeta_deriv,
            bessel_j, bessel_y, bessel_i, bessel_k, hermite, lambert_w
    );

    fn mul_add(&self, multiplier: &Self, addend: &Self) -> Self {
        Self(Numeric::mul_add(&self.0, &multiplier.0, &addend.0))
    }

    fn clamp(&self, min: &Self, max: &Self) -> Self {
        Self(Numeric::clamp(&self.0, &min.0, &max.0))
    }

    fn assoc_legendre(&self, degree: &Self, order: &Self) -> Self {
        Self(Numeric::assoc_legendre(&self.0, &degree.0, &order.0))
    }
}

impl EvaluationValue for Fragile {}

#[test]
fn one_workspace_rebinds_across_programs_and_supports_batches() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let first = Program::<f64>::compile(&Add::add(x, 1), &[x]).expect("first program");
    let second = Program::<f64>::compile(&Mul::mul(x, 2), &[x]).expect("second program");
    let mut workspace = Workspace::new();

    assert_eq!(
        first.eval_with(&mut workspace, &[3.0]).expect("first row"),
        4.0
    );
    assert_eq!(
        second
            .eval_with(&mut workspace, &[3.0])
            .expect("rebound row"),
        6.0
    );
    assert_eq!(
        first
            .eval_with(&mut workspace, &[4.0])
            .expect("original row"),
        5.0
    );

    let mut output = [0.0; 3];
    first
        .eval_batch_with(&mut workspace, &[&[1.0, 2.0, 3.0]], &mut output)
        .expect("batch rows");
    assert_eq!(output, [2.0, 3.0, 4.0]);
    assert_eq!(workspace.lanes, 16);

    let mut empty = [];
    first
        .eval_batch_with(&mut workspace, &[&[]], &mut empty)
        .expect("empty batch preserves workspace lane width");
    assert_eq!(workspace.lanes, 16);
}

#[test]
fn batch_rows_match_scalar_rows_for_every_lane_count() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let expression = Add::add(
        Mul::mul(Add::add(Mul::mul(x, 3), y).sin(), Add::add(x, 2).cos()),
        Div::div(Add::add(x, y), Add::add(Mul::mul(x, x), 1)),
    );
    let program = Program::<f64>::compile(&expression, &[x, y]).expect("program");
    let mut workspace = Workspace::new();
    // One row, partial blocks, whole blocks, and multi-block tails.
    for rows in [1_usize, 3, 15, 16, 17, 32, 33] {
        let xs: Vec<f64> = (0..rows)
            .map(|row| 0.125_f64.mul_add(f64::from(u32::try_from(row).expect("short batch")), 0.25))
            .collect();
        let ys: Vec<f64> = (0..rows)
            .map(|row| {
                0.0625_f64.mul_add(f64::from(u32::try_from(row).expect("short batch")), -0.5)
            })
            .collect();
        let mut output = vec![0.0; rows];
        program
            .eval_batch_with(&mut workspace, &[&xs, &ys], &mut output)
            .expect("batch rows");
        let mut transient = vec![0.0; rows];
        program
            .eval_batch(&[&xs, &ys], &mut transient)
            .expect("transient batch rows");
        assert_eq!(transient, output);
        for (row, batched) in output.iter().enumerate() {
            let scalar = program
                .eval_with(
                    &mut workspace,
                    &[
                        *xs.get(row).expect("row of the column"),
                        *ys.get(row).expect("row of the column"),
                    ],
                )
                .expect("scalar row");
            assert_eq!(*batched, scalar, "row {row} of {rows}");
        }
    }
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
            .eval_with(&mut workspace, &[2.0])
            .expect("function row"),
        3.0
    );
    assert_eq!(
        constant
            .eval_with(&mut workspace, &[])
            .expect("constant row"),
        9.0
    );
    assert_eq!(
        function
            .eval_with(&mut workspace, &[4.0])
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
    let mut workspace = Workspace::new();
    assert_eq!(
        first.eval_with(&mut workspace, &input).expect("bound row"),
        expected
    );
    FAIL_CLONES.set(true);
    let rebind = catch_unwind(AssertUnwindSafe(|| {
        second.eval_with(&mut workspace, &input)
    }));
    FAIL_CLONES.set(false);
    assert!(rebind.is_err());
    assert!(workspace.program.is_none());
    assert_eq!(
        first
            .eval_with(&mut workspace, &input)
            .expect("rebuilt row"),
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
                let mut workspace = Workspace::new();
                for _ in 0..64 {
                    assert_eq!(
                        worker_program
                            .eval_with(&mut workspace, &[input])
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

#[cfg(feature = "rayon")]
#[test]
fn parallel_batches_validate_shapes_and_match_sequential_across_pools() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let y = context.symb("y").expect("available identity");
    let expression = Add::add(
        Mul::mul(Add::add(Mul::mul(x, 3), y).sin(), Add::add(x, 2).cos()),
        Div::div(Add::add(x.pow(3), y.pow(2)), Add::add(Mul::mul(x, x), 1)),
    );
    let program = Program::<f64>::compile(&expression, &[x, y]).expect("float program");

    let mut untouched = [42.0, 99.0];
    assert!(matches!(
        program.eval_batch_par(&[&[1.0, 2.0]], &mut untouched),
        Err(EvalError::ParameterCount {
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        program.eval_batch_par(&[&[1.0, 2.0], &[3.0]], &mut untouched),
        Err(EvalError::BatchLength {
            parameter: 1,
            expected: 2,
            actual: 1
        })
    ));
    assert_eq!(untouched, [42.0, 99.0]);

    let mut empty = [];
    program
        .eval_batch_par(&[&[], &[]], &mut empty)
        .expect("empty batch");

    let constant = Program::<f64>::compile(&Expr::number(7), &[]).expect("constant program");
    let mut repeated = vec![0.0; 257];
    constant
        .eval_batch_par(&[], &mut repeated)
        .expect("zero-parameter batch");
    assert!(
        repeated
            .iter()
            .all(|value| value.to_bits() == 7.0_f64.to_bits())
    );

    let mut workspace = Workspace::new();
    for rows in [1_usize, 2, 5, 16, 31, 32, 33, 128, 257, 2_051] {
        let xs: Vec<f64> = (0..rows)
            .map(|row| 0.01_f64.mul_add(f64::from(u32::try_from(row).expect("small batch")), -1.5))
            .collect();
        let ys: Vec<f64> = (0..rows)
            .map(|row| 0.02_f64.mul_add(f64::from(u32::try_from(row).expect("small batch")), 0.25))
            .collect();
        let mut expected = vec![0.0; rows];
        program
            .eval_batch_with(&mut workspace, &[&xs, &ys], &mut expected)
            .expect("sequential baseline");
        let mut parallel = vec![0.0; rows];
        program
            .eval_batch_par(&[&xs, &ys], &mut parallel)
            .expect("parallel batch");
        assert_eq!(
            parallel.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "parallel mismatch at {rows} rows"
        );
        let pool = ThreadPoolBuilder::new()
            .num_threads(3)
            .build()
            .expect("custom test pool");
        let mut scoped = vec![0.0; rows];
        pool.install(|| program.eval_batch_par(&[&xs, &ys], &mut scoped))
            .expect("scoped parallel batch");
        assert_eq!(
            scoped.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "3-thread pool mismatch at {rows} rows"
        );
    }

    let exact_program = Program::<Number>::compile(&Add::add(x.pow(2), y), &[x, y]).expect("exact");
    let exact_rows = 300_usize;
    let first_col: Vec<Number> = (0..exact_rows)
        .map(|row| Number::from_int(i64::try_from(row).expect("small batch")))
        .collect();
    let second_col: Vec<Number> = (0..exact_rows)
        .map(|row| Number::from_int(i64::try_from(row.saturating_mul(2)).expect("small batch")))
        .collect();
    let mut exact_expected = vec![Number::zero(); exact_rows];
    let mut exact_workspace = Workspace::new();
    exact_program
        .eval_batch_with(
            &mut exact_workspace,
            &[&first_col, &second_col],
            &mut exact_expected,
        )
        .expect("exact baseline");
    let mut exact_parallel = vec![Number::zero(); exact_rows];
    exact_program
        .eval_batch_par(&[&first_col, &second_col], &mut exact_parallel)
        .expect("exact parallel");
    assert_eq!(exact_parallel, exact_expected);
}
