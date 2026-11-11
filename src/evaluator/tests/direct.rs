//! One-off evaluation, shared-node visits, and backend agreement.

use core::{
    ops::{Add, Mul},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{sync::Arc, vec};

use num_anafis::{Number, Numeric, Real};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr, Symbol, UserFunction},
    errors::EvalError,
    evaluator::{EvaluationValue, Program, Workspace},
};

fn generic_scalar<Value: Numeric + EvaluationValue>(
    expression: &Expr,
    symbol: Symbol<'_>,
    value: Value,
) -> Result<Value, EvalError> {
    let program = Program::<Value>::compile(expression, &[symbol])?;
    let mut workspace = Workspace::new();
    program.eval_with(&mut workspace, &[value])
}

proptest! {
    #[test]
    fn direct_and_register_paths_agree_on_shared_logical_nodes(
        input in -0.8_f64..=0.8,
        other in 0.1_f64..=0.9,
        coefficient in -4_i32..=4,
    ) {
        let context = Context::new()?;
        let x = context.symb("x").expect("available identity");
        let y = context.symb("y").expect("available identity");
        let shared = Add::add(x.sin(), y.cos());
        let expression = Expr::sum(vec![shared.pow(2), shared.pow(3), Mul::mul(shared.exp(), coefficient), y.to_expr()]);
        let native = Program::<f64>::compile(&expression, &[x, y]).expect("bound real expression");
        let mut native_workspace = Workspace::new();
        let direct_float = expression.evaluate([(x, input), (y, other)]).expect("complete bindings");
        let compiled_float = native.eval_with(&mut native_workspace, &[input, other]).expect("complete row");
        prop_assert!((direct_float - compiled_float).abs() <= 1.0e-12 * direct_float.abs().max(1.0));
        let left = Number::from_float(input);
        let right = Number::from_float(other);
        let general = Program::<Number>::compile(&expression, &[x, y]).expect("bound expression");
        let mut general_workspace = Workspace::new();
        let direct = expression.evaluate([(x, left.clone()), (y, right.clone())]).expect("complete bindings");
        prop_assert_eq!(Expr::number(direct), Expr::number(general.eval_with(&mut general_workspace, &[left, right]).expect("complete row")));
    }
}

#[test]
fn direct_errors_use_symbol_identity_and_reject_duplicate_bindings() {
    let context = Context::new().expect("context");
    let other = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let unrelated = other.symb("x").expect("distinct identity");
    let expression = Add::add(x, 1);
    assert!(matches!(
        expression.evaluate([(unrelated, 3.0)]),
        Err(EvalError::UnboundSymbol { .. })
    ));
    assert!(matches!(
        expression.evaluate([(x, 2.0), (x, 3.0)]),
        Err(EvalError::DuplicateParameter { index: 1 })
    ));
    assert_eq!(
        expression.evaluate([(x, 3.0)]).expect("complete bindings"),
        4.0
    );
    assert!(matches!(
        Expr::i().evaluate::<f64>([]),
        Err(EvalError::UnsupportedLiteral { .. })
    ));
}

#[test]
fn direct_custom_bodies_are_memoized_per_call() {
    let context = Context::new().expect("context");
    let counter = Arc::new(AtomicUsize::new(0));
    let captured = Arc::clone(&counter);
    let function = context
        .register_function(
            "square",
            UserFunction::new(1..=1).body(Arc::new(move |arguments| {
                captured.fetch_add(1, Ordering::Relaxed);
                arguments.first().expect("unary call").pow(2)
            })),
        )
        .expect("unique name");
    let x = context.symb("x").expect("available identity");
    let call = function.call(vec![x.to_expr()]).expect("valid arity");
    let expression = Add::add(call.sin(), call.cos());
    let value = expression
        .evaluate([(x, Number::from_int(2))])
        .expect("bound custom body");
    assert_eq!(
        value,
        Add::add(Number::from_int(4).sin(), Number::from_int(4).cos())
    );
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn numeric_backends_need_only_the_literal_conversion_contract() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let expression = Add::add(x.pow(2), 1);
    assert_eq!(
        generic_scalar(&expression, x, Real::from_int(3)).expect("real backend"),
        Real::from_int(10)
    );
    assert_eq!(
        generic_scalar(&expression, x, Number::from_int(3)).expect("general backend"),
        Number::from_int(10)
    );
}

#[test]
fn deep_shared_subgraphs_past_recursion_limit_remain_memoized() {
    std::thread_local! {
        static SINS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    }

    #[derive(Clone, Debug, PartialEq, PartialOrd)]
    struct Counted(Number);

    impl core::fmt::Display for Counted {
        fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            core::fmt::Display::fmt(&self.0, formatter)
        }
    }

    macro_rules! counted_ops {
        ($($trait:ident $method:ident),*) => { $(
            impl core::ops::$trait for Counted {
                type Output = Self;
                fn $method(self, other: Self) -> Self {
                    Self(core::ops::$trait::$method(self.0, other.0))
                }
            }
            impl core::ops::$trait<&Self> for Counted {
                type Output = Self;
                fn $method(self, other: &Self) -> Self {
                    Self(core::ops::$trait::$method(self.0, &other.0))
                }
            }
        )* };
    }

    counted_ops!(Add add, Sub sub, Mul mul, Div div);

    impl core::ops::Neg for Counted {
        type Output = Self;
        fn neg(self) -> Self {
            Self(core::ops::Neg::neg(self.0))
        }
    }

    macro_rules! delegate_counted {
        (unary: $($unary:ident),*; binary: $($binary:ident),*) => {
            $(fn $unary(&self) -> Self { Self(Numeric::$unary(&self.0)) })*
            $(fn $binary(&self, other: &Self) -> Self { Self(Numeric::$binary(&self.0, &other.0)) })*
        };
    }

    impl Numeric for Counted {
        fn from_number(value: &Number) -> Option<Self> {
            Some(Self(value.clone()))
        }

        fn sin(&self) -> Self {
            SINS.with(|count| count.set(count.get().saturating_add(1)));
            Self(Numeric::sin(&self.0))
        }

        delegate_counted!(
            unary: cos, tan, cot, sec, csc, asin, acos, atan, acot, asec, acsc, sinh, cosh,
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

    impl EvaluationValue for Counted {}

    let context = Context::new().expect("context");
    let x = context.symb("x").expect("available identity");
    let shared = x.sin();
    let left_branch = Add::add(shared.clone(), 1);
    let right_branch = Add::add(shared, 2);
    // Nest past RECURSION_DEPTH (6) so both branches cross the depth boundary.
    let mut expression = Add::add(
        Add::add(left_branch.clone(), right_branch),
        left_branch.cos(),
    );
    for _ in 0..7 {
        expression = expression.cos();
    }
    SINS.with(|count| count.set(0));
    let result = expression
        .evaluate([(x, Counted(Number::from_int(0)))])
        .expect("deep shared evaluation");
    assert_eq!(SINS.with(core::cell::Cell::get), 1);
    assert_eq!(
        result.0,
        expression
            .evaluate([(x, Number::from_int(0))])
            .expect("baseline evaluation")
    );
}
