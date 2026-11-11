//! Covariance validation, domain edges, and first-order propagation contracts.

use alloc::{vec, vec::Vec};

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
fn a_coordinate_from_another_context_is_rejected_by_name() {
    let model_context = Context::new().expect("context");
    let x = model_context.symb("x").expect("symbol");
    let covariance_context = Context::new().expect("context");
    let foreign = covariance_context
        .symb("x")
        .expect("same spelling, other identity");
    let covariance = CovarianceMatrix::diagonal(vec![Expr::number(4)]).expect("variance");
    assert!(matches!(
        Uncertainty::propagate(&x.to_expr(), &[foreign], &covariance),
        Err(UncertaintyError::ForeignCoordinate { index: 0 })
    ));
    // A parameter the model genuinely does not use keeps its zero contribution.
    let unused = model_context.symb("unused").expect("symbol");
    let result = Uncertainty::propagate(
        &x.to_expr(),
        &[x, unused],
        &CovarianceMatrix::diagonal(vec![Expr::number(4), Expr::number(9)]).expect("variance"),
    )
    .expect("absent name is not a foreign identity");
    assert_eq!(result.variance(), &Expr::number(4));
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

#[test]
fn a_supplied_factor_contracts_a_sum_of_squares() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    // Sigma = [[4, 2], [2, 5]] = L L^T with L = [[2, 0], [1, 2]]. The expanded
    // form is 4 - 2 - 2 + 5; the factored form is (2 - 1)^2 + (0 - 2)^2.
    let factor = CovarianceMatrix::from_factor(vec![
        vec![Expr::number(2), Expr::number(0)],
        vec![Expr::number(1), Expr::number(2)],
    ])
    .expect("factor");
    assert_eq!(factor.dimension(), 2);
    assert_eq!(factor.get(0, 0), Some(&Expr::number(4)));
    assert_eq!(factor.get(0, 1), Some(&Expr::number(2)));
    assert_eq!(factor.get(1, 0), Some(&Expr::number(2)));
    assert_eq!(factor.get(1, 1), Some(&Expr::number(5)));
    let result = Uncertainty::propagate(&(x - y), &[x, y], &factor).expect("model");
    assert_eq!(result.variance(), &Expr::number(5));
    // The same covariance through the expanded form agrees exactly.
    let expanded = CovarianceMatrix::new(vec![
        vec![Expr::number(4), Expr::number(2)],
        vec![Expr::number(2), Expr::number(5)],
    ])
    .expect("covariance");
    let other = Uncertainty::propagate(&(x - y), &[x, y], &expanded).expect("model");
    assert_eq!(other.variance(), result.variance());
    assert!(matches!(
        CovarianceMatrix::from_factor(vec![vec![Expr::number(1)], vec![]]),
        Err(UncertaintyError::NonSquare {
            row: 0,
            expected: 2,
            actual: 1
        })
    ));
    assert!(matches!(
        CovarianceMatrix::from_factor(vec![vec![Expr::i()]]),
        Err(UncertaintyError::InvalidCovariance { row: 0, column: 0 })
    ));
}

#[test]
fn recontract_reuses_the_gradient_across_covariances() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let y = context.symb("y").expect("symbol");
    let model = x + 2 * y;
    let diagonal =
        CovarianceMatrix::diagonal(vec![Expr::number(4), Expr::number(9)]).expect("variance");
    let result = Uncertainty::propagate(&model, &[x, y], &diagonal).expect("model");
    assert_eq!(result.variance(), &Expr::number(40));
    let correlated = CovarianceMatrix::new(vec![
        vec![Expr::number(4), Expr::number(1)],
        vec![Expr::number(1), Expr::number(9)],
    ])
    .expect("covariance");
    assert_eq!(result.recontract(&correlated), Ok(Expr::number(44)));
    assert_eq!(
        result.recontract(&CovarianceMatrix::diagonal(vec![Expr::number(1)]).expect("variance")),
        Err(UncertaintyError::DimensionMismatch {
            variables: 2,
            covariance: 1
        })
    );
}

