//! Varied complex samples and exact identity regressions.

use core::ops::{Add, Div, Mul, Neg, Sub};

use num_anafis::{Number, RationalType};
use proptest::prelude::*;

use crate::{Context, Expr, HashMap, SimplificationSequence, SimplificationStep};

use super::{evaluate, simplify};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn pythagorean_identities_collect_coefficients_and_keep_unmatched_terms(coefficient in 1_i64..=12, extra in -20_i64..=20) {
        let x = Expr::symbol("pythagorean_sequence_x");
        let input = Add::add(Mul::mul(Add::add(x.sin().pow(2), x.cos().pow(2)), coefficient), extra);
        let expected = Expr::number(coefficient.saturating_add(extra));
        prop_assert_eq!(simplify(&SimplificationSequence::compact(), &input), expected);
    }

    #[test]
    fn sine_cosine_products_handle_arbitrary_exact_coefficients(coefficient in -12_i64..=12) {
        let x = Expr::symbol("double_angle_coefficient_x");
        let input = Expr::product(alloc::vec![Expr::number(coefficient), x.sin(), x.cos()]);
        let program = SimplificationSequence::new().then(SimplificationStep::Trigonometric)
            .score_with(|expression| expression.fold(0_usize, |count, node| count.saturating_add(usize::from(
                matches!(node.view(), crate::ExprView::Function { name, .. } if matches!(name.as_str(), "sin" | "cos"))
            ))));
        prop_assert_eq!(simplify(&program, &input), Div::div(Mul::mul(Mul::mul(&x, 2).sin(), coefficient), 2));
    }

    #[test]
    fn function_passes_preserve_values_for_varied_finite_complex_arguments(
        real in -2.0_f64..2.0, imaginary in 0.2_f64..0.8, kind in 0_u8..16,
    ) {
        let context = Context::new();
        let symbol = context.symb("x")?;
        let x = symbol.to_expr();
        let input = match kind {
            0 => Add::add(x.sin().pow(2), x.cos().pow(2)),
            1 => Expr::product(alloc::vec![Expr::number(2), x.sin(), x.cos()]),
            2 => Sub::sub(x.cos().pow(2), x.sin().pow(2)),
            3 => Sub::sub(Mul::mul(x.sin(), 3), Mul::mul(x.sin().pow(3), 4)),
            4 => Div::div(x.sin(), x.cos()),
            5 => Sub::sub(x.cosh().pow(2), x.sinh().pow(2)),
            6 => Expr::product(alloc::vec![Expr::number(2), x.sinh(), x.cosh()]),
            7 => Add::add(x.cosh().pow(2), x.sinh().pow(2)),
            8 => Div::div(Sub::sub(x.exp(), Neg::neg(&x).exp()), 2),
            9 => Div::div(Add::add(x.exp(), Neg::neg(&x).exp()), 2),
            10 => Div::div(Sub::sub(x.exp(), Neg::neg(&x).exp()), Add::add(x.exp(), Neg::neg(&x).exp())),
            11 => Div::div(Sub::sub(Mul::mul(&x, 2).exp(), 1), Add::add(Mul::mul(&x, 2).exp(), 1)),
            12 => Mul::mul(Add::add(&x, 1).exp(), Sub::sub(&x, 1).exp()),
            13 => x.asin().sin(),
            14 => x.asinh().sinh(),
            _ => Neg::neg(&x).cosh(),
        };
        let output = simplify(&SimplificationSequence::compact(), &input);
        let sample = Expr::number(Number::from_complex(Number::from_float(real), Number::from_float(imaginary)));
        let substitutions = HashMap::from([(symbol, sample)]);
        let before = evaluate(&input, &substitutions);
        let after = evaluate(&output, &substitutions);
        prop_assert!(before.approx_eq_number(&after, &Number::from_float(5.0e-10)), "before={before}, after={after}, input={input}, output={output}");
    }

    #[test]
    fn exact_rational_pi_angles_are_periodic_and_keep_their_exact_values(numerator in -10_000_i64..=10_000, denominator in prop_oneof![Just(1_u64), Just(2), Just(3), Just(4), Just(6), Just(12)]) {
        let angle = Mul::mul(Expr::number(Number::from_rational(RationalType::from_parts(numerator, denominator))), Expr::pi());
        let period = 2_i64.saturating_mul(i64::try_from(denominator).expect("small denominator"));
        let shifted = Mul::mul(Expr::number(Number::from_rational(RationalType::from_parts(numerator.saturating_add(period), denominator))), Expr::pi());
        let program = SimplificationSequence::new().then(SimplificationStep::Trigonometric)
            .score_with(|expression| expression.fold(0_usize, |count, node| count.saturating_add(usize::from(
                matches!(node.view(), crate::ExprView::Function { name, .. } if matches!(name.as_str(), "sin" | "cos"))
            ))));
        prop_assert_eq!(simplify(&program, &angle.sin()), simplify(&program, &shifted.sin()));
        prop_assert_eq!(simplify(&program, &angle.cos()), simplify(&program, &shifted.cos()));
    }
}

#[test]
fn exact_angles_and_quarter_turn_translations_do_not_use_float_tolerances() {
    let program = SimplificationSequence::new().then(SimplificationStep::Trigonometric);
    let pi = Expr::pi();
    assert_eq!(simplify(&program, &pi.sin()), Expr::number(0));
    assert_eq!(simplify(&program, &pi.cos()), Expr::number(-1));
    assert_eq!(simplify(&program, &(&pi / 6).sin()), Expr::number(1) / 2);
    assert_eq!(simplify(&program, &(&pi / 2).tan()), (&pi / 2).tan());
    let x = Expr::symbol("quarter_turn_x");
    assert_eq!(simplify(&program, &(&x + 2 * &pi).sin()), x.sin());
    assert_eq!(simplify(&program, &(&pi / 2 - &x).sin()), x.cos());
    let numeric = Expr::number(core::f64::consts::PI);
    assert_ne!(numeric.sin(), Expr::number(0));
}
