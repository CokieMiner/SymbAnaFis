//! Conservative scalar proofs and undefined-value counterexamples.

use core::ops::{Add, Div, Mul, Neg};
use std::thread;

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr, Property, Substitution, Symbol, Truth, UserFunction},
    errors::{FunctionError, SymbolError},
};

fn assert_observed_properties(
    expression: &Expr,
    value: &Number,
) -> Result<(), proptest::test_runner::TestCaseError> {
    let facts = expression.properties();
    let observations = [
        (Property::Finite, value.is_finite()),
        (Property::Real, value.is_finite() && value.is_real()),
        (Property::Integer, value.is_finite() && value.is_int()),
        (Property::Zero, value.is_zero()),
        (Property::Nonzero, value.is_finite() && !value.is_zero()),
        (Property::Positive, value.is_positive()),
        (Property::Negative, value.is_negative()),
        (
            Property::Nonnegative,
            value.is_real() && !value.is_negative(),
        ),
        (
            Property::Nonpositive,
            value.is_real() && !value.is_positive(),
        ),
    ];
    for (property, observed) in observations {
        match facts.get(property) {
            Truth::True => prop_assert!(
                observed,
                "a proven fact must hold for the sample ({expression}): {:?}",
                property
            ),
            Truth::False => prop_assert!(
                !observed,
                "a refuted fact must fail for the sample ({expression}): {:?}",
                property
            ),
            Truth::Unknown => {}
        }
    }
    Ok(())
}

