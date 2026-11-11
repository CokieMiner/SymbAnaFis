//! Varied complex samples, native intrinsic forms, shared evaluation cost, and exact identities.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{
    prelude::ProptestConfig, prop_assert, prop_assert_eq, prop_oneof, proptest, strategy::Just,
};

use crate::{
    Context, Expr, ExprView, Property, SimplificationSequence, SimplificationStep,
    SimplificationTarget, Substitutions, Symbol, Truth,
};

use super::{evaluate, simplify};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn pythagorean_identities_collect_coefficients_and_keep_unmatched_terms(coefficient in 1_i64..=12, extra in -20_i64..=20) {
        let x = Expr::symbol("pythagorean_sequence_x")?;
        let input = Add::add(Mul::mul(Add::add(x.sin().pow(2), x.cos().pow(2)), coefficient), extra);
        let expected = Expr::number(coefficient.saturating_add(extra));
        prop_assert_eq!(simplify(&SimplificationSequence::compact(), &input), expected);
    }

    #[test]
    fn sine_cosine_products_handle_arbitrary_exact_coefficients(coefficient in -12_i64..=12) {
        let x = Expr::symbol("double_angle_coefficient_x")?;
        let input = Expr::product(vec![Expr::number(coefficient), x.sin(), x.cos()]);
        let program = SimplificationSequence::new().then(SimplificationStep::Trigonometric)
            .score_with(|expression| expression.fold(0_usize, |count, node| count.saturating_add(usize::from(
                matches!(node.view(), ExprView::Function { name, .. } if matches!(name.as_str(), "sin" | "cos"))
            ))));
        prop_assert_eq!(simplify(&program, &input), Div::div(Mul::mul(Mul::mul(&x, 2).sin(), coefficient), 2));
    }

    #[test]
    fn function_passes_preserve_values_for_varied_finite_complex_arguments(
        real in -2.0_f64..2.0, imaginary in 0.2_f64..0.8, kind in 0_u8..16,
    ) {
        let context = Context::new()?;
        let symbol = context.symb("x")?;
        let x = symbol.to_expr();
        let input = match kind {
            0 => Add::add(x.sin().pow(2), x.cos().pow(2)),
            1 => Expr::product(vec![Expr::number(2), x.sin(), x.cos()]),
            2 => Sub::sub(x.cos().pow(2), x.sin().pow(2)),
            3 => Sub::sub(Mul::mul(x.sin(), 3), Mul::mul(x.sin().pow(3), 4)),
            4 => Div::div(x.sin(), x.cos()),
            5 => Sub::sub(x.cosh().pow(2), x.sinh().pow(2)),
            6 => Expr::product(vec![Expr::number(2), x.sinh(), x.cosh()]),
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
        let substitutions = Substitutions::new([(symbol.into(), sample)])?;
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
                matches!(node.view(), ExprView::Function { name, .. } if matches!(name.as_str(), "sin" | "cos"))
            ))));
        prop_assert_eq!(simplify(&program, &angle.sin()), simplify(&program, &shifted.sin()));
        prop_assert_eq!(simplify(&program, &angle.cos()), simplify(&program, &shifted.cos()));
    }

    #[test]
    fn cubic_intrinsics_require_a_stored_exact_one_third_power(
        degree in 1_i64..=12, offset in 1_i64..=8, kind in 0_u8..3,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let base = match kind {
            0 => Expr::pi(),
            1 => Expr::e(),
            _ => Add::add(x.abs(), offset),
        };
        let bare = base.pow(degree);
        let program = SimplificationSequence::new()
            .then(SimplificationStep::Compilation)
            .target(SimplificationTarget::Compilation);
        prop_assert_eq!(&simplify(&program, &bare), &bare);
        let positive = Add::add(&bare, offset);
        let third = Expr::number(Number::from_rational(RationalType::new(1, 3)));
        prop_assert_eq!(simplify(&program, &positive.pow(third)), positive.cbrt());
        let approximate = bare.pow(Number::from_float(1.0 / 3.0));
        prop_assert_eq!(simplify(&program, &approximate), approximate);
    }

    #[test]
    fn compilation_passes_preserve_finite_complex_values(real in -0.8_f64..0.8, imaginary in 0.2_f64..0.7, kind in 0_u8..7) {
        let context = Context::new()?;
        let symbol = context.symb("x")?;
        let x = symbol.to_expr();
        let input = match kind {
            0 => Sub::sub(x.exp(), 1),
            1 => Sub::sub(1_i32, x.exp()),
            2 => Add::add(&x, 1).ln(),
            3 => Sub::sub(1_i32, x.erf()),
            4 => Div::div(x.exp().sin(), x.exp()),
            5 => x.abs().exp_m1().ln_1p(),
            _ => Expr::sum(vec![x.pow(3), Mul::mul(x.pow(2), 5), Mul::mul(&x, 7), Expr::number(11)]),
        };
        let compiled = simplify(&SimplificationSequence::compilation(), &input);
        let sample = Expr::number(Number::from_complex(Number::from_float(real), Number::from_float(imaginary)));
        let substitutions = Substitutions::new([(symbol.into(), sample)])?;
        let before = evaluate(&input, &substitutions);
        let after = evaluate(&compiled, &substitutions);
        prop_assert!(before.approx_eq_number(&after, &Number::from_float(5.0e-12)), "before={before}, after={after}, input={input}, compiled={compiled}");
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
    let x = Expr::symbol("quarter_turn_x").expect("symbol");
    assert_eq!(simplify(&program, &(&x + 2 * &pi).sin()), x.sin());
    assert_eq!(simplify(&program, &(&pi / 2 - &x).sin()), x.cos());
    let numeric = Expr::number(core::f64::consts::PI);
    assert_ne!(numeric.sin(), Expr::number(0));
}

