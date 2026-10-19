//! Parameter ownership, custom bodies, and scalar-domain boundaries.

use core::{
    ops::{Add, Mul},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{sync::Arc, vec};

use num_anafis::{Complex, Number, Real};

use crate::{Context, EvalError, Evaluator, Expr, Function, UserFunction};

#[test]
fn input_identity_and_count_errors_do_not_change_outputs() {
    let first = Context::new();
    let second = Context::new();
    let x = first.symb("x").expect("available identity");
    let unrelated = second.symb("x").expect("distinct identity");
    assert!(matches!(
        Evaluator::<f64>::compile(&x.to_expr(), &[unrelated]),
        Err(EvalError::UnboundSymbol { .. })
    ));
    assert!(matches!(
        Evaluator::<f64>::compile(&x.to_expr(), &[x, x]),
        Err(EvalError::DuplicateParameter { index: 1 })
    ));
    let mut evaluator = Evaluator::<f64>::compile(&x.to_expr(), &[x]).expect("bound input");
    assert!(matches!(
        evaluator.evaluate(&[]),
        Err(EvalError::ParameterCount {
            expected: 1,
            actual: 0
        })
    ));
    assert!(matches!(
        evaluator.evaluate(&[1.0, 2.0]),
        Err(EvalError::ParameterCount {
            expected: 1,
            actual: 2
        })
    ));
    let mut output = [123.0, 456.0];
    assert!(matches!(
        evaluator.evaluate_batch(&[&[1.0]], &mut output),
        Err(EvalError::BatchLength { .. })
    ));
    assert_eq!(output, [123.0, 456.0]);
    assert_eq!(evaluator.evaluate(&[7.0]).expect("complete row"), 7.0);
}

#[test]
fn compiled_parameters_outlive_their_context() {
    let mut evaluator = {
        let context = Context::new();
        let x = context.symb("retained").expect("available identity");
        Evaluator::<Number>::compile(&Add::add(x, 1), &[x]).expect("closed expression")
    };
    assert_eq!(
        evaluator
            .parameters()
            .next()
            .expect("one input")
            .name()
            .as_deref(),
        Some("retained")
    );
    assert_eq!(
        evaluator
            .evaluate(&[Number::from_int(2)])
            .expect("complete row"),
        Number::from_int(3)
    );
    let mut cloned = evaluator.clone();
    assert_eq!(
        cloned
            .evaluate(&[Number::from_int(5)])
            .expect("complete row"),
        Number::from_int(6)
    );
    assert_eq!(
        evaluator
            .evaluate(&[Number::from_int(2)])
            .expect("complete row"),
        Number::from_int(3)
    );
}

#[test]
#[expect(
    clippy::suboptimal_flops,
    reason = "the numerical oracle preserves separate multiplication and addition"
)]
fn custom_bodies_expand_once_per_call_and_never_during_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut evaluator = {
        let context = Context::new();
        let counter = Arc::clone(&calls);
        let function = context
            .register_function(
                "square",
                UserFunction::new(1..=1).body(Arc::new(move |arguments| {
                    counter.fetch_add(1, Ordering::Relaxed);
                    arguments.first().expect("unary body").pow(2)
                })),
            )
            .expect("unique custom name");
        let x = context.symb("x").expect("available identity");
        let call = function.call(vec![x.to_expr()]).expect("validated arity");
        let nested = function.call(vec![call.clone()]).expect("validated arity");
        let expression = Add::add(Add::add(call.sin(), call.cos()), nested);
        Evaluator::<f64>::compile(&expression, &[x]).expect("nested nonrecursive calls")
    };
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    for input in [0.5_f64, 0.75, 1.0] {
        let value = evaluator.evaluate(&[input]).expect("complete row");
        let square = input * input;
        assert!((value - (square.sin() + square.cos() + square * square)).abs() < 1.0e-14);
    }
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn recursive_definitions_and_unsupported_calls_fail_during_compilation() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let abstract_function = context
        .register_function("abstract_f", UserFunction::new(1..=1))
        .expect("unique name");
    let abstract_call = abstract_function
        .call(vec![x.to_expr()])
        .expect("validated arity");
    assert!(matches!(
        Evaluator::<Number>::compile(&abstract_call, &[x]),
        Err(EvalError::MissingBody { .. })
    ));
    let derivative = Expr::derivative(abstract_call, x, 1);
    assert!(matches!(
        Evaluator::<f64>::compile(&derivative, &[x]),
        Err(EvalError::UnevaluatedDerivative)
    ));
    let slot = Arc::new(std::sync::Mutex::new(None::<Function>));
    let captured = Arc::clone(&slot);
    let recursive = context
        .register_function(
            "recursive",
            UserFunction::new(1..=1).body(Arc::new(move |arguments| {
                captured
                    .lock()
                    .expect("test mutex")
                    .as_ref()
                    .expect("registered function")
                    .call(vec![Add::add(arguments.first().expect("unary body"), 1)])
                    .expect("validated arity")
            })),
        )
        .expect("unique name");
    *slot.lock().expect("test mutex") = Some(recursive.clone());
    let input = recursive.call(vec![x.to_expr()]).expect("validated arity");
    assert!(matches!(
        Evaluator::<f64>::compile(&input, &[x]),
        Err(EvalError::RecursiveFunction { .. })
    ));
    drop(slot.lock().expect("test mutex").take());
}

