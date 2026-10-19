//! Exact rational function results and rejection of approximate candidates.

use core::ops::{Mul, Neg};

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView};

proptest! {
    #[test]
    fn rational_squares_recover_the_nonnegative_root(
        numerator in -1_000_000_i64..=1_000_000,
        denominator in 1_i64..=1_000_000,
    ) {
        let root = Number::from_rational(RationalType::new(numerator, denominator));
        let square = Expr::number(Mul::mul(&root, &root));
        let expected = Expr::number(Number::from_rational(RationalType::new(
            numerator.checked_abs().expect("the bounded numerator has an absolute value"),
            denominator,
        )));
        prop_assert_eq!(&square.sqrt(), &expected);

        let context = Context::new();
        let x = context.symb("rational_root")?;
        prop_assert_eq!(
            x.sqrt().substitute_symbol(x, &square),
            expected,
        );
    }

    #[test]
    fn rational_cubes_recover_signed_exact_roots(
        numerator in -10_000_i64..=10_000,
        denominator in 1_i64..=10_000,
    ) {
        let root = Number::from_rational(RationalType::new(numerator, denominator));
        let cube = Mul::mul(Mul::mul(&root, &root), &root);
        prop_assert_eq!(Expr::number(cube).cbrt(), Expr::number(root));
    }

    #[test]
    fn negative_rational_squares_have_positive_imaginary_roots(
        numerator in 1_i64..=1_000_000,
        denominator in 1_i64..=1_000_000,
    ) {
        let root = Number::from_rational(RationalType::new(numerator, denominator));
        let negative_square = Neg::neg(Mul::mul(&root, &root));
        let expected = Number::from_complex(Number::zero(), root);
        prop_assert_eq!(Expr::number(negative_square).sqrt(), Expr::number(expected));
    }

    #[test]
    fn complex_rational_squares_recover_principal_roots(
        real in 1_i64..=100,
        imaginary in -100_i64..=100,
        denominator in 1_i64..=100,
    ) {
        let root = Number::from_complex(
            Number::from_rational(RationalType::new(real, denominator)),
            Number::from_rational(RationalType::new(imaginary, denominator)),
        );
        let square = Mul::mul(&root, &root);
        prop_assert_eq!(Expr::number(square).sqrt(), Expr::number(root));
    }

    #[test]
    fn rational_pythagorean_norms_remain_exact(
        numerator in -1_000_i64..=1_000,
        denominator in 1_i64..=1_000,
    ) {
        let scale = Number::from_rational(RationalType::new(numerator, denominator));
        let value = Number::from_complex(
            Mul::mul(&scale, Number::from_int(3)),
            Mul::mul(&scale, Number::from_int(4)),
        );
        let magnitude = numerator
            .checked_abs()
            .and_then(|absolute| absolute.checked_mul(5))
            .expect("the bounded norm numerator fits");
        let expected = Number::from_rational(RationalType::new(magnitude, denominator));
        prop_assert_eq!(Expr::number(value).abs(), Expr::number(expected));
    }

    #[test]
    fn rational_rounding_abs_and_sign_follow_exact_integer_division(
        numerator in -1_000_000_i64..=1_000_000,
        denominator in 1_i64..=10_000,
    ) {
        let quotient = numerator.checked_div(denominator).expect("the denominator is positive");
        let remainder = numerator.checked_rem(denominator).expect("the denominator is positive");
        let magnitude = numerator.checked_abs().expect("the numerator is bounded");
        let doubled_remainder = remainder
            .checked_abs()
            .and_then(|value| value.checked_mul(2))
            .expect("the remainder is bounded by the denominator");
        let nearest = if doubled_remainder >= denominator {
            quotient.checked_add(numerator.signum()).expect("the rounded quotient is bounded")
        } else {
            quotient
        };
        let lower = quotient.checked_sub(i64::from(remainder < 0)).expect("the floor is bounded");
        let upper = quotient.checked_add(i64::from(remainder > 0)).expect("the ceiling is bounded");
        let input = Expr::number(Number::from_rational(RationalType::new(numerator, denominator)));
        for (actual, expected) in [
            (input.floor(), Number::from_int(lower)),
            (input.ceil(), Number::from_int(upper)),
            (input.round(), Number::from_int(nearest)),
            (input.trunc(), Number::from_int(quotient)),
            (input.fract(), Number::from_rational(RationalType::new(remainder, denominator))),
            (input.abs(), Number::from_rational(RationalType::new(magnitude, denominator))),
            (input.signum(), Number::from_int(numerator.signum())),
        ] {
            prop_assert_eq!(actual, Expr::number(expected));
        }
    }

    #[test]
    fn nonperfect_rational_roots_and_transcendentals_remain_symbolic(
        neighbor in 1_i64..=1_000_000,
    ) {
        let denominator = neighbor
            .checked_mul(neighbor)
            .and_then(|square| square.checked_add(1))
            .expect("the nonsquare denominator is bounded");
        let input = Expr::number(Number::from_rational(RationalType::new(1, denominator)));
        prop_assert!(matches!(input.sqrt().view(), ExprView::Function { .. }), "an irrational root remains symbolic");
        prop_assert!(matches!(input.sin().view(), ExprView::Function { .. }), "an approximate sine remains symbolic");
    }
}

#[test]
fn rational_root_boundaries_keep_wide_denominators_and_reject_float_rounding() {
    let quarter = Expr::number(Number::from_rational(RationalType::new(1, 4)));
    let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
    assert_eq!(quarter.sqrt(), half);
    assert_ne!(quarter.sqrt(), Expr::number(0.5));

    let square = Number::from_rational(RationalType::from_parts(1, 1_u64 << 62));
    let square_root = Number::from_rational(RationalType::from_parts(1, 1_u64 << 31));
    assert_eq!(Expr::number(square).sqrt(), Expr::number(square_root));
    let cube = Number::from_rational(RationalType::from_parts(-1, 1_u64 << 63));
    let cube_root = Number::from_rational(RationalType::from_parts(-1, 1_u64 << 21));
    assert_eq!(Expr::number(cube).cbrt(), Expr::number(cube_root));

    let near_one = Number::from_rational(RationalType::new(
        9_007_199_254_740_992,
        9_007_199_254_740_993,
    ));
    assert!(matches!(
        Expr::number(near_one).sqrt().view(),
        ExprView::Function { .. }
    ));
    assert_eq!(Expr::number(0.25).sqrt(), Expr::number(0.5));
}