proptest! {
    #[test]
    fn exact_literals_have_consistent_membership(real in -100_i64..=100, imaginary in -20_i64..=20, denominator in 1_u64..=31) {
        let number = Number::from_complex(Number::from_rational(RationalType::from_parts(real, denominator)), Number::from_int(imaginary));
        let facts = Expr::number(number.clone()).properties();
        prop_assert_eq!(facts.get(Property::Finite), Truth::True);
        prop_assert_eq!(facts.get(Property::Real), if imaginary == 0 { Truth::True } else { Truth::False });
        prop_assert_eq!(facts.get(Property::Integer), if number.is_int() { Truth::True } else { Truth::False });
        prop_assert_eq!(facts.get(Property::Zero), if number.is_zero() { Truth::True } else { Truth::False });
        prop_assert_eq!(facts.get(Property::Nonzero), if number.is_zero() { Truth::False } else { Truth::True });
        prop_assert_eq!(facts.get(Property::Positive), if number.is_positive() { Truth::True } else { Truth::False });
        prop_assert_eq!(facts.get(Property::Negative), if number.is_negative() { Truth::True } else { Truth::False });
    }

    #[test]
    fn real_nonnegative_compositions_and_poles_keep_their_proofs(power in 1_i64..=15, offset in 1_i64..=20) {
        let context = Context::new()?;
        let x = context.symb("x")?.to_expr();
        let magnitude = x.abs();
        prop_assert_eq!(magnitude.properties().get(Property::Nonnegative), Truth::True);
        let positive = Add::add(magnitude.pow(power), Expr::number(offset));
        let facts = positive.properties();
        prop_assert_eq!(facts.get(Property::Positive), Truth::True);
        prop_assert_eq!(positive.exp().properties().get(Property::Positive), Truth::True);
        prop_assert_eq!(positive.cosh().properties().get(Property::Positive), Truth::True);
        prop_assert_eq!(Div::div(Expr::number(1), &positive).properties().get(Property::Positive), Truth::True);
        prop_assert_eq!(Div::div(Expr::number(0), &x).properties().get(Property::Finite), Truth::Unknown);
        prop_assert_eq!(Div::div(Expr::number(0), &x).properties().get(Property::Zero), Truth::Unknown);
    }

    #[test]
    fn compact_polynomials_have_the_same_sign_proofs_as_their_terms(first in 2_i64..=6, second in 8_i64..=13) {
        let context = Context::new()?;
        let base = context.symb("x")?.abs();
        let polynomial = Expr::sum(vec![base.pow(first), base.pow(second)]);
        let facts = polynomial.properties();
        prop_assert_eq!(facts.get(Property::Finite), Truth::True);
        prop_assert_eq!(facts.get(Property::Real), Truth::True);
        prop_assert_eq!(facts.get(Property::Nonnegative), Truth::True);
        prop_assert_eq!(facts.get(Property::Positive), Truth::Unknown);
        prop_assert_eq!(Add::add(polynomial, 1).properties().get(Property::Positive), Truth::True);
    }

    #[test]
    fn proven_properties_agree_with_finite_complex_samples(real in -3_i32..=3, imaginary in -3_i32..=3, kind in 0_u8..10) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let expression = match kind {
            0 => x.abs(),
            1 => x.exp(),
            2 => x.abs().exp(),
            3 => x.sqrt(),
            4 => x.pow(2),
            5 => Add::add(x.abs().pow(2), 1),
            6 => x.sin(),
            7 => Div::div(Expr::number(1), Add::add(x.abs(), 1)),
            8 => Mul::mul(x.abs(), x.abs().cosh()),
            _ => x.abs().floor(),
        };
        let sample = Expr::number(Number::from_complex(Number::from_float(f64::from(real)), Number::from_float(f64::from(imaginary))));
        let evaluated = Substitution::new(x.into(), sample).apply_all(&expression);
        let value = evaluated.as_number().expect("bounded approximate inputs produce numeric values");
        assert_observed_properties(&expression, value)?;
    }

    #[test]
    fn real_and_complex_builtin_proofs_agree_with_sampled_evaluations(
        real in -4_i32..=4,
        imaginary in -4_i32..=4,
        half_step in 0_u8..=1,
    ) {
        let context = Context::new()?;
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let complex_sample = Expr::number(Number::from_complex(
            Number::from_float(f64::from(real)),
            Number::from_float(f64::from(imaginary)),
        ));
        for expr in [
            x.sinh(),
            x.asinh(),
            x.erf(),
            x.erfc(),
            x.exp_m1(),
            x.signum(),
            x.exp().signum(),
            x.re(),
            x.im(),
            x.conj(),
            x.hypot(y),
        ] {
            let substituted = Substitution::new(x.into(), complex_sample.clone()).apply_all(&expr);
            let evaluated = Substitution::new(y.into(), complex_sample.clone()).apply_all(&substituted);
            if let Some(value) = evaluated.as_number() {
                assert_observed_properties(&expr, value)?;
            }
        }

        let real_input = x.abs().sin();
        let positive = Add::add(x.abs(), 0.5);
        let negative = Neg::neg(&positive);
        let nonnegative = x.abs();
        let nonpositive = Neg::neg(&nonnegative);
        let step = if half_step == 0 { 0.0 } else { 0.5 };
        let real_sample = Expr::number(f64::from(real) + step);
        let real_non_zero_sample = Expr::number(if real == 0 && half_step == 0 {
            0.25
        } else {
            f64::from(real) + step
        });
        let real_expressions = [
            real_input.sinh(),
            positive.sinh(),
            negative.sinh(),
            nonnegative.sinh(),
            nonpositive.sinh(),
            real_input.asinh(),
            positive.asinh(),
            negative.asinh(),
            real_input.erf(),
            positive.erf(),
            negative.erf(),
            real_input.erfc(),
            positive.erfc(),
            negative.erfc(),
            real_input.exp_m1(),
            positive.exp_m1(),
            negative.exp_m1(),
            real_input.tanh(),
            positive.tanh(),
            negative.tanh(),
            nonnegative.tanh(),
            nonpositive.tanh(),
            real_input.atan(),
            positive.atan(),
            negative.atan(),
            nonnegative.atan(),
            nonpositive.atan(),
            real_input.signum(),
            positive.signum(),
            negative.signum(),
            nonnegative.signum(),
            nonpositive.signum(),
            nonnegative.ln_1p(),
            positive.ln_1p(),
            real_input.floor(),
            real_input.ceil(),
            real_input.trunc(),
            real_input.round(),
            positive.floor(),
            positive.ceil(),
            positive.trunc(),
            positive.round(),
            negative.floor(),
            negative.ceil(),
            negative.trunc(),
            negative.round(),
            nonnegative.floor(),
            nonnegative.ceil(),
            nonpositive.floor(),
            nonpositive.ceil(),
            positive.atan2(&real_input),
            negative.atan2(&real_input),
            nonnegative.atan2(&positive),
            nonnegative.atan2(&negative),
            nonpositive.atan2(&positive),
            Expr::number(0).atan2(&positive),
            Expr::number(0).atan2(&negative),
            real_input.re(),
            real_input.im(),
            real_input.conj(),
            real_input.heaviside(),
            positive.heaviside(),
            negative.heaviside(),
            nonnegative.heaviside(),
            nonpositive.heaviside(),
            positive.hypot(&real_input),
            nonnegative.hypot(&negative),
            real_input.max(&positive),
            negative.max(&nonpositive),
            real_input.min(&negative),
            positive.min(&nonnegative),
            real_input.clamp(&negative, &positive),
            real_input.clamp(&positive, Add::add(&positive, 1)),
            real_input.modulo(&positive),
            real_input.modulo(&negative),
            nonnegative.rem(&positive),
            nonpositive.rem(&negative),
        ];
        for expr in real_expressions {
            let evaluated = Substitution::new(x.into(), real_sample.clone()).apply_all(&expr);
            if let Some(value) = evaluated.as_number() {
                assert_observed_properties(&expr, value)?;
            }
        }
        // Integer-input identities for `abs`, `gamma`, `beta`, and rounding.
        let int_expr = real_input.floor();
        let signed_int_expr = real_input.floor();
        let positive_int = Add::add(int_expr.abs(), 1);
        let negative_int = Neg::neg(&positive_int);
        let exact_sample = Expr::number(i64::from(real));
        let positive_exact = Expr::number(i64::from(real.abs()).saturating_add(1));
        for expr in [
            signed_int_expr.abs(),
            positive_int.gamma(),
            positive_int.floor(),
            positive_int.ceil(),
            positive_int.trunc(),
            positive_int.round(),
            negative_int.floor(),
            negative_int.ceil(),
            negative_int.trunc(),
            negative_int.round(),
            y.abs().floor(),
        ] {
            let substituted = Substitution::new(x.into(), exact_sample.clone()).apply_all(&expr);
            let evaluated =
                Substitution::new(y.into(), real_non_zero_sample.clone()).apply_all(&substituted);
            if let Some(value) = evaluated.as_number() {
                assert_observed_properties(&expr, value)?;
            }
        }
        let exact_gamma = positive_int.gamma();
        let gamma_eval = Substitution::new(x.into(), positive_exact).apply_all(&exact_gamma);
        if let Some(value) = gamma_eval.as_number() {
            assert_observed_properties(&exact_gamma, value)?;
        }
    }
}

