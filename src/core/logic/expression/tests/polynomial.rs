//! Deterministic polynomial edges for domains, rewriting, rendering, limits,
//! and derivatives.

use core::{
    iter::once,
    ops::{Add, Div, Mul, Neg, Sub},
};

use std::{sync::Barrier, thread};

use alloc::{format, sync::Arc, vec, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::core::{Context, Substitution};

use super::super::{CompactStorage, ExactIdentity, Expr, ExprView, Node, StoredForm, Symbol};

/// The product \(c\,n \prod_{i} b_{i}^{k_{i}}\) of a coefficient, an integer
/// scale, and powers; an exact unit coefficient is omitted, as in a monomial's
/// product.
fn monomial_product(coefficient: &Expr, scale: u32, powers: &[(&Expr, u32)]) -> Expr {
    let mut factors = vec![Expr::number(i64::from(scale))];
    if !coefficient.is_exact_one() {
        factors.push(coefficient.clone());
    }
    factors.extend(
        powers
            .iter()
            .map(|(base, power)| base.pow(i64::from(*power))),
    );
    Expr::product(factors)
}

proptest! {
    #[test]
    fn multibase_storage_holds_each_base_once_in_structural_order(
        monomials in collection::vec((1_i64..=5, 0_u32..=3, 0_u32..=3, 0_u32..=2), 2..=8),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let opaque = context.symb("z")?.sin();
        let terms: Vec<_> = monomials
            .iter()
            .map(|&(coefficient, x_power, y_power, opaque_power)| {
                let mut factors = vec![Expr::number(coefficient)];
                for (base, power) in [(&x, x_power), (&y, y_power), (&opaque, opaque_power)] {
                    if power > 0 {
                        factors.push(base.pow(i64::from(power)));
                    }
                }
                Expr::product(factors)
            })
            .collect();
        let sum = Expr::sum(terms);
        let group = once(&sum)
            .chain(sum.stored_children())
            .find(|term| matches!(term.node(), Node::Poly(_)));
        prop_assume!(group.is_some());
        let Some(compact) = group else {
            return Ok(());
        };
        let Node::Poly(storage) = compact.node() else {
            return Ok(());
        };
        prop_assert!(storage
            .bases
            .iter()
            .zip(storage.bases.iter().skip(1))
            .all(|(left, right)| Node::compare(left, right).is_lt()));
        prop_assert_eq!(storage.lengths.iter().sum::<usize>(), storage.powers.len());
        let mut used = vec![false; storage.bases.len()];
        for (_, powers) in storage.terms() {
            prop_assert!(!powers.is_empty());
            prop_assert!(powers.iter().zip(powers.iter().skip(1)).all(|(left, right)| left.0 < right.0));
            for &(position, exponent) in powers {
                prop_assert!(exponent > 0);
                let Some(slot) = used.get_mut(position) else {
                    return Err(proptest::test_runner::TestCaseError::fail("a base position lies outside the table"));
                };
                *slot = true;
            }
        }
        prop_assert!(used.iter().all(|used_base| *used_base));
        prop_assert_eq!(
            compact.stored_children().count(),
            storage.bases.len().saturating_add(storage.coefficients.len())
        );
        prop_assert_eq!(&Expr::sum(compact.children().cloned().collect()), compact);
    }

    #[test]
    fn scalar_projections_equal_sorted_canonical_products(
        monomials in collection::vec((0_u8..6, -6_i64..=6, 0_u32..=4, 0_u32..=3, 0_u32..=2, 0_u32..=1), 1..=10),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let pi = Expr::pi();
        let e = Expr::e();
        let sum = Expr::sum(
            monomials
                .iter()
                .map(|&(kind, value, x_power, y_power, pi_power, e_power)| {
                    let mut factors = vec![match kind {
                        0 => Expr::number(value),
                        1 => Expr::number(f64::from(i32::try_from(value).unwrap_or(0)) * 0.5),
                        2 => Expr::number(1),
                        3 => Expr::number(1.0),
                        4 => Expr::i(),
                        _ => Expr::number(value).div(Expr::number(7)),
                    }];
                    for (base, power) in [(&x, x_power), (&y, y_power), (&pi, pi_power), (&e, e_power)] {
                        if power > 0 {
                            factors.push(base.pow(i64::from(power)));
                        }
                    }
                    Expr::product(factors)
                })
                .collect(),
        );
        let group = once(&sum)
            .chain(sum.stored_children())
            .find(|term| matches!(term.node(), Node::Poly(_)));
        let Some(compact) = group else {
            return Ok(());
        };
        let Node::Poly(storage) = compact.node() else {
            return Ok(());
        };
        prop_assume!(storage.simple_bases);
        let mut expected: Vec<_> = storage
            .monomials()
            .map(|(coefficient, powers)| {
                Expr::product(Node::monomial_factors(coefficient.clone(), powers))
            })
            .collect();
        Node::sort_operands(&mut expected);
        prop_assert_eq!(compact.children().cloned().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn derivatives_equal_canonical_sums_of_lowered_products(
        monomials in collection::vec((0_u8..4, 1_i64..=6, 0_u32..=4, 0_u32..=3, 0_u32..=2), 1..=8),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let pi = Expr::pi();
        let polynomial = Expr::sum(
            monomials
                .iter()
                .map(|&(kind, value, x_power, y_power, pi_power)| {
                    let mut factors = vec![match kind {
                        0 => Expr::number(value),
                        1 => Expr::number(f64::from(i32::try_from(value).unwrap_or(0)) * 0.5),
                        2 => Expr::number(1),
                        _ => Expr::number(1.0),
                    }];
                    // A zeroth power keeps its base's zero exclusion, so only
                    // positive powers enter.
                    for (base, power) in [(&x, x_power), (&y, y_power), (&pi, pi_power)] {
                        if power > 0 {
                            factors.push(base.pow(i64::from(power)));
                        }
                    }
                    Expr::product(factors)
                })
                .collect(),
        );
        let Node::Poly(storage) = polynomial.node() else {
            prop_assert_eq!(polynomial.polynomial_derivative(&x), None);
            return Ok(());
        };
        for variable in [&x, &y] {
            let expected = Expr::sum(
                storage
                    .monomials()
                    .filter_map(|(coefficient, powers)| {
                        let (_, scale) = powers.clone().find(|(base, _)| *base == variable)?;
                        let lowered: Vec<_> = powers
                            .filter_map(|(base, power)| {
                                if base == variable {
                                    (power > 1).then(|| (base, power.saturating_sub(1)))
                                } else {
                                    Some((base, power))
                                }
                            })
                            .collect();
                        Some(monomial_product(coefficient, scale, &lowered))
                    })
                    .collect(),
            );
            prop_assert_eq!(polynomial.polynomial_derivative(variable), Some(expected));
        }
    }

    #[test]
    fn monomials_own_each_base_without_a_redundant_union(
        mut exponents in collection::vec(2_i64..=16, 2..=8),
        kind in 0_usize..3,
    ) {
        exponents.sort_unstable();
        exponents.dedup();
        prop_assume!(exponents.len() >= 2);
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = match kind {
            0 => x.to_expr(),
            1 => x.sin(),
            _ => Expr::sum(alloc::vec![x.into(), y.into()]),
        };
        let terms: Vec<_> = exponents.iter().map(|power| base.pow(*power)).collect();
        let owner = &*base.0;
        let owners = Arc::strong_count(owner);
        let observer = Arc::downgrade(owner);
        let polynomial = Expr::sum(terms.clone());
        let Node::Poly(storage) = polynomial.node() else {
            return Err(proptest::test_runner::TestCaseError::fail("distinct positive powers select compact storage"));
        };
        prop_assert_eq!(storage.coefficients.len(), terms.len());
        // The base is stored once: the edges are that base and one coefficient
        // per term, and the storage holds one handle to the base.
        prop_assert_eq!(polynomial.stored_children().count(), terms.len().saturating_add(1));
        prop_assert_eq!(Arc::strong_count(owner), owners.saturating_add(1));
        prop_assert!(storage.monomials().all(|(_, mut powers)|
            matches!((powers.next(), powers.next()), (Some((stored, _)), None) if stored == &base)));
        prop_assert_eq!(&Expr::sum(polynomial.children().cloned().collect()), &polynomial);
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
    let x = Symbol::intern("poly_coefficient_x").expect("symbol");
    let rational = Expr::number(1) / 3;
    let expression = &rational * x.pow(2) + &rational * x.pow(3);
    assert!(matches!(expression.node(), Node::Poly(polynomial)
        if polynomial.coefficients.iter().all(|coefficient| *coefficient == rational)));
    assert_eq!(
        Substitution::new(x.into(), Expr::number(2)).apply_all(&expression),
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
    let x = Expr::symbol("poly_concurrent_x").expect("symbol");
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
    let observer = Arc::downgrade(&first.0);
    drop(roots);
    assert!(observer.upgrade().is_none());
}

#[test]
fn concurrent_first_projections_publish_one_term_list() {
    let x = Expr::symbol("poly_projection_x").expect("symbol");
    let root = Arc::new(x.pow(2) + x.pow(3) + x.pow(5));
    let barrier = Arc::new(Barrier::new(4));
    #[expect(
        clippy::needless_collect,
        reason = "all workers must reach the barrier before joining"
    )]
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let shared = Arc::clone(&barrier);
            let polynomial = Arc::clone(&root);
            thread::spawn(move || {
                shared.wait();
                polynomial
                    .children()
                    .map(|term| Arc::as_ptr(&term.0).addr())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let projections: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("projection worker"))
        .collect();
    let first = projections.first().expect("projected roots");
    assert_eq!(first.len(), 3);
    assert!(projections.iter().all(|terms| terms == first));
    let observer = Arc::downgrade(&root.children().next().expect("a projected term").0);
    assert!(observer.upgrade().is_some());
    drop(root);
    assert!(observer.upgrade().is_none());
}

#[test]
fn deep_opaque_polynomial_bases_walk_iteratively() {
    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut base = Symbol::intern("poly_deep_leaf").expect("symbol").to_expr();
            let context = crate::core::Context::new().expect("context");
            let function = context
                .register_function("deep", crate::core::UserFunction::new(1..=1))
                .expect("the custom name is unbound");
            for _ in 0..10_000 {
                base = function
                    .call(alloc::vec![base])
                    .expect("the function is unary");
            }
            let polynomial = base.pow(2) + base.pow(3);
            let observer = Arc::downgrade(&base.0);
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

#[test]
fn product_slices_match_canonical_sum_construction() {
    let x = Symbol::intern("sum_of_products_x")
        .expect("symbol")
        .to_expr();
    let y = Symbol::intern("sum_of_products_y")
        .expect("symbol")
        .to_expr();
    let z = Symbol::intern("sum_of_products_z")
        .expect("symbol")
        .to_expr();
    let products = vec![
        vec![Expr::number(-48), x.clone(), y.clone(), z.pow(2)],
        vec![Expr::number(3), x.pow(2), y.clone(), z],
        vec![x.clone(), y.pow(3)],
        vec![Expr::number(7), x.pow(4), y],
    ];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    let direct = Expr::sum_of_products(products);
    assert_eq!(direct, expected);
    assert!(matches!(direct.node(), Node::Poly(_)));
}

#[test]
fn product_slices_flatten_bare_additive_terms() {
    let x = Symbol::intern("sum_of_products_bare_x")
        .expect("symbol")
        .to_expr();
    let y = Symbol::intern("sum_of_products_bare_y")
        .expect("symbol")
        .to_expr();
    let base = Expr::sum(vec![x.clone(), y]);
    let products = vec![
        vec![x.pow(3)],
        vec![x.pow(2)],
        vec![base.clone()],
        vec![base.pow(2)],
    ];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    assert_eq!(Expr::sum_of_products(products), expected);
    assert!(
        Node::try_monomial_polynomial(&[x.pow(3), x.pow(2), base.clone(), base.pow(2)]).is_none()
    );
}

#[test]
fn product_slices_fall_back_for_duplicates_and_literals() {
    let x = Symbol::intern("sum_of_products_fallback_x")
        .expect("symbol")
        .to_expr();
    for products in [
        vec![
            vec![Expr::number(2), x.pow(2)],
            vec![Expr::number(3), x.pow(2)],
            vec![x.pow(3)],
        ],
        vec![
            vec![Expr::number(2)],
            vec![Expr::number(3)],
            vec![x.pow(2)],
            vec![x.pow(3)],
        ],
        vec![
            vec![Expr::number(2), Expr::product(vec![x.clone(), x.clone()])],
            vec![x.pow(3)],
        ],
    ] {
        let expected = Expr::sum(
            products
                .iter()
                .cloned()
                .map(Expr::product)
                .collect::<Vec<_>>(),
        );
        assert_eq!(Expr::sum_of_products(products), expected);
    }
}

#[test]
fn order_keys_continue_past_the_inline_prefix() {
    let bases: Vec<_> = (0..10)
        .map(|index| Symbol::intern(format!("sum_order_base_{index}")).expect("symbol"))
        .collect();
    let make = |exponents: [i64; 10]| {
        bases
            .iter()
            .zip(exponents)
            .map(|(base, exponent)| base.pow(exponent))
            .collect::<Vec<_>>()
    };
    let products = vec![
        make([1, 1, 1, 1, 1, 1, 1, 1, 2, 3]),
        make([1, 1, 1, 1, 1, 1, 1, 1, 2, 4]),
    ];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    let direct = Expr::sum_of_products(products);
    assert_eq!(direct, expected);
    assert!(matches!(direct.node(), Node::Poly(_)));
}

#[test]
fn order_keys_fall_back_beyond_the_compact_prefix() {
    let bases: Vec<_> = (0..11)
        .map(|index| Symbol::intern(format!("sum_order_long_base_{index}")).expect("symbol"))
        .collect();
    let make = |last: i64| {
        let mut exponents = [1; 11];
        exponents[0] = 2;
        exponents[10] = last;
        bases
            .iter()
            .zip(exponents)
            .map(|(base, exponent)| base.pow(exponent))
            .collect::<Vec<_>>()
    };
    let products = vec![make(3), make(4)];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    let direct = Expr::sum_of_products(products);
    assert_eq!(direct, expected);
    assert!(matches!(direct.node(), Node::Poly(_)));
}

#[test]
fn order_keys_fall_back_for_large_exponents() {
    let x = Symbol::intern("sum_order_large_exponent_x").expect("symbol");
    let products = vec![vec![x.pow(64), x.pow(2)], vec![x.pow(65), x.pow(2)]];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    let direct = Expr::sum_of_products(products);
    assert_eq!(direct, expected);
    assert!(matches!(direct.node(), Node::Poly(_)));
}

#[test]
fn product_slices_flatten_unit_coefficient_sums() {
    let x = Symbol::intern("sum_of_products_unit_x")
        .expect("symbol")
        .to_expr();
    let y = Symbol::intern("sum_of_products_unit_y")
        .expect("symbol")
        .to_expr();
    let base = Expr::sum(vec![x.clone(), y]);
    let products = vec![
        vec![Expr::number(1), base.clone()],
        vec![base.pow(2)],
        vec![x.pow(2)],
        vec![x.pow(3)],
    ];
    let expected = Expr::sum(
        products
            .iter()
            .cloned()
            .map(Expr::product)
            .collect::<Vec<_>>(),
    );
    assert_eq!(Expr::sum_of_products(products), expected);
}

#[test]
fn rendering_omits_only_exact_unit_coefficients() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    assert_eq!(format!("{}", Sub::sub(x, y)), "x - y");
    assert_eq!(format!("{}", Sub::sub(x, Mul::mul(1.0, y))), "x - 1*y");
    assert_eq!(
        format!("{}", Add::add(x.pow(2), Mul::mul(1.0, y))),
        "1*y + x^2"
    );
    assert_eq!(format!("{}", Neg::neg(x)), "-x");
    assert_eq!(format!("{}", Mul::mul(-1.0, x)), "-1*x");
    // A compact group after another term takes the sign of its leading monomial.
    let shifted = Add::add(Sub::sub(x.pow(2), Mul::mul(2, x)), 1);
    assert_eq!(format!("{shifted}"), "1 - 2*x + x^2");
}

#[test]
fn derivative_fallbacks_keep_constant_terms_and_overflowing_scales() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol").to_expr();
    let y = context.symb("y").expect("symbol").to_expr();
    // The linear term leaves a constant, which compact storage never holds.
    let quadratic = Expr::sum(vec![
        x.pow(2),
        Expr::product(vec![Expr::number(2), x.clone()]),
    ]);
    assert!(matches!(quadratic.node(), Node::Poly(_)));
    assert_eq!(
        quadratic.polynomial_derivative(&x),
        Some(Expr::sum(vec![
            Expr::product(vec![Expr::number(2), x.clone()]),
            Expr::number(2),
        ]))
    );
    // An exact scale that overflows keeps the coefficient and the scale.
    let wide = Expr::sum(vec![
        Expr::product(vec![Expr::number(i64::MAX), x.pow(2)]),
        x.pow(3),
    ]);
    assert!(matches!(wide.node(), Node::Poly(_)));
    let one = Expr::number(1);
    assert_eq!(
        wide.polynomial_derivative(&x),
        Some(Expr::sum(vec![
            monomial_product(&Expr::number(i64::MAX), 2, &[(&x, 1)]),
            monomial_product(&one, 3, &[(&x, 2)]),
        ]))
    );
    assert_eq!(wide.polynomial_derivative(&y), Some(Expr::number(0)));
}

#[test]
fn derivatives_need_compact_storage_in_scalar_bases() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let opaque = Expr::sum(vec![x.sin().pow(2), x.sin().pow(3)]);
    assert!(matches!(opaque.node(), Node::Poly(_)));
    assert_eq!(opaque.polynomial_derivative(&x.to_expr()), None);
    assert_eq!(x.pow(2).polynomial_derivative(&x.to_expr()), None);
}

#[test]
fn partial_derivatives_keep_uniform_monomials_beside_compact_storage() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol").to_expr();
    let z = context.symb("z").expect("symbol").to_expr();
    let cubic = Expr::sum(vec![
        x.pow(3),
        x.pow(2),
        Expr::product(vec![x.clone(), z.clone()]),
    ]);
    assert!(matches!(cubic.node(), Node::Poly(_)));
    let derivative = cubic
        .polynomial_derivative(&x)
        .expect("compact storage in symbols has a derivative");
    // `z` has one power, so its monomial stays beside the stored 3x^2 + 2x.
    assert!(
        matches!(derivative.node(), Node::Sum(terms) if terms.len() == 2
        && Node::compact_group(terms).is_some())
    );
    assert_eq!(
        derivative,
        Expr::sum(vec![
            Expr::product(vec![Expr::number(3), x.pow(2)]),
            Expr::product(vec![Expr::number(2), x]),
            z,
        ])
    );
}
