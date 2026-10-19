//! One-off evaluation, shared-node visits, and backend agreement.

use core::ops::{Add, Mul};

use alloc::{sync::Arc, vec};

use num_anafis::{Number, Numeric, Real};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{Context, EvalError, EvaluationValue, Evaluator, Expr, Symbol, UserFunction};

proptest! {
    #[test]
    fn direct_and_register_paths_agree_on_shared_logical_nodes(
        input in -0.8_f64..=0.8,
        other in 0.1_f64..=0.9,
        coefficient in -4_i32..=4,
    ) {
        let context = Context::new();
        let x = context.symb("x").expect("available identity");
        let y = context.symb("y").expect("available identity");
        let shared = Add::add(x.sin(), y.cos());
        let expression = Expr::sum(vec![shared.pow(2), shared.pow(3), Mul::mul(shared.exp(), coefficient), y.to_expr()]);
        let mut native = Evaluator::<f64>::compile(&expression, &[x, y]).expect("bound real expression");
        let direct_float = expression.evaluate([(x, input), (y, other)]).expect("complete bindings");
        let compiled_float = native.evaluate(&[input, other]).expect("complete row");
        prop_assert!((direct_float - compiled_float).abs() <= 1.0e-12 * direct_float.abs().max(1.0));
        let left = Number::from_float(input);
        let right = Number::from_float(other);
        let mut general = Evaluator::<Number>::compile(&expression, &[x, y]).expect("bound expression");
        let direct = expression.evaluate([(x, left.clone()), (y, right.clone())]).expect("complete bindings");
        prop_assert_eq!(Expr::number(direct), Expr::number(general.evaluate(&[left, right]).expect("complete row")));
    }
}

#[test]
fn direct_errors_use_symbol_identity_and_reject_duplicate_bindings() {
    let context = Context::new();
    let other = Context::new();
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
    use core::sync::atomic::{AtomicUsize, Ordering};
    let context = Context::new();
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

fn generic_scalar<Value: Numeric + EvaluationValue>(
    expression: &Expr,
    symbol: Symbol<'_>,
    value: Value,
) -> Result<Value, EvalError> {
    let mut evaluator = Evaluator::<Value>::compile(expression, &[symbol])?;
    evaluator.evaluate(&[value])
}

#[test]
fn numeric_backends_need_only_the_literal_conversion_contract() {
    let context = Context::new();
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