#[test]
fn complex_values_unknown_functions_and_nonfinite_literals_do_not_gain_real_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    assert_eq!(x.to_expr().properties().get(Property::Real), Truth::Unknown);
    assert_eq!(
        x.pow(2).properties().get(Property::Nonnegative),
        Truth::Unknown
    );
    assert_eq!(x.exp().properties().get(Property::Positive), Truth::Unknown);
    assert_eq!(
        Expr::i().exp().properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        Expr::number(-2).sqrt().properties().get(Property::Real),
        Truth::False
    );
    let shifted_imaginary = Add::add(x.abs(), Expr::i());
    assert_eq!(
        shifted_imaginary.properties().get(Property::Real),
        Truth::False
    );
    assert_eq!(
        shifted_imaginary.properties().get(Property::Nonzero),
        Truth::True
    );
    for invalid_real_domain in [
        shifted_imaginary.heaviside(),
        shifted_imaginary.max(x),
        shifted_imaginary.min(x),
        shifted_imaginary.clamp(0, 1),
        x.clamp(0, &shifted_imaginary),
        shifted_imaginary.modulo(2),
        shifted_imaginary.rem(2),
    ] {
        assert_eq!(
            invalid_real_domain.properties().get(Property::Finite),
            Truth::False
        );
    }
    assert_eq!(
        Add::add(x.to_expr(), Expr::i())
            .properties()
            .get(Property::Real),
        Truth::Unknown
    );
    let custom = context
        .register_function("custom_domain", UserFunction::new(1..=1))
        .expect("unbound function");
    let call = custom.call(vec![x.into()]).expect("unary function");
    assert_eq!(
        Mul::mul(Expr::number(0), call)
            .properties()
            .get(Property::Zero),
        Truth::Unknown
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let facts = Expr::number(value).properties();
        assert_eq!(facts.get(Property::Finite), Truth::False);
        assert_eq!(facts.get(Property::Positive), Truth::False);
        assert_eq!(facts.get(Property::Nonzero), Truth::False);
    }
}

