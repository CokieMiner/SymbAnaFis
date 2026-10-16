//! Generated checks for exact backend roots, norms, and integer cells.

use core::ops::Mul;

use num_anafis::{Number, Real};
use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::core::{Expr, ExprView};

fn assert_number(expression: &Expr, expected: Number, message: &str) {
    assert!(
        matches!(expression.view(), ExprView::Number(_)),
        "{message}"
    );
    assert_eq!(expression, &Expr::number(expected), "{message}");
}

fn assert_function(expression: &Expr, message: &str) {
    assert!(
        matches!(expression.view(), ExprView::Function { .. }),
        "{message}"
    );
}

proptest! {
    #[test]
    fn perfect_integer_cubes_keep_the_integer_tier(root in -10_000_i64..=10_000_i64) {
        let cube = root
            .checked_mul(root)
            .and_then(|value| value.checked_mul(root))
            .expect("the generated cube is bounded");
        let expected = Number::from_int(root);
        let backend = Number::from_int(cube).cbrt();
        prop_assert!(backend == expected);
        prop_assert!(!backend.is_float());
        assert_number(&Expr::number(cube).cbrt(), expected, "exact cbrt must fold");
    }

    #[test]
    fn perfect_integer_squares_keep_the_principal_integer_root(root in 0_i64..=100_000_i64) {
        let square = root
            .checked_mul(root)
            .expect("the generated square is bounded");
        let expected = Number::from_int(root);
        let backend = Number::from_int(square).sqrt();
        prop_assert!(backend == expected);
        prop_assert!(!expected.is_float());
        assert_number(&Expr::number(square).sqrt(), expected, "exact sqrt must fold");
    }

    #[test]
    fn gaussian_square_roots_round_trip_principal_integer_roots(
        real in 1_i64..=10_000_i64,
        imaginary in -10_000_i64..=10_000_i64,
    ) {
        let root = Number::from_complex(Number::from_int(real), Number::from_int(imaginary));
        let expected_root = root.clone();
        let square = Mul::mul(root, expected_root.clone());
        let result = square.sqrt();
        prop_assert!(result == expected_root);
        prop_assert!(!result.is_float());
        assert_number(&Expr::number(square).sqrt(), result, "exact Gaussian sqrt must fold");
    }

    #[test]
    fn gaussian_norms_are_exact_for_generated_pythagorean_triples(
        scale in 0_i64..=100_000_i64,
        negative_real in any::<bool>(),
        negative_imaginary in any::<bool>(),
    ) {
        let real_magnitude = scale
            .checked_mul(3)
            .expect("the generated Gaussian real component is bounded");
        let imaginary_magnitude = scale
            .checked_mul(4)
            .expect("the generated Gaussian imaginary component is bounded");
        let real = if negative_real {
            real_magnitude
                .checked_neg()
                .expect("the generated magnitude is safe")
        } else {
            real_magnitude
        };
        let imaginary = if negative_imaginary {
            imaginary_magnitude
                .checked_neg()
                .expect("the generated magnitude is safe")
        } else {
            imaginary_magnitude
        };
        let value = Number::from_complex(Number::from_int(real), Number::from_int(imaginary));
        let expected = value.abs();
        prop_assert!(!expected.is_float());
        assert_number(&Expr::number(value).abs(), expected, "exact Gaussian norm must fold");
    }

    #[test]
    fn real_signum_preserves_exact_integer_components(input in -1_000_000_i64..=1_000_000_i64) {
        let expected = Number::from_int(input.signum());
        let result = Number::from_int(input).signum();
        prop_assert!(result == expected);
        let parts = result.extract_complex().expect("real numbers have components");
        prop_assert!(matches!(parts.re, Real::Int(_)));
        assert_number(&Expr::number(input).signum(), expected, "exact signum must fold");
    }

    #[test]
    fn bounded_integer_powers_match_backend_results(
        base in -100_i64..=100_i64,
        exponent in 1_u32..=8_u32,
    ) {
        let base_number = Number::from_int(base);
        let exponent_number = Number::from_int(i64::from(exponent));
        let expected = base_number.pow(&exponent_number);
        assert_number(
            &Expr::number(base).pow(i64::from(exponent)),
            expected,
            "bounded integer powers must use backend promotion",
        );
    }

    #[test]
    fn exact_real_gamma_recurrence_preserves_integer_storage(order in 1_i64..=19_i64) {
        let successor = order.checked_add(1).expect("the generated order is bounded");
        let current = Number::from_int(order).gamma();
        let next = Number::from_int(successor).gamma();
        prop_assert_eq!(next, Mul::mul(current, Number::from_int(order)));
        prop_assert!(!Number::from_int(successor).gamma().is_float());
    }
}

#[test]
fn backend_rejects_nonexact_cells_and_preserves_large_exact_limits() {
    let noncube = Number::from_int(i64::MAX).cbrt();
    assert!(noncube.is_float());
    assert_function(
        &Expr::number(i64::MAX).cbrt(),
        "an inexact cube root remains symbolic",
    );
    assert_function(
        &Expr::number(2).sqrt(),
        "a nonsquare integer root remains symbolic",
    );
    assert_function(
        &Expr::number(-2).sqrt(),
        "a negative nonsquare remains symbolic",
    );

    let large_real = 3_000_000_000_i64;
    let large_imaginary = 100_000_000_i64;
    let square = Number::from_complex(
        Number::from_int(
            large_real
                .checked_mul(large_real)
                .and_then(|value| {
                    large_imaginary
                        .checked_mul(large_imaginary)
                        .and_then(|imaginary_square| value.checked_sub(imaginary_square))
                })
                .expect("large Gaussian real component fits"),
        ),
        Number::from_int(
            large_real
                .checked_mul(large_imaginary)
                .and_then(|value| value.checked_mul(2))
                .expect("large Gaussian imaginary component fits"),
        ),
    );
    assert_number(
        &Expr::number(square).sqrt(),
        Number::from_complex(
            Number::from_int(large_real),
            Number::from_int(large_imaginary),
        ),
        "large exact Gaussian roots remain exact",
    );
}

#[test]
fn backend_special_cells_and_minimum_boundaries_remain_explicit() {
    assert_number(
        &Expr::number(21).gamma(),
        Number::from_int(2_432_902_008_176_640_000),
        "the final representable factorial remains exact",
    );
    for input in [1, 2] {
        assert_number(
            &Expr::number(input).lgamma(),
            Number::zero(),
            "unit-factorial log gamma remains exact",
        );
    }
    assert_number(
        &Expr::number(1_000_000_000_000_000_000_i64).log10(),
        Number::from_int(18),
        "large powers of ten retain exact logarithms",
    );
    let imaginary_axis = Number::from_complex(Number::zero(), Number::from_int(i64::MIN));
    assert_number(
        &Expr::number(imaginary_axis).signum(),
        Number::from_complex(Number::zero(), Number::from_int(-1)),
        "Gaussian axis signum remains exact",
    );
    let minimum = Number::from_int(i64::MIN);
    assert_eq!(minimum.negate(), minimum);
    assert_function(
        &crate::core::Function::builtin("negate")
            .expect("negation is a known builtin")
            .call(alloc::vec![Expr::number(i64::MIN)])
            .expect("negation is unary"),
        "checked exact-cell negation rejects the minimum integer",
    );
}
