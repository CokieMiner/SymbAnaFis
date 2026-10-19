//! Deterministic fast-path edges for numeric prefixes and symbol identity.

use core::ops::{Add, Div};

use alloc::{vec, vec::Vec};

use proptest::{collection, prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::core::{Substitution, Substitutions};

use super::super::{Expr, ExprView, Node, Symbol};

proptest! {
    #[test]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "twenty coefficients bounded by eight and six values bounded by three fit i64"
    )]
    fn atomic_terms_and_weighted_groups_preserve_permutations_and_numeric_values(
        terms in collection::vec((0_usize..6, -8_i64..=8), 0..=20),
        values in collection::vec(-3_i64..=3, 6),
        constant in -8_i64..=8,
    ) {
        let x = Expr::symbol("atomic_term_x");
        let y = Symbol::intern("atomic_term_y");
        let atoms = [x.clone(), x.sin(), Expr::pi(), Expr::e(), &x / y, Expr::derivative(y.abs(), y, 1)];
        let mut operands: Vec<_> = terms.iter().map(|(index, coefficient)| {
            Expr::product(vec![Expr::number(*coefficient), atoms.get(*index)
                .expect("indices are bounded by six").clone()])
        }).collect();
        operands.push(Expr::number(constant));
        let weighted = Expr::sum(operands.clone());
        operands.reverse();
        prop_assert_eq!(&weighted, &Expr::sum(operands));

        let mut unweighted: Vec<_> = terms.iter().map(|(index, _)| atoms.get(*index)
            .expect("indices are bounded by six").clone()).collect();
        let unit_sum = Expr::sum(unweighted.clone());
        unweighted.reverse();
        prop_assert_eq!(unit_sum, Expr::sum(unweighted));

        let substitutions = Substitutions::new(atoms.into_iter().zip(values.iter().map(|value| Expr::number(*value))))
            .expect("all six sources are distinct");
        let expected = terms.iter().fold(constant, |total, (index, coefficient)| {
            total + coefficient * values.get(*index).expect("indices are bounded by six")
        });
        prop_assert_eq!(substitutions.apply_all(&weighted), Expr::number(expected));
    }

    #[test]
    fn singleton_float_weights_preserve_their_literal_handles(
        bits in proptest::num::u64::ANY,
        kind in 0_usize..7,
    ) {
        let value = f64::from_bits(bits);
        prop_assume!(value.is_finite());
        let coefficient = Expr::number(value);
        let x = Expr::symbol("singleton_weight_x");
        let y = Symbol::intern("singleton_weight_y");
        let base = match kind {
            0 => x,
            1 => x.sin(),
            2 => x.pow(3),
            3 => Div::div(&x, y),
            4 => Expr::derivative(y.abs(), y, 1),
            5 => Add::add(&x, y),
            _ => Add::add(x.pow(2), x.pow(3)),
        };
        let product = Expr::product(vec![coefficient.clone(), base.clone()]);
        if coefficient.is_one_num() {
            prop_assert_eq!(&product, &base);
        } else {
            prop_assert!(matches!(product.view(), ExprView::Product([stored, factor])
                if stored == &coefficient && factor == &base));
        }
        prop_assert_eq!(&product, &Expr::product(vec![base.clone(), coefficient.clone()]));
        let sum = Expr::sum(vec![coefficient.clone(), base.clone()]);
        if coefficient.is_zero_num() {
            prop_assert_eq!(&sum, &base);
        } else {
            prop_assert!(sum.fold(false, |found, node| found || node == &coefficient));
        }
        prop_assert_eq!(sum, Expr::sum(vec![base, coefficient]));
    }

    #[test]
    fn binary_collection_agrees_with_grouped_collection(
        first in -8_i64..=8,
        second in -8_i64..=8,
        kind in 0_usize..7,
    ) {
        let x = Expr::symbol("binary_collection_x");
        let y = Symbol::intern("binary_collection_y");
        let z = Expr::symbol("binary_collection_z");
        let base = match kind {
            0 => x,
            1 => x.sin(),
            2 => x.pow(3),
            3 => Div::div(&x, y),
            4 => Expr::derivative(y.abs(), y, 1),
            5 => Expr::product(vec![x, y.to_expr()]),
            _ => Expr::number(f64::INFINITY),
        };
        let mut terms = vec![
            Expr::product(vec![Expr::number(first), base.clone()]),
            Expr::product(vec![Expr::number(second), base.clone()]),
        ];
        let binary = Expr::sum(terms.clone());
        terms.push(z.clone());
        let grouped = Expr::sum(terms);
        let expected = Expr::product(vec![Expr::number(first.saturating_add(second)), base]);
        prop_assert_eq!(&binary, &expected);
        prop_assert_eq!(Substitution::new(z, Expr::number(0)).apply_all(&grouped), expected);
    }
}

#[test]
fn singleton_literal_normalization_keeps_float_bits() {
    for expression in [Expr::number(-0.0), Expr::number(f64::NAN)] {
        assert_eq!(Expr::sum(vec![expression.clone()]), expression);
        assert_eq!(Expr::product(vec![expression.clone()]), expression);
    }
}

#[test]
fn numeric_prefix_keeps_nonfinite_literals_and_symbolic_tails() {
    let x = Expr::symbol("prefix_tail");
    let y = Expr::symbol("prefix_tail_y");
    let operands = vec![
        Expr::number(2),
        Expr::number(3),
        Expr::i(),
        Expr::number(f64::INFINITY),
        Expr::number(f64::from_bits(0x7ff8_0000_0000_0001)),
        Expr::pi(),
        Expr::e(),
        x.clone(),
    ];
    let sum = Expr::sum(operands.clone());
    let product = Expr::product(operands.clone());
    for rotation in 0..operands.len() {
        let mut permutation = operands.clone();
        permutation.rotate_left(rotation);
        assert_eq!(Expr::sum(permutation.clone()), sum);
        assert_eq!(Expr::product(permutation), product);
    }
    assert!(product.contains_var("prefix_tail"));
    assert!(
        matches!(Expr::sum(vec![Expr::number(f64::INFINITY), x, y]).view(), ExprView::Sum(terms) if terms.len() == 3)
    );
}

#[test]
fn failed_numeric_prefix_folds_retain_every_original_literal() {
    let x = Expr::symbol("failed_prefix_x");
    for (first, second, sum) in [
        (f64::MAX, f64::MAX, true),
        (f64::MAX, 2.0, false),
        (f64::from_bits(1), 0.5, false),
    ] {
        let operands = vec![
            Expr::number(first),
            Expr::number(second),
            Expr::number(f64::INFINITY),
            Expr::number(f64::from_bits(0x7ff8_0000_0000_0001)),
            x.clone(),
        ];
        let result = if sum {
            Expr::sum(operands.clone())
        } else {
            Expr::product(operands.clone())
        };
        let stored: Vec<_> = Node::children(&result).cloned().collect();
        assert_eq!(stored.len(), operands.len());
        for operand in &operands {
            assert_eq!(
                stored.iter().filter(|value| *value == operand).count(),
                operands.iter().filter(|value| *value == operand).count(),
            );
        }
        for rotation in 0..operands.len() {
            let mut permutation = operands.clone();
            permutation.rotate_left(rotation);
            assert_eq!(
                if sum {
                    Expr::sum(permutation)
                } else {
                    Expr::product(permutation)
                },
                result,
            );
        }
    }
}