#[test]
fn a_compilation_step_selects_native_intrinsics() {
    let x = Symbol::intern("native_compilation_x").expect("symbol");
    let program = SimplificationSequence::new()
        .then(SimplificationStep::Compilation)
        .target(SimplificationTarget::Compilation);
    let cases = [
        (Sub::sub(x.exp(), 1), x.exp_m1()),
        (Sub::sub(1_i32, x.exp()), Neg::neg(x.exp_m1())),
        (Add::add(&x, 1).ln(), x.ln_1p()),
        (Sub::sub(1_i32, x.erf()), x.erfc()),
        (Add::add(x.exp_m1(), 1), x.exp()),
        (Div::div(x.exp().sin(), x.exp()), x.exp().sinc()),
        (
            Add::add(x.abs(), 1).pow(Expr::number(Number::from_rational(RationalType::new(1, 3)))),
            Add::add(x.abs(), 1).cbrt(),
        ),
        (x.abs().exp_m1().ln_1p(), x.abs()),
        (Add::add(x.abs(), 1).ln_1p().exp_m1(), Add::add(x.abs(), 1)),
    ];
    for (input, expected) in cases {
        assert_eq!(simplify(&program, &input), expected);
    }
}

#[test]
fn native_logarithm_and_sinc_rewrites_retain_their_domains() {
    let x = Symbol::intern("compile_domain_x").expect("symbol");
    let program = SimplificationSequence::compilation();
    let quotient = Div::div(x.sin(), &x);
    assert_eq!(simplify(&program, &quotient), quotient);
    let principal = x.exp_m1().ln_1p();
    assert_eq!(simplify(&program, &principal), principal);
    let zero_exclusion = x.ln_1p().exp_m1();
    assert_eq!(simplify(&program, &zero_exclusion), zero_exclusion);
    let logarithm = simplify(&program, &Add::add(&x, 1).ln());
    assert_eq!(logarithm, x.ln_1p());
    assert_ne!(
        logarithm
            .substitute("compile_domain_x", &Expr::number(-1))
            .properties()
            .get(Property::Finite),
        Truth::True
    );
}

#[test]
fn compilation_cost_preserves_reused_transcendental_results() {
    let x = Symbol::intern("shared_compile_x").expect("symbol");
    let input = Expr::sum(vec![
        Expr::product(vec![Expr::number(2), x.sin(), x.cos()]),
        x.sin(),
        x.cos(),
    ]);
    let steps = SimplificationSequence::new().then(SimplificationStep::Trigonometric);
    let compact = simplify(&steps, &input);
    assert_ne!(compact, input);
    let compiled = simplify(&steps.target(SimplificationTarget::Compilation), &input);
    assert_eq!(compiled, input);
}

#[test]
fn compilation_factors_polynomials_into_cheaper_nested_arithmetic() {
    let x = Symbol::intern("horner_compile_x").expect("symbol");
    let input = Expr::sum(vec![
        x.pow(4),
        Mul::mul(x.pow(3), 3),
        Mul::mul(x.pow(2), 5),
        Mul::mul(&x, 7),
        Expr::number(11),
    ]);
    let compiled = simplify(&SimplificationSequence::compilation(), &input);
    let powers = |node: &Expr| {
        node.fold(0_usize, |count, part| {
            count.saturating_add(usize::from(matches!(part.view(), ExprView::Pow(..))))
        })
    };
    assert!(powers(&compiled) < powers(&input));
    for value in -3..=3 {
        assert_eq!(
            input.substitute("horner_compile_x", &Expr::number(value)),
            compiled.substitute("horner_compile_x", &Expr::number(value))
        );
    }
}

#[test]
fn native_cancellation_intrinsics_retain_tiny_nonzero_results() {
    let x = Symbol::intern("tiny_compile_x").expect("symbol");
    let program = SimplificationSequence::compilation();
    let tiny = Expr::number(1.0e-20);
    for input in [Sub::sub(x.exp(), 1), Add::add(&x, 1).ln()] {
        let compiled = simplify(&program, &input);
        assert!(input.substitute("tiny_compile_x", &tiny).is_zero_num());
        assert_eq!(compiled.substitute("tiny_compile_x", &tiny), tiny);
    }
}

#[test]
fn gaussian_density_derivative_respects_the_commutative_search_bound() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol");
    let density = context
        .parse("exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)")
        .expect("valid density formula");
    let derivative = Expr::derivative(density, x, 1);
    let sequence = SimplificationSequence::compilation().match_limit(10_000);
    let compiled = simplify(&sequence, &derivative);
    assert_eq!(compiled, derivative);
    assert_eq!(simplify(&sequence, &compiled), compiled);
}
