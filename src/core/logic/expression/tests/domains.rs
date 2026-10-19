//! Conservative scalar proofs and undefined-value counterexamples.

use core::ops::{Add, Div, Mul};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, Property, Truth, UserFunction};

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
        let context = Context::new();
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
        let context = Context::new();
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
        let context = Context::new();
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
        let facts = expression.properties();
        let sample = Expr::number(Number::from_complex(Number::from_float(f64::from(real)), Number::from_float(f64::from(imaginary))));
        let evaluated = expression.substitute_symbol(x, &sample);
        let value = evaluated.as_number().expect("bounded approximate inputs produce numeric values");
        let observations = [
            (Property::Finite, value.is_finite()),
            (Property::Real, value.is_finite() && value.is_real()),
            (Property::Integer, value.is_finite() && value.is_int()),
            (Property::Zero, value.is_zero()),
            (Property::Nonzero, value.is_finite() && !value.is_zero()),
            (Property::Positive, value.is_positive()),
            (Property::Negative, value.is_negative()),
            (Property::Nonnegative, value.is_real() && !value.is_negative()),
            (Property::Nonpositive, value.is_real() && !value.is_positive()),
        ];
        for (property, observed) in observations {
            match facts.get(property) {
                Truth::True => prop_assert!(observed, "a proven fact must hold for the sample: {:?}", property),
                Truth::False => prop_assert!(!observed, "a refuted fact must fail for the sample: {:?}", property),
                Truth::Unknown => {}
            }
        }
    }
}

#[test]
fn complex_values_unknown_functions_and_nonfinite_literals_do_not_gain_real_proofs() {
    let context = Context::new();
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
            let mut expression = Expr::symbol("deep_property_x");
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