#[test]
fn complex_continuations_and_real_only_literals_follow_the_scalar() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let expression = x.to_expr().sqrt();
    let mut general = Evaluator::<Number>::compile(&expression, &[x]).expect("scalar square root");
    assert_eq!(
        general
            .evaluate(&[Number::from_int(-4)])
            .expect("complete row"),
        Number::from_complex(Number::zero(), Number::from_int(2))
    );
    let mut real = Evaluator::<Real>::compile(&expression, &[x]).expect("real square root");
    assert!(
        real.evaluate(&[Real::from_int(-4)])
            .expect("complete row")
            .is_nan()
    );
    let mut complex =
        Evaluator::<Complex>::compile(&Mul::mul(x, Expr::i()), &[x]).expect("complex literal");
    assert_eq!(
        complex
            .evaluate(&[Number::from_int(2)
                .extract_complex()
                .expect("scalar components")])
            .expect("complete row"),
        Number::from_complex(Number::zero(), Number::from_int(2))
            .extract_complex()
            .expect("scalar components")
    );
    assert!(matches!(
        Evaluator::<f64>::compile(&Expr::i(), &[]),
        Err(EvalError::UnsupportedLiteral { .. })
    ));
    assert!(matches!(
        Evaluator::<Real>::compile(&Expr::i(), &[]),
        Err(EvalError::UnsupportedLiteral { .. })
    ));
}

#[test]
fn signed_zero_nonfinite_inputs_and_constant_programs_are_preserved() {
    let context = Context::new();
    let x = context.symb("x").expect("available identity");
    let mut identity = Evaluator::<f64>::compile(&x.to_expr(), &[x]).expect("one input");
    assert_eq!(identity.instruction_count(), 0);
    assert_eq!(
        identity.evaluate(&[-0.0]).expect("complete row").to_bits(),
        (-0.0_f64).to_bits()
    );
    let mut square = Evaluator::<f64>::compile(&x.pow(2), &[x]).expect("square");
    assert_eq!(square.instruction_count(), 1);
    assert_eq!(
        square.evaluate(&[-0.0]).expect("complete row").to_bits(),
        0.0_f64.to_bits()
    );
    assert!(square.evaluate(&[f64::NAN]).expect("complete row").is_nan());
    let mut constant = Evaluator::<Number>::compile(&Expr::number(7), &[]).expect("constant");
    let mut output = vec![Number::zero(); 4];
    constant
        .evaluate_batch(&[], &mut output)
        .expect("no columns");
    assert_eq!(output, vec![Number::from_int(7); 4]);
}

#[test]
fn symbolic_constants_use_the_numeric_backends_values() {
    for (expression, expected) in [(Expr::pi(), Number::pi()), (Expr::e(), Number::e())] {
        let mut general = Evaluator::<Number>::compile(&expression, &[]).expect("constant program");
        assert_eq!(
            Expr::number(general.evaluate(&[]).expect("no inputs")),
            Expr::number(expected.clone())
        );
        assert_eq!(
            Expr::number(expression.evaluate::<Number>([]).expect("no bindings")),
            Expr::number(expected.clone())
        );
        let mut native = Evaluator::<f64>::compile(&expression, &[]).expect("real constant");
        assert_eq!(
            native.evaluate(&[]).expect("no inputs").to_bits(),
            expected.to_f64().to_bits()
        );
    }
}

#[cfg(not(miri))]
#[test]
fn deep_compilation_and_execution_use_bounded_call_stack() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("available identity");
            let mut expression = x.to_expr();
            for _ in 0..2_000 {
                expression = expression.sin();
            }
            let mut evaluator =
                Evaluator::<f64>::compile(&expression, &[x]).expect("deep scalar DAG");
            assert!(
                evaluator
                    .evaluate(&[0.1])
                    .expect("complete row")
                    .is_finite()
            );
            assert!(evaluator.workspace_size() <= 2);
            assert_eq!(
                expression
                    .evaluate([(x, 0.0)])
                    .expect("deep direct evaluation"),
                0.0
            );
            assert_eq!(
                expression
                    .evaluate([(x, Number::zero())])
                    .expect("deep direct Number evaluation"),
                Number::zero()
            );
        })
        .expect("test thread")
        .join()
        .expect("bounded stack");
}
