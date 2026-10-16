//! Deterministic expression edges that need fixed identities, limits, or layout.

use core::mem::size_of;

use std::{sync::Barrier, thread};

use alloc::{format, sync::Arc, vec, vec::Vec};

use crate::{
    collections::HashSet,
    core::{Context, Symbol, UserFunction},
};

use super::super::{Expr, ExprView};

#[test]
fn handle_is_one_pointer() {
    assert_eq!(size_of::<Expr>(), size_of::<usize>());
}

#[test]
fn literal_identity_keeps_float_bits_including_nan() {
    for value in [0.0, -0.0, f64::INFINITY, f64::NAN] {
        let first = Expr::number(value);
        let second = Expr::number(value);
        assert!(first.same_node(&second));
        assert_eq!(first, second);
    }
    let literals: HashSet<_> = [
        0.0,
        -0.0,
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0x7ff8_0000_0000_0002),
    ]
    .into_iter()
    .map(Expr::number)
    .collect();
    assert_eq!(literals.len(), 4);
}

#[test]
fn concurrent_publication_allocates_one_live_node() {
    let x = Symbol::intern("dag_concurrent_x");
    let context = Context::new();
    let function = context
        .register_function("concurrent", UserFunction::new(1..=1))
        .expect("the custom name is unbound");
    let barrier = Arc::new(Barrier::new(9));
    #[expect(
        clippy::needless_collect,
        reason = "all workers must be spawned before the barrier is released"
    )]
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let shared_barrier = Arc::clone(&barrier);
            let handle = function.clone();
            thread::spawn(move || {
                shared_barrier.wait();
                handle.call(vec![x - 1]).expect("the function is unary")
            })
        })
        .collect();
    barrier.wait();
    let roots: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("construction thread succeeded"))
        .collect();
    let first = roots.first().expect("workers produced roots");
    assert!(roots.iter().all(|root| root.same_node(first)));
}

#[test]
#[expect(
    clippy::erasing_op,
    reason = "zero multiplied by an unknown function must remain explicit"
)]
fn domain_edges_and_overflow_remain_explicit() {
    let x = Expr::symbol("dag_domain_x");
    assert!(matches!((&x / &x).view(), ExprView::Div(..)));
    assert!(matches!((0 * x.sin()).view(), ExprView::Product(_)));
    assert!(matches!(x.pow(0).view(), ExprView::Pow(..)));
    assert!(matches!(Expr::number(0.0).pow(0).view(), ExprView::Pow(..)));
    let largest = Expr::number(f64::MAX);
    assert!(matches!((&largest + &largest).view(), ExprView::Sum(terms) if terms.len() == 2));
    assert!(
        matches!((&largest * &largest).view(), ExprView::Product(factors) if factors.len() == 2)
    );
}

#[test]
fn deep_graph_operations_use_a_small_stack() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut expression = Expr::symbol("dag_deep_leaf");
            let context = Context::new();
            let function = context
                .register_function("deep", UserFunction::new(1..=1))
                .expect("the custom name is unbound");
            for _ in 0..20_000 {
                expression = function
                    .call(vec![expression])
                    .expect("the function is unary");
            }
            assert_eq!(expression.node_count(), 20_001);
            assert_eq!(expression.max_depth(), 20_001);
            assert!(
                expression
                    .map(|_| Ok::<_, ()>(None))
                    .expect("the identity transformation cannot fail")
                    .same_node(&expression)
            );
            assert!(format!("{expression}").ends_with(')'));
            let observer =
                Arc::downgrade(expression.0.as_ref().expect("a live handle owns its node"));
            drop(expression);
            assert!(observer.upgrade().is_none());
        })
        .expect("start small-stack test")
        .join()
        .expect("small-stack DAG operations succeeded");
}