/// The transposed entry of an upper-triangle pair is read from the diagonal of
/// its own row, so a dense matrix of dimension three or more contracts to the
/// same variance as the diagonal matrix of its diagonal.
#[test]
fn dense_contraction_reads_the_transposed_entry_of_each_pair() {
    let context = Context::new().expect("context");
    let x = context.symb("dense_x").expect("symbol");
    let y = context.symb("dense_y").expect("symbol");
    let z = context.symb("dense_z").expect("symbol");
    let model = Expr::sum(vec![x.to_expr(), y.to_expr(), z.to_expr()]);
    let variables = [x, y, z];
    let cases = [
        (
            vec![
                vec![Expr::number(1), Expr::number(0), Expr::number(0)],
                vec![Expr::number(0), Expr::number(5), Expr::number(0)],
                vec![Expr::number(0), Expr::number(0), Expr::number(1)],
            ],
            7,
        ),
        (
            vec![
                vec![Expr::number(4), Expr::number(1), Expr::number(0)],
                vec![Expr::number(1), Expr::number(9), Expr::number(2)],
                vec![Expr::number(0), Expr::number(2), Expr::number(1)],
            ],
            20,
        ),
        (
            vec![
                vec![Expr::number(1), Expr::number(1), Expr::number(1)],
                vec![Expr::number(1), Expr::number(1), Expr::number(1)],
                vec![Expr::number(1), Expr::number(1), Expr::number(1)],
            ],
            9,
        ),
    ];
    for (rows, expected) in cases {
        let covariance = CovarianceMatrix::new(rows).expect("covariance");
        let result = Uncertainty::propagate(&model, &variables, &covariance).expect("model");
        assert_eq!(
            result.variance(),
            &Expr::number(expected),
            "a dense contraction sums every pair once"
        );
    }
}

#[test]
fn contract_enforces_matching_dimensions_across_all_storage_forms() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let empty = CovarianceMatrix::new(Vec::new()).expect("empty covariance");
    assert_eq!(empty.dimension(), 0);
    assert_eq!(empty.get(0, 0), None);
    assert_eq!(empty.contract(&[]), Ok(Expr::number(0)));
    assert_eq!(
        empty.contract(&[x.into()]),
        Err(UncertaintyError::DimensionMismatch {
            variables: 1,
            covariance: 0,
        })
    );

    let diagonal =
        CovarianceMatrix::diagonal(vec![Expr::number(1), Expr::number(2)]).expect("diag");
    let dense = CovarianceMatrix::new(vec![
        vec![Expr::number(1), Expr::number(0)],
        vec![Expr::number(0), Expr::number(2)],
    ])
    .expect("dense");
    let factored = CovarianceMatrix::from_factor(vec![
        vec![Expr::number(1), Expr::number(0)],
        vec![Expr::number(0), Expr::number(2)],
    ])
    .expect("factored");
    for covariance in [&diagonal, &dense, &factored] {
        assert_eq!(
            covariance.contract(&[Expr::number(1)]),
            Err(UncertaintyError::DimensionMismatch {
                variables: 1,
                covariance: 2,
            })
        );
        assert_eq!(
            covariance.contract(&[Expr::number(1), Expr::number(1), Expr::number(1)]),
            Err(UncertaintyError::DimensionMismatch {
                variables: 3,
                covariance: 2,
            })
        );
    }
}

#[test]
fn factored_3x3_matches_expanded_and_preserves_approximate_zero_tiers() {
    let context = Context::new().expect("context");
    let x = context.symb("fx").expect("symbol");
    let y = context.symb("fy").expect("symbol");
    let z = context.symb("fz").expect("symbol");
    let factor = CovarianceMatrix::from_factor(vec![
        vec![Expr::number(2), Expr::number(0), Expr::number(0)],
        vec![Expr::number(1), Expr::number(3), Expr::number(0)],
        vec![Expr::number(0), Expr::number(-1), Expr::number(2)],
    ])
    .expect("3x3 factor");
    assert_eq!(factor.get(0, 0), Some(&Expr::number(4)));
    assert_eq!(factor.get(0, 1), Some(&Expr::number(2)));
    assert_eq!(factor.get(0, 2), Some(&Expr::number(0)));
    assert_eq!(factor.get(1, 1), Some(&Expr::number(10)));
    assert_eq!(factor.get(1, 2), Some(&Expr::number(-3)));
    assert_eq!(factor.get(2, 2), Some(&Expr::number(5)));
    assert_eq!(factor.get(2, 1), Some(&Expr::number(-3)));

    let model = Expr::sum(vec![x.to_expr(), 2 * y, -z.to_expr()]);
    let factored_result = Uncertainty::propagate(&model, &[x, y, z], &factor).expect("model");
    let expanded = CovarianceMatrix::new(vec![
        vec![Expr::number(4), Expr::number(2), Expr::number(0)],
        vec![Expr::number(2), Expr::number(10), Expr::number(-3)],
        vec![Expr::number(0), Expr::number(-3), Expr::number(5)],
    ])
    .expect("expanded");
    let expanded_result = Uncertainty::propagate(&model, &[x, y, z], &expanded).expect("model");
    assert_eq!(factored_result.variance(), expanded_result.variance());

    // Exact zero sensitivity times approximate factor retains float tier.
    let approx = CovarianceMatrix::from_factor(vec![vec![Expr::number(2.0)]]).expect("approx");
    let constant = Uncertainty::propagate(&Expr::number(5), &[x], &approx).expect("constant");
    assert_eq!(constant.variance(), &Expr::number(0.0));
}

