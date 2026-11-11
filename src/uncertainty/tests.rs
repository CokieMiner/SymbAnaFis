//! Covariance validation, domain edges, and first-order propagation contracts.

use alloc::vec;

use proptest::{bool, prelude::ProptestConfig, prop_assert_eq, proptest};

use crate::{
    Context, CovarianceMatrix, Expr, ExprView, Property, Substitution, Substitutions, Truth,
    Uncertainty, UncertaintyError,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn correlated_polynomial_variance_matches_an_independent_real_oracle(
        coefficient in -4_i64..=4, mixed in -4_i64..=4,
        value_x in -3_i64..=3, value_y in -3_i64..=3,
        factor_a in 1_i64..=4, factor_b in -3_i64..=3, factor_c in 1_i64..=4,
    ) {
        let context = Context::new()?;
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
        let actual = Substitutions::new([
            (x.into(), Expr::number(value_x)),
            (y.into(), Expr::number(value_y)),
        ])?.apply_all(result.variance());
        prop_assert_eq!(actual, Expr::number(expected));
        prop_assert_eq!(result.sensitivities(), model.gradient(&[x, y]));
    }

    #[test]
    fn diagonal_and_dense_covariances_agree_with_regular_or_pole_bearing_sensitivities(
        first in -8_i64..=8, second in -8_i64..=8,
        variance_x in 0_i64..=8, variance_y in 0_i64..=8,
        logarithmic in bool::ANY,
    ) {
        let context = Context::new()?;
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
fn covariance_validation_rejects_invalid_shapes_domains_and_asymmetry() {
    assert!(matches!(
        CovarianceMatrix::new(vec![vec![Expr::number(1), Expr::number(0)]]),
        Err(UncertaintyError::NonSquare { .. })
    ));
    // Every row length is checked before any entry, so a short later row is
    // reported even when an earlier entry has no mirror image.
    assert_eq!(
        CovarianceMatrix::new(vec![
            vec![Expr::number(1), Expr::number(2)],
            vec![Expr::number(3)],
        ])
        .err(),
        Some(UncertaintyError::NonSquare {
            row: 1,
            expected: 2,
            actual: 1
        })
    );
    let tiny = Expr::number(1.0e-30);
    assert!(matches!(
        CovarianceMatrix::new(vec![
            vec![Expr::number(1), tiny],
            vec![Expr::number(0), Expr::number(1)],
        ]),
        Err(UncertaintyError::Asymmetric { .. })
    ));
    for invalid in [
        Expr::number(-1),
        Expr::i(),
        Expr::number(f64::INFINITY),
        Expr::number(f64::NAN),
    ] {
        assert!(matches!(
            CovarianceMatrix::diagonal(vec![invalid]),
            Err(UncertaintyError::InvalidCovariance { row: 0, column: 0 })
        ));
    }
    let exact = Expr::number(1);
    let approximate = Expr::number(1.0);
    let matrix = CovarianceMatrix::new(vec![
        vec![Expr::number(2), exact.clone()],
        vec![approximate.clone(), Expr::number(2)],
    ])
    .expect("numerically symmetric without retagging entries");
    assert_eq!(matrix.get(0, 1), Some(&exact));
    assert_eq!(matrix.get(1, 0), Some(&approximate));
    assert_eq!(matrix.get(2, 0), None);
}

#[test]
fn propagation_checks_dimensions_duplicate_identities_and_known_invalid_models() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let one = CovarianceMatrix::diagonal(vec![Expr::number(1)]).expect("variance");
    assert!(matches!(
        Uncertainty::propagate(&x.to_expr(), &[], &one),
        Err(UncertaintyError::DimensionMismatch { .. })
    ));
    let two = CovarianceMatrix::diagonal(vec![Expr::number(1), Expr::number(1)]).expect("variance");
    assert!(matches!(
        Uncertainty::propagate(&x.to_expr(), &[x, x], &two),
        Err(UncertaintyError::DuplicateVariable {
            first: 0,
            duplicate: 1
        })
    ));
    assert!(matches!(
        Uncertainty::propagate(&Expr::i(), &[x], &one),
        Err(UncertaintyError::InvalidModel)
    ));
    let nonreal_slope = Expr::i() * x;
    assert!(matches!(
        Uncertainty::propagate(&nonreal_slope, &[x], &one),
        Err(UncertaintyError::InvalidSensitivity { index: 0 })
    ));
}

#[test]
fn empty_coordinates_and_relative_zero_exclusions_remain_explicit() {
    let covariance = CovarianceMatrix::diagonal(vec![]).expect("empty covariance");
    assert_eq!(covariance.dimension(), 0);
    assert_eq!(covariance.get(0, 0), None);
    let result =
        Uncertainty::propagate(&Expr::number(0), &[], &covariance).expect("constant model");
    assert_eq!(result.variance(), &Expr::number(0));
    assert_eq!(result.standard_deviation(), Expr::number(0));
    assert_eq!(result.relative(), Expr::number(0) / Expr::number(0));
    assert_eq!(result.sensitivities().len(), 0);
}

#[test]
fn physical_covariance_and_symbols_survive_the_context() {
    let result = {
        let context = Context::new().expect("context");
        let x = context.symb("x").expect("symbol");
        let variance = context.symb("variance").expect("symbol").to_expr();
        let covariance = CovarianceMatrix::diagonal(vec![variance])
            .expect("caller supplies real nonnegative variance");
        Uncertainty::propagate(&x.pow(2), &[x], &covariance).expect("regular real model")
    };
    assert!(result.variance().contains_var("x"));
    assert!(result.variance().contains_var("variance"));
    assert_eq!(result.expression().variables_ordered(), vec!["x"]);
    let other = Context::new().expect("context");
    let x = other.symb("x").expect("fresh identity");
    assert_eq!(result.expression().gradient(&[x]), vec![Expr::number(0)]);
}

#[test]
fn numeric_correlations_and_known_negative_result_variances_are_checked() {
    let context = Context::new().expect("context");
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
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(0)]).expect("zero variance");
    let result = Uncertainty::propagate(&x.ln(), &[x], &covariance)
        .expect("positive real regular input assumed");
    let at_pole = Substitution::new(x.into(), Expr::number(0)).apply_all(result.variance());
    assert_ne!(at_pole, Expr::number(0));
    assert_ne!(at_pole.properties().get(Property::Finite), Truth::True);
    assert!(at_pole.fold(false, |found, node| found
        || matches!(
            node.view(), ExprView::Div(_, denominator) if denominator.is_zero_num()
        )));
}

#[test]
fn numeric_symmetry_preserves_approximation_from_either_triangle() {
    let context = Context::new().expect("context");
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
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(2.0)]).expect("variance");
    let result = Uncertainty::propagate(&Expr::number(5), &[x], &covariance).expect("constant");
    assert_eq!(result.variance(), &Expr::number(0.0));
}
