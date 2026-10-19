//! Native forms, shared evaluation cost, and numerical regression evidence.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::prelude::*;

use crate::{
    Context, Expr, ExprView, HashMap, Property, SimplificationSequence, SimplificationStep,
    SimplificationTarget, Truth,
};

use super::{evaluate, simplify};

#[test]
fn a_compilation_step_selects_native_intrinsics() {
    let x = Expr::symbol("native_compilation_x");
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
    let x = Expr::symbol("compile_domain_x");
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
    let x = Expr::symbol("shared_compile_x");
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
    let x = Expr::symbol("horner_compile_x");
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
    let x = Expr::symbol("tiny_compile_x");
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
    let context = Context::new();
    let x = context.symb("x").expect("symbol");
    let density = context
        .parse("exp(-(x - mu)^2 / (2 * sigma^2)) / sqrt(2 * pi * sigma^2)")
        .expect("valid density formula");
    let derivative = Expr::derivative(density, x, 1);
    let sequence = SimplificationSequence::compilation().match_limit(Some(10_000));
    let compiled = simplify(&sequence, &derivative);
    assert_eq!(compiled, derivative);
    assert_eq!(simplify(&sequence, &compiled), compiled);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn cubic_intrinsics_require_a_stored_exact_one_third_power(
        degree in 1_i64..=12, offset in 1_i64..=8, kind in 0_u8..3,
    ) {
        let context = Context::new();
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
        let context = Context::new();
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
        let substitutions = HashMap::from([(symbol, sample)]);
        let before = evaluate(&input, &substitutions);
        let after = evaluate(&compiled, &substitutions);
        prop_assert!(before.approx_eq_number(&after, &Number::from_float(5.0e-12)), "before={before}, after={after}, input={input}, compiled={compiled}");
    }
}
