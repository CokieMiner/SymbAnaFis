//! Exact symmetry, scalar domains, identity lifetimes, and error boundaries.

use alloc::vec;

use crate::{Context, CovarianceMatrix, Expr, Uncertainty, UncertaintyError};

#[test]
fn covariance_validation_rejects_invalid_shapes_domains_and_asymmetry() {
    assert!(matches!(
        CovarianceMatrix::new(vec![vec![Expr::number(1), Expr::number(0)]]),
        Err(UncertaintyError::NonSquare { .. })
    ));
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
    let context = Context::new();
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
        let context = Context::new();
        let x = context.symb("x").expect("symbol");
        let variance = context.symb("variance").expect("symbol").to_expr();
        let covariance = CovarianceMatrix::diagonal(vec![variance])
            .expect("caller supplies real nonnegative variance");
        Uncertainty::propagate(&x.pow(2), &[x], &covariance).expect("regular real model")
    };
    assert!(result.variance().contains_var("x"));
    assert!(result.variance().contains_var("variance"));
    assert_eq!(result.expression().variables_ordered(), vec!["x"]);
    let other = Context::new();
    let x = other.symb("x").expect("fresh identity");
    assert_eq!(result.expression().gradient(&[x]), vec![Expr::number(0)]);
}
