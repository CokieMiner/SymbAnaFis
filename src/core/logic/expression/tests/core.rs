//! Deterministic expression edges that need fixed identities, limits, or layout.

use core::{mem::size_of, num::NonZeroU32};

use std::{sync::Barrier, thread};

use alloc::{format, string::String, sync::Arc, vec, vec::Vec};

use crate::{
    collections::HashSet,
    core::{Context, Function, Symbol, SymbolData, UserFunction},
};

use super::super::{Expr, ExprView, Node};

#[test]
fn handle_is_one_pointer() {
    assert_eq!(size_of::<Expr>(), size_of::<usize>());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn node_payload_fits_six_native_words() {
    assert!(size_of::<super::super::Node>() <= size_of::<usize>().saturating_mul(6));
}

#[test]
fn literal_identity_keeps_float_bits_including_nan() {
    for value in [0.0, -0.0, f64::INFINITY, f64::NAN] {
        let first = Expr::number(value);
        let second = Expr::number(value);
        assert!(core::ptr::eq(first.pointer(), second.pointer()));
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
    let x = Symbol::intern("dag_concurrent_x").expect("symbol");
    let context = Context::new().expect("context");
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
    assert!(roots.iter().all(|root| root == first));
}

#[test]
#[expect(
    clippy::erasing_op,
    reason = "zero multiplied by an unknown function must remain explicit"
)]
fn domain_edges_and_overflow_remain_explicit() {
    let x = Expr::symbol("dag_domain_x").expect("symbol");
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
            let mut expression = Symbol::intern("dag_deep_leaf").expect("symbol").to_expr();
            let context = Context::new().expect("context");
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
            assert_eq!(
                expression
                    .map(|_| Ok::<_, ()>(None))
                    .expect("the identity transformation cannot fail"),
                expression
            );
            assert!(format!("{expression}").ends_with(')'));
            let observer = Arc::downgrade(&expression.0);
            drop(expression);
            assert!(observer.upgrade().is_none());
        })
        .expect("start small-stack test")
        .join()
        .expect("small-stack DAG operations succeeded");
}

#[test]
fn free_variables_name_anonymous_symbols_by_identity() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let anonymous = Symbol::anon().expect("anonymous symbol");
    let input = Expr::sum(vec![x.to_expr(), anonymous.to_expr()]);
    let mut excluded = HashSet::new();
    excluded.insert(String::from("x"));
    // No excluded name has the anonymous form, so the anonymous symbol is free.
    assert!(input.has_free_variables(&excluded));
    excluded.insert(format!("${}", anonymous.id()));
    assert!(!input.has_free_variables(&excluded));
    // A different anonymous name does not exclude this symbol.
    let mut other = HashSet::new();
    other.insert(String::from("x"));
    other.insert(format!("${}", anonymous.id().saturating_add(1)));
    assert!(input.has_free_variables(&other));
}

#[test]
fn is_constant_distinguishes_constant_subtrees_from_symbols_and_derivatives() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let anonymous = Symbol::anon().expect("anonymous symbol");
    let custom = context
        .register_function("const_fn", UserFunction::new(2..=2))
        .expect("custom function");

    assert!(Expr::number(0).is_constant());
    assert!(Expr::number(-42).is_constant());
    assert!(Expr::number(2.5).is_constant());
    assert!(Expr::i().is_constant());
    assert!(Expr::pi().is_constant());
    assert!(Expr::e().is_constant());
    assert!((Expr::pi().sin() + Expr::e().pow(2)).is_constant());
    assert!((Expr::pi().pow(2) + Expr::pi().pow(3)).is_constant());
    let opaque_const = Expr::pi() + Expr::e();
    assert!((opaque_const.pow(2) + opaque_const.pow(3)).is_constant());
    assert!(
        custom
            .call(vec![Expr::number(1), Expr::pi()])
            .expect("binary call")
            .is_constant()
    );

    assert!(!x.to_expr().is_constant());
    assert!(!anonymous.to_expr().is_constant());
    assert!(!(x + Expr::number(1)).is_constant());
    assert!(!(x.pow(2) + x.pow(3)).is_constant());
    assert!(!(x.sin().pow(2) + x.sin().pow(3)).is_constant());
    let one = NonZeroU32::new(1).expect("positive order");
    let two = NonZeroU32::new(2).expect("positive order");
    assert!(
        !SymbolData::of(x)
            .derivative_of(Expr::number(1), one)
            .is_constant()
    );
    assert!(
        !SymbolData::of(anonymous)
            .derivative_of(Expr::number(1), two)
            .is_constant()
    );
    assert!(!Expr::derivative(x.hermite(0.5), x, 1).is_constant());
    assert!(!(&x.to_expr() / &x.to_expr()).is_constant());
}

