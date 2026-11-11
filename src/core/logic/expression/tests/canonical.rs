//! Canonical sums and products are independent of operand order, grouping,
//! and compact storage, and direct constructors agree with them.

use core::ops::Div;

use alloc::{vec, vec::Vec};

use proptest::{bool, collection, prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::core::Context;

use super::super::{Expr, Node, Symbol};

/// A term for like-term grouping: symbolic products with exact, approximate,
/// zero, and non-finite coefficients, plain bases, functions, and literals.
fn term(kind: u8, coefficient: i64, power: i64, a: Symbol<'_>, b: Symbol<'_>) -> Expr {
    match kind {
        0 => a.to_expr(),
        1 => Expr::product(vec![a.to_expr(), b.to_expr()]),
        2 => Expr::product(vec![Expr::number(coefficient), a.to_expr(), b.to_expr()]),
        3 => Expr::product(vec![Expr::number(coefficient), a.pow(power)]),
        4 => Expr::product(vec![Expr::number(0), b.pow(power)]),
        5 => Expr::product(vec![Expr::number(f64::NAN), a.to_expr()]),
        6 => Expr::product(vec![Expr::number(0.5), a.to_expr(), b.to_expr()]),
        7 => Expr::number(coefficient),
        8 => a.sin(),
        _ => Expr::product(vec![Expr::number(coefficient), b.to_expr(), a.sin()]),
    }
}

/// A factor for product grouping: powers, literals, functions, quotients, and
/// the imaginary unit.
fn factor(kind: u8, coefficient: i64, power: i64, a: Symbol<'_>, b: Symbol<'_>) -> Expr {
    match kind {
        0 => a.to_expr(),
        1 => a.pow(power),
        2 => b.pow(power),
        3 => Expr::number(coefficient),
        4 => a.sin().pow(power),
        5 => Div::div(1, b.to_expr()),
        _ => Expr::i(),
    }
}

proptest! {
    #[test]
    fn sums_do_not_depend_on_operand_order_or_grouping(
        shapes in collection::vec((0_u8..10, -4_i64..=4, 1_i64..=4), 1..=8),
        split in 0_usize..=8,
    ) {
        let context = Context::new()?;
        let a = context.symb("a")?;
        let b = context.symb("b")?;
        let terms: Vec<Expr> = shapes
            .iter()
            .map(|&(kind, coefficient, power)| term(kind, coefficient, power, a, b))
            .collect();
        let sum = Expr::sum(terms.clone());
        let mut reversed = terms.clone();
        reversed.reverse();
        prop_assert_eq!(&sum, &Expr::sum(reversed));
        let (left, right) = terms.split_at(split.min(terms.len()));
        prop_assert_eq!(&sum, &Expr::sum(vec![Expr::sum(left.to_vec()), Expr::sum(right.to_vec())]));
    }

    #[test]
    fn products_do_not_depend_on_factor_order_or_grouping(
        shapes in collection::vec((0_u8..7, -3_i64..=3, 1_i64..=4), 1..=7),
        split in 0_usize..=7,
    ) {
        let context = Context::new()?;
        let a = context.symb("a")?;
        let b = context.symb("b")?;
        let factors: Vec<Expr> = shapes
            .iter()
            .map(|&(kind, coefficient, power)| factor(kind, coefficient, power, a, b))
            .collect();
        let product = Expr::product(factors.clone());
        let mut reversed = factors.clone();
        reversed.reverse();
        prop_assert_eq!(&product, &Expr::product(reversed));
        let (left, right) = factors.split_at(split.min(factors.len()));
        prop_assert_eq!(
            &product,
            &Expr::product(vec![Expr::product(left.to_vec()), Expr::product(right.to_vec())])
        );
    }

    #[test]
    fn approximate_literals_keep_their_tier_beside_symbols(bits in proptest::num::f64::NORMAL | proptest::num::f64::ZERO) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let literal = Expr::number(bits);
        // Only exact identities vanish, so the literal survives with its bits.
        let product = Expr::product(vec![literal.clone(), x.to_expr()]);
        prop_assert!(matches!(product.node(), Node::Product(factors) if factors.contains(&literal)));
        let sum = Expr::sum(vec![x.to_expr(), literal.clone()]);
        prop_assert!(matches!(sum.node(), Node::Sum(terms) if terms.contains(&literal)));
    }

    #[test]
    fn positive_powers_of_one_base_collect(
        powers in collection::vec(1_i64..=1_000, 2..=6),
        other in 1_i64..=5,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let mut factors: Vec<Expr> = powers.iter().map(|power| x.pow(*power)).collect();
        factors.push(y.pow(other));
        let total: i64 = powers.iter().sum();
        prop_assert_eq!(Expr::product(factors), Expr::product(vec![x.pow(total), y.pow(other)]));
    }

    #[test]
    fn sums_with_compact_groups_equal_sums_of_their_logical_terms(
        monomials in collection::vec((1_i64..=5, bool::ANY, 0_i64..=3), 2..=6),
        others in collection::vec((0_u8..9, -3_i64..=3, 1_i64..=4), 0..=5),
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let w = context.symb("w")?;
        // Distinct powers of `x` make every monomial distinct and compact.
        let group = Expr::sum(
            monomials
                .iter()
                .zip(1_i64..)
                .map(|(&(magnitude, negative, y_power), x_power)| {
                    let coefficient = if negative { magnitude.saturating_neg() } else { magnitude };
                    Expr::product(vec![Expr::number(coefficient), x.pow(x_power), y.pow(y_power)])
                })
                .collect(),
        );
        prop_assume!(matches!(group.node(), Node::Poly(_)));
        let terms: Vec<Expr> = others
            .iter()
            .map(|&(kind, coefficient, power)| match kind {
                0 => w.to_expr(),
                1 => Expr::product(vec![Expr::number(coefficient), w.pow(power)]),
                2 => x.pow(power),
                3 => Expr::product(vec![Expr::number(coefficient), x.pow(power), y.to_expr()]),
                4 => x.sin(),
                5 => Expr::number(coefficient),
                6 => Expr::product(vec![Expr::number(0), y.pow(power)]),
                7 => Expr::product(vec![Expr::number(coefficient), w.to_expr(), y.cos()]),
                _ => Expr::product(vec![Expr::number(coefficient), w.pow(power), x.sin().pow(2)]),
            })
            .collect();
        let mut compact_terms = vec![group.clone()];
        compact_terms.extend(terms.iter().cloned());
        let compact = Expr::sum(compact_terms);
        let mut logical_terms: Vec<Expr> = group.children().cloned().collect();
        logical_terms.extend(terms.iter().cloned());
        prop_assert_eq!(&compact, &Expr::sum(logical_terms));
        prop_assert_eq!(&compact, &Expr::sum(vec![group, Expr::sum(terms)]));
    }
}

