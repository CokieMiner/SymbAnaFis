//! Independent real polynomial and positive-semidefinite covariance oracles.

use alloc::vec;

use proptest::prelude::*;

use crate::{
    Context, CovarianceMatrix, Expr, ExprView, Property, Truth, Uncertainty, UncertaintyError,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn correlated_polynomial_variance_matches_an_independent_real_oracle(
        coefficient in -4_i64..=4, mixed in -4_i64..=4,
        value_x in -3_i64..=3, value_y in -3_i64..=3,
        factor_a in 1_i64..=4, factor_b in -3_i64..=3, factor_c in 1_i64..=4,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let model = Expr::sum(vec![
            Expr::product(vec![x.pow(2), Expr::number(coefficient)]),
            Expr::product(vec![x.to_expr(), Expr::number(mixed), y.to_expr()]),
            y.pow(2),
        ]);
        // Sigma = L L^T, L = [[a, 0], [b, c]], so every sample is positive definite.
        let variance_x = factor_a.saturating_mul(factor_a);
        let cross = factor_a.saturating_mul(factor_b);
        let variance_y = factor_b.saturating_mul(factor_b).saturating_add(factor_c.saturating_mul(factor_c));
        let covariance = CovarianceMatrix::new(vec![
            vec![Expr::number(variance_x), Expr::number(cross)],
            vec![Expr::number(cross), Expr::number(variance_y)],
        ])?;
        let result = Uncertainty::propagate(&model, &[x, y], &covariance)?;
        let gradient_x = coefficient.saturating_mul(2).saturating_mul(value_x).saturating_add(mixed.saturating_mul(value_y));
        let gradient_y = mixed.saturating_mul(value_x).saturating_add(value_y.saturating_mul(2));
        let expected = variance_x.saturating_mul(gradient_x.saturating_mul(gradient_x))
            .saturating_add(2_i64.saturating_mul(cross).saturating_mul(gradient_x).saturating_mul(gradient_y))
            .saturating_add(variance_y.saturating_mul(gradient_y.saturating_mul(gradient_y)));
        let actual = result.variance().substitute_symbol(x, &Expr::number(value_x))
            .substitute_symbol(y, &Expr::number(value_y));
        prop_assert_eq!(actual, Expr::number(expected));
        prop_assert_eq!(result.sensitivities(), model.gradient(&[x, y]));
    }

    #[test]
    fn diagonal_and_dense_covariances_agree_with_regular_or_pole_bearing_sensitivities(
        first in -8_i64..=8, second in -8_i64..=8,
        variance_x in 0_i64..=8, variance_y in 0_i64..=8,
        logarithmic in proptest::bool::ANY,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let model = Expr::sum(vec![
            Expr::product(vec![if logarithmic { x.ln() } else { x.to_expr() }, Expr::number(first)]),
            Expr::product(vec![if logarithmic { y.ln() } else { y.to_expr() }, Expr::number(second)]),
        ]);
        let diagonal = CovarianceMatrix::diagonal(vec![Expr::number(variance_x), Expr::number(variance_y)])?;
        let dense = CovarianceMatrix::new(vec![
            vec![Expr::number(variance_x), Expr::number(0)],
            vec![Expr::number(0), Expr::number(variance_y)],
        ])?;
        let first_result = Uncertainty::propagate(&model, &[x, y], &diagonal)?;
        let second_result = Uncertainty::propagate(&model, &[x, y], &dense)?;
        prop_assert_eq!(first_result.variance(), second_result.variance());
    }
}

#[test]
fn numeric_correlations_and_known_negative_result_variances_are_checked() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    let covariance = CovarianceMatrix::new(vec![
        vec![Expr::number(4), Expr::number(1)],
        vec![Expr::number(1), Expr::number(9)],
    ])
    .expect("covariance");
    let result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &covariance).expect("model");
    assert_eq!(result.variance(), &Expr::number(44));
    let invalid = CovarianceMatrix::new(vec![
        vec![Expr::number(1), Expr::number(2)],
        vec![Expr::number(2), Expr::number(1)],
    ])
    .expect("shape valid; caller still owes positive semidefiniteness");
    assert!(matches!(
        Uncertainty::propagate(&(x - y), &[x, y], &invalid),
        Err(UncertaintyError::InvalidVariance)
    ));
}

#[test]
fn zero_variance_never_cancels_an_unknown_sensitivity_pole() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(0)]).expect("zero variance");
    let result = Uncertainty::propagate(&x.ln(), &[x], &covariance)
        .expect("positive real regular input assumed");
    let at_pole = result.variance().substitute_symbol(x, &Expr::number(0));
    assert_ne!(at_pole, Expr::number(0));
    assert_ne!(at_pole.properties().get(Property::Finite), Truth::True);
    assert!(at_pole.fold(false, |found, node| found
        || matches!(
            node.view(), ExprView::Div(_, denominator) if denominator.is_zero_num()
        )));
}

#[test]
fn numeric_symmetry_preserves_approximation_from_either_triangle() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    let model = x + y;
    for (upper, lower) in [
        (Expr::number(1), Expr::number(1.0)),
        (Expr::number(1.0), Expr::number(1)),
        (Expr::number(0), Expr::number(0.0)),
        (Expr::number(0.0), Expr::number(0)),
    ] {
        let expected = if upper.is_zero_num() { 4.0 } else { 6.0 };
        let covariance = CovarianceMatrix::new(vec![
            vec![Expr::number(2), upper],
            vec![lower, Expr::number(2)],
        ])
        .expect("equal numeric values");
        let result = Uncertainty::propagate(&model, &[x, y], &covariance).expect("linear model");
        assert_eq!(result.variance(), &Expr::number(expected));
    }
}

#[test]
fn zero_sensitivities_retain_approximate_covariance_arithmetic() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(2.0)]).expect("variance");
    let result = Uncertainty::propagate(&Expr::number(5), &[x], &covariance).expect("constant");
    assert_eq!(result.variance(), &Expr::number(0.0));
}