#[test]
fn property_analysis_and_destruction_fit_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let mut expression = Symbol::intern("deep_property_x").expect("symbol").to_expr();
            for _ in 0..4096 {
                expression = expression.abs();
            }
            assert_eq!(
                expression.properties().get(Property::Nonnegative),
                Truth::True
            );
        })
        .expect("small-stack thread starts")
        .join()
        .expect("analysis and release remain iterative");
}

#[test]
fn nonfinite_arguments_do_not_refute_unproved_composition_domains() {
    let infinity = Expr::number(f64::INFINITY);
    let negative_infinity = Expr::number(f64::NEG_INFINITY);
    assert_eq!(
        negative_infinity.exp().properties().get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        Div::div(Expr::number(0), infinity.clone())
            .properties()
            .get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        infinity.pow(0).properties().get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        Div::div(Expr::number(1), Expr::number(0))
            .properties()
            .get(Property::Finite),
        Truth::False
    );
    assert_eq!(
        Expr::number(0).ln().properties().get(Property::Finite),
        Truth::False
    );
}

#[test]
fn sign_conjunctions_close_into_strict_signs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    for nonnegative_and_nonzero in [x.exp().abs(), Expr::i().abs()] {
        let facts = nonnegative_and_nonzero.properties();
        assert_eq!(facts.get(Property::Positive), Truth::True);
        assert_eq!(facts.get(Property::Nonpositive), Truth::False);
    }
    let negative = Mul::mul(Expr::number(-1), x.exp().abs());
    assert_eq!(negative.properties().get(Property::Negative), Truth::True);
    assert_eq!(x.abs().properties().get(Property::Positive), Truth::Unknown);
}

#[test]
fn abs_and_erfc_domain_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let real = x.abs().sin();
    let rational = Div::div(Expr::number(1), Add::add(real.floor().abs(), 2));

    assert_eq!(
        x.floor().abs().properties().get(Property::Integer),
        Truth::Unknown
    );
    assert_eq!(
        real.floor().abs().properties().get(Property::Integer),
        Truth::True
    );
    assert_eq!(
        rational.abs().properties().get(Property::Rational),
        Truth::True
    );
    assert_eq!(
        rational.abs().properties().get(Property::Integer),
        Truth::Unknown
    );
    let complex_nz = Add::add(Expr::number(1), Mul::mul(x.abs(), Expr::i()));
    assert_eq!(
        complex_nz.abs().properties().get(Property::Integer),
        Truth::Unknown
    );
    assert_eq!(
        complex_nz.abs().properties().get(Property::Rational),
        Truth::Unknown
    );

    assert_eq!(x.erfc().properties().get(Property::Finite), Truth::True);
    assert_eq!(x.erfc().properties().get(Property::Real), Truth::Unknown);
    assert_eq!(
        x.sin().erfc().properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        real.erfc().properties().get(Property::Positive),
        Truth::True
    );
}

#[test]
fn monotonic_zero_and_signum_domain_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let real = x.abs().sin();
    let positive = Add::add(x.abs(), 0.5);
    let negative = Neg::neg(&positive);
    let nonnegative = x.abs();
    let nonpositive = Neg::neg(&nonnegative);
    let zero = Mul::mul(Expr::number(0), x.to_expr());

    for expr in [x.sinh(), x.asinh(), x.erf(), x.exp_m1(), x.signum()] {
        assert_eq!(expr.properties().get(Property::Finite), Truth::True);
        assert_eq!(expr.properties().get(Property::Real), Truth::Unknown);
    }
    for expr in [x.tanh(), x.atan()] {
        assert_eq!(expr.properties().get(Property::Finite), Truth::Unknown);
        assert_eq!(expr.properties().get(Property::Real), Truth::Unknown);
    }
    for builder in [
        Expr::sinh,
        Expr::asinh,
        Expr::erf,
        Expr::exp_m1,
        Expr::tanh,
        Expr::atan,
        Expr::signum,
    ] {
        assert_eq!(builder(&real).properties().get(Property::Real), Truth::True);
        assert_eq!(
            builder(&real).properties().get(Property::Positive),
            Truth::Unknown
        );
        assert_eq!(
            builder(&positive).properties().get(Property::Positive),
            Truth::True
        );
        assert_eq!(
            builder(&negative).properties().get(Property::Negative),
            Truth::True
        );
        assert_eq!(
            builder(&nonnegative)
                .properties()
                .get(Property::Nonnegative),
            Truth::True
        );
        assert_eq!(
            builder(&nonnegative).properties().get(Property::Positive),
            Truth::Unknown
        );
        assert_eq!(
            builder(&nonpositive)
                .properties()
                .get(Property::Nonpositive),
            Truth::True
        );
        assert_eq!(builder(&zero).properties().get(Property::Zero), Truth::True);
    }

    assert_eq!(
        x.exp().signum().properties().get(Property::Nonzero),
        Truth::True
    );
    assert_eq!(
        x.exp().signum().properties().get(Property::Integer),
        Truth::Unknown
    );
    assert_eq!(
        real.signum().properties().get(Property::Integer),
        Truth::True
    );
}