#[test]
fn compact_groups_absorb_zero_literals_and_merge_like_monomials() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let group = Expr::sum(vec![x.pow(2), x.pow(3)]);
    assert!(matches!(group.node(), Node::Poly(_)));
    assert_eq!(Expr::sum(vec![group.clone(), Expr::number(0)]), group);
    // A like monomial outside the group combines with it.
    assert_eq!(
        Expr::sum(vec![group, x.pow(2)]),
        Expr::sum(vec![
            Expr::product(vec![Expr::number(2), x.pow(2)]),
            x.pow(3)
        ])
    );
}

#[test]
fn like_terms_with_symbolic_factors_combine_without_a_coefficient_on_the_first() {
    let context = Context::new().expect("context");
    let a = context.symb("a").expect("symbol");
    let b = context.symb("b").expect("symbol");
    let plain = Expr::product(vec![a.to_expr(), b.to_expr()]);
    let scaled = Expr::product(vec![Expr::number(3), a.to_expr(), b.to_expr()]);
    assert_eq!(
        Expr::sum(vec![plain, scaled]),
        Expr::product(vec![Expr::number(4), a.to_expr(), b.to_expr()])
    );
}

#[test]
fn overflowing_power_counts_stay_uncollected() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let product = Expr::product(vec![x.pow(i64::MAX), x.to_expr()]);
    assert!(matches!(product.node(), Node::Product(factors) if factors.len() == 2));
}

#[test]
fn only_exact_identities_vanish() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol").to_expr();
    assert_eq!(Expr::product(vec![Expr::number(1), x.clone()]), x);
    assert_eq!(Expr::sum(vec![x.clone(), Expr::number(0)]), x);
    assert_eq!(x.pow(1), x);
    assert_eq!(Div::div(x.clone(), 1), x);
    for unit in [Expr::number(1.0), Expr::number(-0.0), Expr::number(0.0)] {
        assert_ne!(Expr::product(vec![unit.clone(), x.clone()]), x);
        assert_ne!(Expr::sum(vec![x.clone(), unit]), x);
    }
    assert_ne!(x.pow(1.0), x);
    assert_ne!(Div::div(x.clone(), 1.0), x);
}