#[test]
fn functions_and_contains_function_track_distinct_identities() {
    let first_context = Context::new().expect("first context");
    let second_context = Context::new().expect("second context");
    let x = first_context.symb("x").expect("symbol");
    let y = first_context.symb("y").expect("symbol");

    let sin = Function::builtin("sin").expect("sin");
    let cos = Function::builtin("cos").expect("cos");
    let exp = Function::builtin("exp").expect("exp");
    let ln = Function::builtin("ln").expect("ln");
    let log = Function::builtin("log").expect("log");

    assert_eq!((x + y).functions(), HashSet::new());
    assert!(!(x + y).contains_function(&sin));

    let trig = x.sin() + y.cos() + x.sin();
    assert_eq!(trig.functions(), HashSet::from([sin.clone(), cos.clone()]));
    assert!(trig.contains_function(&sin));
    assert!(trig.contains_function(&cos));
    assert!(!trig.contains_function(&exp));

    let ln_expr = x.log(Expr::e());
    assert_eq!(ln_expr.functions(), HashSet::from([ln.clone()]));
    assert!(ln_expr.contains_function(&ln));
    assert!(!ln_expr.contains_function(&log));

    let opaque_poly = x.sin().pow(2) + x.sin().pow(3);
    assert!(matches!(opaque_poly.node(), Node::Poly(_)));
    assert_eq!(opaque_poly.functions(), HashSet::from([sin.clone()]));
    assert!(opaque_poly.contains_function(&sin));

    let f1 = first_context
        .register_function("f", UserFunction::new(1..=1))
        .expect("f1");
    let f2 = second_context
        .register_function("f", UserFunction::new(1..=1))
        .expect("f2");
    let c1 = f1.call(vec![x.to_expr()]).expect("unary");
    let c2 = f2.call(vec![y.to_expr()]).expect("unary");
    assert!(c1.contains_function(&f1));
    assert!(!c1.contains_function(&f2));
    let combined = c1 + c2;
    assert_eq!(
        combined.functions(),
        HashSet::from([f1.clone(), f2.clone()])
    );
    assert!(combined.contains_function(&f1));
    assert!(combined.contains_function(&f2));
}

#[test]
fn variables_ordered_sorts_deduplicates_and_matches_variables() {
    let first_context = Context::new().expect("first context");
    let second_context = Context::new().expect("second context");
    let z = first_context.symb("z").expect("z");
    let a = first_context.symb("a").expect("a");
    let m1 = first_context.symb("m").expect("m1");
    let m2 = second_context.symb("m").expect("m2");
    let anon = Symbol::anon().expect("anonymous symbol");
    let anon_name = format!("${}", anon.id());
    let shadowed_anon_name = first_context.symb(&anon_name).expect("named $id");

    assert_eq!(
        (Expr::pi() + Expr::e() + 1).variables_ordered(),
        Vec::<String>::new()
    );

    let expr = Expr::sum(vec![
        z.pow(2),
        z.pow(3),
        m1.to_expr(),
        m2.to_expr(),
        SymbolData::of(z).derivative_of(a.sin(), NonZeroU32::new(1).expect("positive order")),
        anon.to_expr(),
        shadowed_anon_name.to_expr(),
    ]);
    let ordered = expr.variables_ordered();
    let expected = vec![
        anon_name,
        String::from("a"),
        String::from("m"),
        String::from("z"),
    ];
    assert_eq!(ordered, expected);
    assert!(ordered.is_sorted());
    let from_set: HashSet<_> = ordered.into_iter().collect();
    assert_eq!(from_set, expr.variables());
}

#[test]
fn exact_identity_predicates_distinguish_exact_from_approximate_literals() {
    assert!(Expr::number(0).is_exact_zero());
    assert!(!Expr::number(0).is_exact_one());
    assert!(!Expr::number(0.0).is_exact_zero());
    assert!(!Expr::number(-0.0).is_exact_zero());
    assert!(Expr::number(0.0).is_zero_num());
    assert!(Expr::number(-0.0).is_zero_num());

    assert!(Expr::number(1).is_exact_one());
    assert!(!Expr::number(1).is_exact_zero());
    assert!(!Expr::number(1.0).is_exact_one());
    assert!(Expr::number(1.0).is_one_num());

    assert!(!Expr::i().is_exact_zero());
    assert!(!Expr::i().is_exact_one());
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x").to_expr();
    assert!(!x.is_exact_zero());
    assert!(!x.is_exact_one());
}