#[test]
fn ln1p_and_gamma_property_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let real = x.abs().sin();
    let positive = Add::add(x.abs(), 0.5);
    let negative = Neg::neg(&positive);
    let nonnegative = x.abs();
    let zero = Mul::mul(Expr::number(0), x.to_expr());
    let positive_int = Add::add(real.floor().abs(), 1);

    assert_eq!(
        nonnegative.ln_1p().properties().get(Property::Nonnegative),
        Truth::True
    );
    assert_eq!(
        nonnegative.ln_1p().properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        positive.ln_1p().properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(zero.ln_1p().properties().get(Property::Zero), Truth::True);
    assert_eq!(
        negative.ln_1p().properties().get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        real.ln_1p().properties().get(Property::Real),
        Truth::Unknown
    );

    assert_eq!(
        positive.gamma().properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        positive.gamma().properties().get(Property::Integer),
        Truth::Unknown
    );
    assert_eq!(
        positive_int.gamma().properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        positive_int.gamma().properties().get(Property::Integer),
        Truth::True
    );
    assert_eq!(
        nonnegative.gamma().properties().get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        negative.gamma().properties().get(Property::Real),
        Truth::Unknown
    );
}

#[test]
fn rounding_property_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let real = x.abs().sin();
    let positive = Add::add(x.abs(), 0.5);
    let negative = Neg::neg(&positive);
    let nonnegative = x.abs();
    let nonpositive = Neg::neg(&nonnegative);
    let zero = Mul::mul(Expr::number(0), x.to_expr());
    let positive_int = Add::add(real.floor().abs(), 1);
    let negative_int = Neg::neg(&positive_int);

    for builder in [Expr::floor, Expr::ceil, Expr::trunc, Expr::round] {
        assert_eq!(
            builder(&real).properties().get(Property::Integer),
            Truth::True
        );
        assert_eq!(builder(&zero).properties().get(Property::Zero), Truth::True);
        assert_eq!(
            builder(&nonnegative)
                .properties()
                .get(Property::Nonnegative),
            Truth::True
        );
        assert_eq!(
            builder(&nonpositive)
                .properties()
                .get(Property::Nonpositive),
            Truth::True
        );
        assert_eq!(
            builder(&positive_int).properties().get(Property::Positive),
            Truth::True
        );
        assert_eq!(
            builder(&negative_int).properties().get(Property::Negative),
            Truth::True
        );
    }
    assert_eq!(
        positive.ceil().properties().get(Property::Positive),
        Truth::True
    );
    for expr in [positive.floor(), positive.trunc(), positive.round()] {
        assert_eq!(expr.properties().get(Property::Positive), Truth::Unknown);
    }
    assert_eq!(
        negative.floor().properties().get(Property::Negative),
        Truth::True
    );
    for expr in [negative.ceil(), negative.trunc(), negative.round()] {
        assert_eq!(expr.properties().get(Property::Negative), Truth::Unknown);
    }
}