#[test]
fn symbolic_diagonal_and_correlated_create_scoped_symbols_and_propagate() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("x");
    let y = context.symb("y").expect("y");
    let z = context.symb("z").expect("z");

    let diag = CovarianceMatrix::symbolic_diagonal(&context, &[x, y]).expect("symbolic diagonal");
    let sigma_x = context.get_symbol("sigma_x").expect("sigma_x created");
    let sigma_y = context.get_symbol("sigma_y").expect("sigma_y created");
    assert_eq!(diag.dimension(), 2);
    assert_eq!(diag.get(0, 0), Some(&sigma_x.pow(2)));
    assert_eq!(diag.get(0, 1), Some(&Expr::number(0)));
    assert_eq!(diag.get(1, 1), Some(&sigma_y.pow(2)));
    let diag_result = Uncertainty::propagate(&(x + 2 * y), &[x, y], &diag).expect("diag prop");
    assert_eq!(
        diag_result.variance(),
        &(sigma_x.pow(2) + 4 * sigma_y.pow(2))
    );

    let corr =
        CovarianceMatrix::symbolic_correlated(&context, &[x, y, z]).expect("symbolic correlated");
    let sigma_z = context.get_symbol("sigma_z").expect("sigma_z created");
    let first_second = context.get_symbol("rho_x_y").expect("rho_x_y created");
    let first_third = context.get_symbol("rho_x_z").expect("rho_x_z created");
    let second_third = context.get_symbol("rho_y_z").expect("rho_y_z created");
    assert_eq!(corr.dimension(), 3);
    assert_eq!(corr.get(0, 1), Some(&(first_second * sigma_x * sigma_y)));
    assert_eq!(corr.get(1, 0), corr.get(0, 1));
    assert_eq!(corr.get(0, 2), Some(&(first_third * sigma_x * sigma_z)));
    assert_eq!(corr.get(1, 2), Some(&(second_third * sigma_y * sigma_z)));
    let corr_result = Uncertainty::propagate(
        &(x + y),
        &[x, y],
        &CovarianceMatrix::symbolic_correlated(&context, &[x, y]).expect("2x2 corr"),
    )
    .expect("corr prop");
    assert_eq!(
        corr_result.variance(),
        &(sigma_x.pow(2) + 2 * first_second * sigma_x * sigma_y + sigma_y.pow(2))
    );

    // Empty coordinate lists produce 0x0 matrices.
    assert_eq!(
        CovarianceMatrix::symbolic_diagonal(&context, &[])
            .expect("empty diag")
            .dimension(),
        0
    );
    assert_eq!(
        CovarianceMatrix::symbolic_correlated(&context, &[])
            .expect("empty corr")
            .dimension(),
        0
    );

    // Duplicate variables are rejected.
    assert_eq!(
        CovarianceMatrix::symbolic_diagonal(&context, &[x, x]).err(),
        Some(UncertaintyError::DuplicateVariable {
            first: 0,
            duplicate: 1,
        })
    );
    assert_eq!(
        CovarianceMatrix::symbolic_correlated(&context, &[x, y, x]).err(),
        Some(UncertaintyError::DuplicateVariable {
            first: 0,
            duplicate: 2,
        })
    );
}
