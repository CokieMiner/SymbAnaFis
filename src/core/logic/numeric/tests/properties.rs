//! Generated identities for exact tiers, arithmetic promotion, and function cells.

use core::ops::{Add, Div, Mul, Neg};

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use crate::core::Expr;

proptest! {
    #[test]
    fn integer_literals_reintern_without_crossing_the_float_tier(value in -1_000_000_i32..=1_000_000_i32) {
        let exact = Expr::number(value);
        prop_assert!(exact.same_node(&Expr::number(value)));
        prop_assert_ne!(exact, Expr::number(f64::from(value)));
    }

    #[test]
    fn equivalent_reduced_rationals_share_one_literal_node(
        numerator in -10_000_i64..=10_000_i64,
        denominator in 1_i64..=1_000_i64,
        factor in 1_i64..=16_i64,
    ) {
        let scaled_numerator = numerator
            .checked_mul(factor)
            .expect("the generated rational numerator is bounded");
        let scaled_denominator = denominator
            .checked_mul(factor)
            .expect("the generated rational denominator is bounded");
        let first = Expr::number(Number::from_rational(RationalType::new(numerator, denominator)));
        let second = Expr::number(Number::from_rational(RationalType::new(
            scaled_numerator,
            scaled_denominator,
        )));
        prop_assert!(first.same_node(&second));
    }

    #[test]
    fn bounded_integer_addition_multiplication_and_division_match_backend(
        left in -1_000_000_i64..=1_000_000_i64,
        right in -1_000_000_i64..=1_000_000_i64,
        divisor in 1_i64..=10_000_i64,
    ) {
        let left_number = Number::from_int(left);
        let right_number = Number::from_int(right);
        let sum = Add::add(left_number.clone(), right_number);
        prop_assert_eq!(
            Add::add(Expr::number(left), Expr::number(right)),
            Expr::number(sum),
        );
        let product_left = left.rem_euclid(1_000);
        let product_right = right.rem_euclid(1_000);
        let product = Mul::mul(Number::from_int(product_left), Number::from_int(product_right));
        prop_assert_eq!(
            Mul::mul(Expr::number(product_left), Expr::number(product_right)),
            Expr::number(product),
        );
        let quotient = Div::div(left_number, Number::from_int(divisor));
        prop_assert_eq!(
            Div::div(Expr::number(left), Expr::number(divisor)),
            Expr::number(quotient),
        );
    }

    #[test]
    fn bounded_gaussian_arithmetic_matches_component_promotion(
        real_left in -1_000_i64..=1_000_i64,
        imag_left in -1_000_i64..=1_000_i64,
        real_right in -1_000_i64..=1_000_i64,
        imag_right in -1_000_i64..=1_000_i64,
    ) {
        let left = Number::from_complex(Number::from_int(real_left), Number::from_int(imag_left));
        let right = Number::from_complex(Number::from_int(real_right), Number::from_int(imag_right));
        let expected_sum = Add::add(left.clone(), right.clone());
        let expected_product = Mul::mul(left.clone(), right.clone());
        prop_assert_eq!(
            Add::add(Expr::number(left.clone()), Expr::number(right.clone())),
            Expr::number(expected_sum),
        );
        prop_assert_eq!(
            Mul::mul(Expr::number(left), Expr::number(right)),
            Expr::number(expected_product),
        );
    }

    #[test]
    fn scalar_multiplication_commutes_for_generated_coefficients(coefficient in -1_000_000_i64..=1_000_000_i64) {
        let symbol = Expr::symbol("numeric_property_symbol");
        prop_assert_eq!(
            Mul::mul(symbol.clone(), Expr::number(coefficient)),
            Mul::mul(Expr::number(coefficient), symbol),
        );
    }

    #[test]
    fn powers_of_two_and_integer_logarithms_round_trip(
        exponent in 0_u32..=60_u32,
        base in 2_i64..=8_i64,
        base_exponent in 1_u32..=10_u32,
    ) {
        let power_of_two = 1_i64
            .checked_shl(exponent)
            .expect("the generated power of two fits in i64");
        prop_assert_eq!(Expr::number(power_of_two).log2(), Expr::number(i64::from(exponent)));

        let power = base
            .checked_pow(base_exponent)
            .expect("the generated integer power fits in i64");
        prop_assert_eq!(
            Expr::number(power).log_base(base),
            Expr::number(i64::from(base_exponent)),
        );
    }

    #[test]
    fn gamma_cells_obey_the_factorial_recurrence(order in 1_i64..=19_i64) {
        let successor = order.checked_add(1).expect("the generated order is bounded");
        let recurrence = Mul::mul(Expr::number(order).gamma(), Expr::number(order));
        prop_assert_eq!(Expr::number(successor).gamma(), recurrence);
        prop_assert!(
            Expr::number(successor)
                .gamma()
                .as_number()
                .is_some_and(|value| !value.is_float())
        );
    }

    #[test]
    fn imaginary_unit_powers_repeat_every_four(exponent in 0_u32..=64_u32) {
        let expected = match exponent % 4 {
            0 => Expr::number(1),
            1 => Expr::i(),
            2 => Expr::number(-1),
            _ => Neg::neg(Expr::i()),
        };
        prop_assert_eq!(Expr::i().pow(i64::from(exponent)), expected);
    }
}