#[test]
fn atan2_property_proofs() {
    assert!(Number::zero().atan2(&Number::from_int(-1)).is_positive());
    assert!(
        Number::from_float(-0.0)
            .atan2(&Number::from_int(-1))
            .is_negative()
    );
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    let real = x.abs().sin();
    let positive = Add::add(x.abs(), 0.5);
    let negative = Neg::neg(&positive);
    let nonnegative = x.abs();
    let nonpositive = Neg::neg(&nonnegative);
    let zero = Mul::mul(Expr::number(0), x.to_expr());
    let neg_zero = Mul::mul(Expr::number(-0.0), x.abs());
    let real_y = y.abs().sin();
    let pos_y = Add::add(y.abs(), 0.5);
    let neg_y = Neg::neg(&pos_y);

    assert_eq!(x.atan2(y).properties().get(Property::Real), Truth::Unknown);
    assert_eq!(
        real.atan2(&real_y).properties().get(Property::Real),
        Truth::Unknown
    );
    assert_eq!(
        positive.atan2(Expr::i()).properties().get(Property::Real),
        Truth::Unknown
    );
    assert_eq!(
        real.atan2(&pos_y).properties().get(Property::Real),
        Truth::True
    );
    assert_eq!(
        real.atan2(&pos_y).properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        real.atan2(&neg_y).properties().get(Property::Nonzero),
        Truth::True
    );
    assert_eq!(
        positive.atan2(&real_y).properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        negative.atan2(&real_y).properties().get(Property::Negative),
        Truth::True
    );
    assert_eq!(
        zero.atan2(&pos_y).properties().get(Property::Zero),
        Truth::True
    );
    // Signed zero in IEEE 754 yields $\pm\pi$ along the negative real axis.
    for left in [&zero, &neg_zero, &nonnegative, &nonpositive] {
        let facts = left.atan2(&neg_y).properties();
        assert_eq!(facts.get(Property::Real), Truth::True);
        assert_eq!(facts.get(Property::Nonzero), Truth::True);
        assert_eq!(facts.get(Property::Positive), Truth::Unknown);
        assert_eq!(facts.get(Property::Nonnegative), Truth::Unknown);
        assert_eq!(facts.get(Property::Nonpositive), Truth::Unknown);
    }
    let signed_zero_branch = neg_zero.atan2(&neg_y);
    let substituted = Substitution::new(x.into(), Expr::number(1.0)).apply_all(&signed_zero_branch);
    let evaluated = Substitution::new(y.into(), Expr::number(1.0)).apply_all(&substituted);
    let sample = evaluated
        .as_number()
        .expect("signed-zero atan2 evaluates numerically");
    assert!(sample.is_negative());
    assert_observed_properties(&signed_zero_branch, sample)
        .expect("signed-zero atan2 properties remain sound");
    assert_eq!(
        nonnegative
            .atan2(&pos_y)
            .properties()
            .get(Property::Nonnegative),
        Truth::True
    );
    assert_eq!(
        nonnegative
            .atan2(&pos_y)
            .properties()
            .get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        nonpositive
            .atan2(&pos_y)
            .properties()
            .get(Property::Nonpositive),
        Truth::True
    );
}

#[test]
fn beta_property_proofs() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let y = context.symb("y").expect("symbol allocation");
    let positive = Add::add(x.abs(), 0.5);
    let negative = Neg::neg(&positive);
    let nonnegative = x.abs();
    let pos_y = Add::add(y.abs(), 0.5);
    let pos_int_x = Add::add(x.abs().sin().floor().abs(), 1);
    let pos_int_y = Add::add(y.abs().sin().floor().abs(), 1);

    assert_eq!(x.beta(y).properties().get(Property::Finite), Truth::Unknown);
    assert_eq!(
        positive.beta(&pos_y).properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        positive.beta(&pos_y).properties().get(Property::Rational),
        Truth::Unknown
    );
    assert_eq!(
        pos_int_x
            .beta(&pos_int_y)
            .properties()
            .get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        pos_int_x
            .beta(&pos_int_y)
            .properties()
            .get(Property::Rational),
        Truth::True
    );
    assert_eq!(
        nonnegative.beta(&pos_y).properties().get(Property::Finite),
        Truth::Unknown
    );
    assert_eq!(
        negative.beta(&pos_y).properties().get(Property::Positive),
        Truth::Unknown
    );
}

