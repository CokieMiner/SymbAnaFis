//! Deterministic fast-path edges for numeric prefixes and symbol identity.

use alloc::vec;

use super::super::{Expr, ExprView};

#[test]
fn singleton_literal_normalization_keeps_float_bits() {
    for expression in [Expr::number(-0.0), Expr::number(f64::NAN)] {
        assert!(Expr::sum(vec![expression.clone()]).same_node(&expression));
        assert!(Expr::product(vec![expression.clone()]).same_node(&expression));
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
