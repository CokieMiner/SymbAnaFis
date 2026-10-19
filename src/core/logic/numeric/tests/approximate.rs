//! Generated coverage for finite approximate builtin evaluation.

use alloc::vec;

use num_anafis::{Number, RationalType, Real};
use proptest::{prop_assert, prop_assert_eq, prop_assume, proptest};

use crate::core::{Context, Expr, ExprView, Function, Pattern, RewriteRule, UserFunction};

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
    fn finite_float_square_roots_fold_without_losing_the_float_tier(input in 0.000_001_f64..=1_000_000.0_f64) {
        let expected = Number::from_float(input).sqrt();
        let folded = Expr::number(input).sqrt();
        prop_assert!(expected.is_finite());
        assert_number(&folded, expected, "finite approximate sqrt must fold");
        prop_assert!(folded.as_number().is_some_and(Number::is_float));
    }

    #[test]
    fn finite_float_sine_matches_the_numeric_backend(input in -1_000.0_f64..=1_000.0_f64) {
        let expected = Number::from_float(input).sin();
        let folded = Expr::number(input).sin();
        assert_number(&folded, expected, "finite approximate sine must fold");
        prop_assert!(folded.as_number().is_some_and(Number::is_float));
    }

    #[test]
    fn mixed_complex_components_fold_and_retain_an_approximate_component(
        real in -2.0_f64..=2.0_f64,
        imaginary in -2_i64..=2_i64,
    ) {
        let input = Number::from_complex(Number::from_float(real), Number::from_int(imaginary));
        let expected = input.sin();
        let folded = Expr::number(input).sin();
        assert_number(&folded, expected, "mixed complex sine must fold");
        let parts = folded
            .as_number()
            .expect("mixed complex sine should produce a number")
            .extract_complex()
            .expect("mixed complex sine should have components");
        prop_assert!(parts.re.is_float() || parts.im.is_float());
    }

    #[test]
    fn finite_binary_log_uses_value_first_backend_order(
        value in 0.1_f64..=1_000_000.0_f64,
        base in 1.1_f64..=10.0_f64,
    ) {
        let value_number = Number::from_float(value);
        let base_number = Number::from_float(base);
        let expected = value_number.log(&base_number);
        assert_number(
            &Expr::number(value).log(base),
            expected,
            "log(base, value) must pass value then base",
        );
    }

    #[test]
    fn mixed_binary_log_arguments_preserve_the_float_input(
        value in 1_i64..=1_000_000_i64,
        base in 1.1_f64..=10.0_f64,
    ) {
        let value_number = Number::from_int(value);
        let base_number = Number::from_float(base);
        let expected = value_number.log(&base_number);
        assert_number(
            &Expr::number(value).log(base),
            expected,
            "an approximate base must enable folding",
        );
    }

    #[test]
    fn integral_float_orders_are_valid_for_discrete_functions(order in 0_i32..=4_i32, value in -0.5_f64..=0.5_f64) {
        let order_float = f64::from(order);
        let value_number = Number::from_float(value);
        let order_number = Number::from_float(order_float);
        assert_number(
            &Expr::number(value).hermite(order_float),
            value_number.hermite(&order_number),
            "an integral float order must be accepted",
        );
    }

    #[test]
    fn higher_arity_functions_keep_argument_positions(
        degree in 0_i32..=3_i32,
        order in -2_i32..=2_i32,
        argument in 0.1_f64..=0.9_f64,
        azimuth in 0.2_f64..=1.0_f64,
    ) {
        let negative_degree = degree
            .checked_neg()
            .expect("the generated degree is safely negatable");
        prop_assume!(order >= negative_degree && order <= degree);
        let degree_float = f64::from(degree);
        let order_float = f64::from(order);
        let degree_number = Number::from_float(degree_float);
        let order_number = Number::from_float(order_float);
        let argument_number = Number::from_float(argument);
        let azimuth_number = Number::from_float(azimuth);
        assert_number(
            &Expr::number(argument).assoc_legendre(degree_float, order_float),
            argument_number.assoc_legendre(&degree_number, &order_number),
            "assoc_legendre must preserve degree, order, and value positions",
        );
        let expected = argument_number.spherical_harmonic(
            &degree_number,
            &order_number,
            &azimuth_number,
        );
        assert_number(
            &Expr::number(argument).spherical_harmonic(
                degree_float,
                order_float,
                azimuth,
            ),
            expected,
            "spherical_harmonic must preserve all argument positions",
        );
    }

    #[test]
    fn substitution_rebuilds_the_same_approximate_constructor(input in -1.0_f64..=1.0_f64) {
        let expression = Expr::symbol("numeric_approximate_rewrite_x");
        let symbol = expression.as_symbol().expect("symbol expression has an identity");
        let expected = Expr::number(input).sin();
        prop_assert_eq!(
            expression.sin().substitute_symbol(symbol, &Expr::number(input)),
            expected,
        );
    }

    #[test]
    fn rewrites_rebuild_approximate_cosine_calls(input in -1.0_f64..=1.0_f64) {
        let x = Expr::symbol("numeric_approximate_rewrite_edge");
        let capture = Pattern::capture("argument", None);
        let rule = RewriteRule::new(capture.sin(), capture.cos())
            .expect("valid rewrite rule");
        let rewritten = rule.apply_all(&x.sin(), None, None)
            .expect("unrestricted matching cannot exhaust a limit");
        let symbol = x.as_symbol().expect("symbol expression has an identity");
        let expected = Expr::number(Number::from_float(input).cos());
        prop_assert_eq!(
            rewritten.substitute_symbol(symbol, &Expr::number(input)),
            expected,
        );
    }

    #[test]
    fn continuous_hermite_degrees_preserve_real_and_imaginary_components(
        degree in 0_i32..=6,
        offset in -0.49_f64..=0.49,
        imaginary in -2.0_f64..=2.0,
        input in -1.0_f64..=1.0,
    ) {
        let receiver = Number::from_float(input);
        let real = f64::from(degree) + offset;
        let complex_order = Number::from_complex(
            Number::from_float(real), Number::from_float(imaginary),
        );
        let expected_real = receiver.hermite(&Number::from_float(real));
        let expected_complex = receiver.hermite(&complex_order);
        prop_assume!(expected_real.is_finite() && expected_complex.is_finite());
        prop_assert_eq!(Expr::number(receiver.clone()).hermite(real), Expr::number(expected_real));
        prop_assert_eq!(Expr::number(receiver).hermite(complex_order), Expr::number(expected_complex));
    }

    #[test]
    fn unrepresentable_hermite_degrees_remain_symbolic(
        degree in f64::from_bits(0x43e0_0000_0000_0000)..=f64::MAX,
        input in -1.0_f64..=1.0_f64,
    ) {
        prop_assert!(matches!(Expr::number(input).hermite(degree).view(), ExprView::Function { .. }), "an unrepresentable degree stays symbolic");
    }
}

