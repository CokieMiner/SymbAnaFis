//! Deterministic polynomial edges for domains, rewriting, rendering, and limits.

use std::{sync::Barrier, thread};

use alloc::{format, sync::Arc, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::core::Context;

use super::super::{Expr, ExprView, Node, Symbol};

proptest! {
    #[test]
    fn monomials_own_each_base_without_a_redundant_union(
        mut exponents in collection::vec(2_i64..=16, 2..=8),
        kind in 0_usize..3,
    ) {
        exponents.sort_unstable();
        exponents.dedup();
        prop_assume!(exponents.len() >= 2);
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = match kind {
            0 => x.to_expr(),
            1 => x.sin(),
            _ => Expr::sum(alloc::vec![x.into(), y.into()]),
        };
        let terms: Vec<_> = exponents.iter().map(|power| base.pow(*power)).collect();
        let owner = base.0.as_ref().expect("a live base owns its node");
        let owners = Arc::strong_count(owner);
        let observer = Arc::downgrade(owner);
        let polynomial = Expr::sum(terms.clone());
        let Node::Poly(storage) = polynomial.node() else {
            return Err(proptest::test_runner::TestCaseError::fail("distinct positive powers select compact storage"));
        };
        prop_assert_eq!(storage.terms.len(), terms.len());
        prop_assert_eq!(storage.children().count(), terms.len().saturating_mul(2));
        prop_assert_eq!(Arc::strong_count(owner), owners.saturating_add(terms.len()));
        prop_assert!(storage.terms.iter().all(|term|
            matches!(term.powers.as_slice(), [(stored, _)] if stored == &base)));
        prop_assert_eq!(&Expr::sum(polynomial.children().collect()), &polynomial);
        drop(polynomial);
        prop_assert_eq!(Arc::strong_count(owner), owners);
        drop(terms);
        prop_assert_eq!(Arc::strong_count(owner), 1);
        drop(base);
        prop_assert!(observer.upgrade().is_none());
    }
}

#[test]
fn exact_coefficients_and_nonfinite_terms_keep_sparse_domain_boundaries() {
    let x = Symbol::intern("poly_coefficient_x");
    let rational = Expr::number(1) / 3;
    let expression = &rational * x.pow(2) + &rational * x.pow(3);
    assert!(matches!(expression.node(), Node::Poly(polynomial)
        if polynomial.terms.iter().all(|term| term.coefficient == rational)));
    assert_eq!(
        expression.substitute_symbol(x, &Expr::number(2)),
        Expr::number(4)
    );
    let undefined = Expr::number(0).pow(0);
    let nonfinite = Expr::number(f64::INFINITY) * x.pow(2) + x.pow(3);
    assert!(matches!(nonfinite.view(), ExprView::Sum(_)));
    assert_eq!(undefined, Expr::number(0).pow(0));
    let cancelled = x.pow(2) + x.pow(3) - x.pow(2);
    assert!(cancelled.children().any(|term| {
        matches!(term.view(), ExprView::Product(factors)
            if factors.first().is_some_and(Expr::is_zero_num))
    }));
}

#[test]
fn concurrent_polynomial_construction_reuses_one_live_node() {
    let x = Expr::symbol("poly_concurrent_x");
    let barrier = Arc::new(Barrier::new(5));
    #[expect(
        clippy::needless_collect,
        reason = "all workers must reach the barrier before joining"
    )]
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let shared = Arc::clone(&barrier);
            let base = x.clone();
            thread::spawn(move || {
                shared.wait();
                base.pow(2) + base.pow(3)
            })
        })
        .collect();
    barrier.wait();
    let roots: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("polynomial worker"))
        .collect();
    let first = roots.first().expect("constructed roots");
    assert!(roots.iter().all(|root| first == root));
    let observer = Arc::downgrade(first.0.as_ref().expect("a live handle owns its node"));
    drop(roots);
    assert!(observer.upgrade().is_none());
}

#[test]
fn deep_opaque_polynomial_bases_walk_iteratively() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut base = Expr::symbol("poly_deep_leaf");
            let context = crate::core::Context::new();
            let function = context
                .register_function("deep", crate::core::UserFunction::new(1..=1))
                .expect("the custom name is unbound");
            for _ in 0..10_000 {
                base = function
                    .call(alloc::vec![base])
                    .expect("the function is unary");
            }
            let polynomial = base.pow(2) + base.pow(3);
            let observer = Arc::downgrade(base.0.as_ref().expect("a live handle owns its node"));
            assert_eq!(polynomial.max_depth(), 10_002);
            assert_eq!(
                polynomial
                    .map(|_| Ok::<_, ()>(None))
                    .expect("the reference traversal cannot fail"),
                polynomial
            );
            assert!(format!("{polynomial}").ends_with("^3"));
            drop(base);
            drop(polynomial);
            assert!(observer.upgrade().is_none());
        })
        .expect("small-stack polynomial worker")
        .join()
        .expect("iterative polynomial operations");
}
