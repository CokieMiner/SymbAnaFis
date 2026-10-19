//! Callback ownership, DAG memoization, reconstruction, and bounded-stack contracts.

use core::{
    ops::Neg,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{sync::Arc, vec};

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView, Substitution, UserFunction};

proptest! {
    #[test]
    fn shared_custom_nodes_invoke_each_active_partial_once(edges in 2_usize..=20) {
        let context = Context::new();
        let x = context.symb("x")?;
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let f = context.register_function("f", UserFunction::new(1..=1)
            .partial(0, Arc::new(move |_| { counter.fetch_add(1, Ordering::Relaxed); Expr::number(1) }))?)?;
        let pair = context.register_function("pair", UserFunction::new(2..=20))?;
        let input = pair.call(vec![f.call(vec![x.into()])?; edges])?;
        let result = Expr::derivative(input, x, 1);
        prop_assert!(matches!(result.view(), ExprView::Derivative { .. }), "the missing outer partial remains symbolic");
        prop_assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn high_orders_skip_periodic_derivative_states(order in proptest::num::u32::ANY) {
        let context = Context::new();
        let x = context.symb("x")?;
        let expected = match order.checked_rem(4).expect("four is nonzero") {
            0 => x.sin(), 1 => x.cos(), 2 => Neg::neg(x.sin()), _ => Neg::neg(x.cos()),
        };
        prop_assert_eq!(Expr::derivative(x.sin(), x, order), expected);
    }

    #[test]
    fn equal_symbol_names_from_other_owners_are_constants(order in 1_u32..=16) {
        let first = Context::new();
        let second = Context::new();
        let x = first.symb("x")?;
        let other = second.symb("x")?;
        prop_assert_eq!(Expr::derivative(x.sin(), other, order), Expr::number(0));
    }
}

#[test]
fn missing_partials_are_checked_before_running_any_callback() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let f = context
        .register_function(
            "f",
            UserFunction::new(2..=2)
                .partial(
                    0,
                    Arc::new(move |_| {
                        counter.fetch_add(1, Ordering::Relaxed);
                        Expr::number(1)
                    }),
                )
                .expect("argument zero exists"),
        )
        .expect("unbound custom name");
    let input = f.call(vec![x.into(), x.sin()]).expect("binary call");
    let deferred = Expr::derivative(input.clone(), x, 1);
    assert!(matches!(deferred.view(), ExprView::Derivative { inner, .. } if inner == &input));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let independent = f
        .call(vec![x.into(), Expr::number(2)])
        .expect("binary call");
    assert_eq!(Expr::derivative(independent, x, 1), Expr::number(1));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn expressions_retain_partials_after_context_destruction_without_expanding_bodies() {
    let input = {
        let context = Context::new();
        let x = context.symb("x").expect("symbol allocation");
        let f = context
            .register_function(
                "f",
                UserFunction::new(1..=1)
                    .body(Arc::new(|_| Expr::number(42)))
                    .partial(
                        0,
                        Arc::new(|args| {
                            Expr::product(vec![
                                Expr::number(2),
                                args.first().expect("unary arguments").clone(),
                            ])
                        }),
                    )
                    .expect("argument zero exists"),
            )
            .expect("unbound custom name");
        f.call(vec![x.into()]).expect("unary call")
    };
    let argument = input.children().next().expect("unary custom call");
    let variable = argument.as_symbol().expect("symbol argument");
    assert_eq!(
        Expr::derivative(input.clone(), variable, 2),
        Expr::number(2)
    );
    assert_eq!(Expr::derivative(input, variable, 3), Expr::number(0));
}

#[test]
fn rebuilding_a_deferred_derivative_retries_supported_rules() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let f = context
        .register_function("f", UserFunction::new(1..=1))
        .expect("unbound custom name");
    let call = f.call(vec![x.into()]).expect("unary call");
    let deferred = Expr::derivative(call.clone(), x, 2);
    let replacement = x.sin();
    let expected = -x.sin();
    assert_eq!(
        Substitution::new(call.clone(), replacement.clone()).apply_all(&deferred),
        expected
    );
    let mapped = deferred
        .map(|node| Ok::<_, ()>((node == &call).then(|| replacement.clone())))
        .expect("infallible transformation");
    assert_eq!(mapped, expected);
}

#[test]
fn iterative_differentiation_and_reclamation_fit_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("symbol allocation");
            let mut input = x.to_expr();
            for _ in 0..512 {
                input = input.sin();
            }
            let derivative = Expr::derivative(input, x, 1);
            assert!(derivative.contains_symbol(x));
            assert!(!derivative.fold(false, |found, node| found
                || matches!(node.view(), ExprView::Derivative { .. })));
        })
        .expect("small-stack worker")
        .join()
        .expect("iterative differentiation and drop");
}