#[test]
fn symbol_and_user_function_properties_propagate_and_invalidate_settled_caches() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let root_square = x.pow(2).sqrt();

    // Default symbol has only Finite proved, so sqrt(x^2) settles unchanged.
    assert_eq!(x.properties().get(Property::Finite), Truth::True);
    assert_eq!(x.properties().get(Property::Real), Truth::Unknown);
    assert_eq!(
        root_square.properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(root_square.compact().expect("compact"), root_square);

    // Contradictory property sets are rejected without mutating the symbol.
    assert_eq!(
        x.set_properties(&[Property::Positive, Property::Negative]),
        Err(SymbolError::ContradictoryProperties)
    );
    assert_eq!(
        x.set_properties(&[Property::Zero, Property::Nonzero]),
        Err(SymbolError::ContradictoryProperties)
    );
    assert_eq!(
        x.set_properties(&[
            Property::Nonnegative,
            Property::Nonpositive,
            Property::Nonzero,
        ]),
        Err(SymbolError::ContradictoryProperties)
    );
    assert_eq!(x.properties().get(Property::Real), Truth::Unknown);

    // Setting Nonnegative + Nonzero closes to Positive and invalidates both
    // cached node facts and settled simplification masks on existing expressions.
    x.set_properties(&[Property::Nonnegative, Property::Nonzero])
        .expect("consistent properties");
    assert_eq!(x.properties().get(Property::Positive), Truth::True);
    assert_eq!(x.properties().get(Property::Real), Truth::True);
    assert_eq!(x.properties().get(Property::Zero), Truth::False);
    assert_eq!(
        root_square.properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(
        root_square.compact().expect("compact after epoch"),
        x.to_expr()
    );
    assert_eq!(x.abs().derivative(x, 1), Expr::number(1));

    // Resetting with an empty slice restores the default domain and invalidates caches again.
    x.set_properties(&[]).expect("empty property set resets");
    assert_eq!(x.properties().get(Property::Real), Truth::Unknown);
    assert_eq!(
        root_square.properties().get(Property::Positive),
        Truth::Unknown
    );
    assert_eq!(
        root_square.compact().expect("compact after reset"),
        root_square
    );

    // UserFunction return properties validate contradictions and propagate on finite args.
    assert!(matches!(
        UserFunction::new(1..=1).returns(&[Property::Positive, Property::Zero]),
        Err(FunctionError::ContradictoryProperties)
    ));
    let definition = UserFunction::new(1..=1)
        .returns(&[Property::Positive, Property::Integer])
        .expect("consistent function properties");
    assert_eq!(definition.properties().get(Property::Rational), Truth::True);
    let positive_fn = context
        .register_function("positive_int_fn", definition)
        .expect("register");
    let call = positive_fn.call(vec![x.into()]).expect("call");
    assert_eq!(call.properties().get(Property::Positive), Truth::True);
    assert_eq!(call.properties().get(Property::Integer), Truth::True);
    let nonfinite_call = positive_fn
        .call(vec![Expr::number(f64::NAN)])
        .expect("call on NaN");
    assert_eq!(
        nonfinite_call.properties().get(Property::Positive),
        Truth::Unknown
    );
}

#[test]
fn concurrent_property_updates_never_poison_cached_facts_or_settled_masks() {
    let context = Context::new().expect("context");
    let x = context.symb("x").expect("symbol allocation");
    let root_square = x.pow(2).sqrt();
    let writer_ctx = context.clone();
    let reader_expr = root_square.clone();

    let writer = thread::spawn(move || {
        let writer_x = writer_ctx.symb("x").expect("symbol lookup");
        for step in 0..400_u32 {
            if step % 2 == 0 {
                writer_x
                    .set_properties(&[Property::Positive])
                    .expect("positive");
            } else {
                writer_x.set_properties(&[]).expect("reset");
            }
        }
        writer_x
            .set_properties(&[Property::Positive])
            .expect("final positive");
    });

    let reader = thread::spawn(move || {
        for _ in 0..400_u32 {
            let props = reader_expr.properties();
            assert_eq!(props.get(Property::Finite), Truth::True);
            let simplified = reader_expr.compact().expect("compact");
            assert!(simplified == reader_expr || simplified.as_symbol().is_some());
        }
    });

    writer.join().expect("writer thread");
    reader.join().expect("reader thread");

    // Once the writer settles on Positive, cached facts and settled masks must
    // reflect Positive rather than any stale state from earlier epochs.
    assert_eq!(
        root_square.properties().get(Property::Positive),
        Truth::True
    );
    assert_eq!(root_square.compact().expect("final compact"), x.to_expr());
}