#[test]
fn signed_zero_is_preserved_by_approximate_sine() {
    let expression = Expr::number(-0.0).sin();
    let value = expression
        .as_number()
        .expect("finite approximate sine should fold")
        .extract_complex()
        .expect("every Number has complex components");
    assert!(matches!(value.re, Real::Float(_)));
    assert_eq!(value.re.to_f64().to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn continuous_degrees_fold_and_nonfinite_calls_remain_symbolic() {
    let half = Expr::number(Number::from_float(0.25));
    assert_number(
        &half.hermite(1.5),
        Number::from_float(0.25).hermite(&Number::from_float(1.5)),
        "Hermite retains its continuous degree",
    );
    for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_function(&Expr::number(input).sin(), "nonfinite inputs stay symbolic");
    }
    assert_function(
        &Expr::number(f64::INFINITY).tanh(),
        "an infinite tanh input stays symbolic",
    );
    assert_function(
        &Expr::number(0.0).ln(),
        "a nonfinite logarithm result stays symbolic",
    );
}

#[test]
fn invalid_arities_fail_and_custom_and_rational_calls_remain_symbolic() {
    assert!(
        Function::builtin("sin")
            .expect("sine is a known builtin")
            .call(vec![Expr::number(0.0), Expr::number(1.0)])
            .is_err()
    );
    assert!(Function::builtin("numeric_unknown_approximate").is_none());
    let context = Context::new();
    let local = context
        .register_function("custom_sin", UserFunction::new(1..=1))
        .expect("the custom name is unbound");
    assert_function(
        &local
            .call(vec![Expr::number(0.0)])
            .expect("the custom function is unary"),
        "registered custom functions remain symbolic",
    );

    let half = Expr::number(Number::from_rational(RationalType::new(1, 2)));
    assert_function(&half.sin(), "rational sine remains symbolic");
    assert_function(&half.sqrt(), "rational square root remains symbolic");
}
