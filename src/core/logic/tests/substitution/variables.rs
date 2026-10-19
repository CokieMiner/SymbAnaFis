//! Explicit inverse substitutions, scalar elimination, and retained identities.

use core::ops::{Add, Div, Mul, Neg, Sub};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, Substitution, Substitutions, UserFunction};

proptest! {
    #[test]
    fn chosen_inverses_replace_all_scalar_positions_and_shared_edges(
        inverse_kind in 0_usize..6,
        power in 2_i64..=6,
        numerator in -5_i64..=5,
        denominator in 1_i64..=5,
        coefficient in -5_i64..=5,
        edges in 1_usize..=6,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let inverse = match inverse_kind {
            0 => u.sqrt(),
            1 => Neg::neg(u.sqrt()),
            2 => u.ln(),
            3 => Div::div(Expr::number(1), u),
            4 => Div::div(Sub::sub(u, y), 3),
            _ => Add::add(u.cbrt(), y),
        };
        let f = context.register_function("f", UserFunction::new(2..=2))
            .expect("the function name is unbound");
        let tuple = context.register_function("tuple", UserFunction::default())
            .expect("the function name is unbound");
        let exponent = Expr::number(Number::from_rational(RationalType::new(numerator, denominator)));
        let construct = |base: &Expr| {
            let shared = f.call(vec![base.sin(), base.pow(2)]).expect("f is binary");
            let mut arguments = vec![
                base.clone(),
                Expr::sum(vec![base.pow(power), base.pow(power.saturating_add(1)), y.pow(2), y.pow(3)]),
                Expr::product(vec![Expr::number(coefficient), base.clone(), base.sin(), y.into()]),
                Div::div(Expr::number(1), base),
                base.pow(&exponent),
                y.pow(base),
                base.pow(2).sqrt(),
                base.pow(0),
                Expr::derivative(y.abs(), y, 2),
            ];
            arguments.extend(vec![shared; edges]);
            tuple.call(arguments).expect("tuple accepts any arity")
        };
        let input = construct(&x.to_expr());
        let expected = construct(&inverse);
        let result = Substitution::new(x.into(), inverse.clone()).apply_all(&input);
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x), "every scalar occurrence of the old identity is replaced");
        prop_assert!(result.contains_symbol(u));
        let simultaneous = Substitutions::new([(x.into(), inverse)]).expect("a single source cannot conflict");
        prop_assert_eq!(simultaneous.apply_all(&input), expected);
    }

    #[test]
    fn square_root_branches_preserve_values_on_the_chosen_real_half_line(
        magnitude in 1_i32..=6,
        copies in 1_i64..=4,
        negative in proptest::bool::ANY,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let inverse = if negative { Neg::neg(u.sqrt()) } else { u.sqrt() };
        let power = copies.saturating_mul(2).saturating_add(1);
        let input = x.pow(power);
        let result = Substitution::new(x.into(), inverse).apply_all(&input);
        let exponent = Number::from_rational(RationalType::new(power, 2));
        let expected = if negative { Neg::neg(u.pow(exponent)) } else { u.pow(exponent) };
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x));

        let signed = if negative { magnitude.saturating_neg() } else { magnitude };
        let square = magnitude.saturating_mul(magnitude);
        let evaluated = result.substitute_symbol(u, &Expr::number(f64::from(square)));
        let original = input.substitute_symbol(x, &Expr::number(f64::from(signed)));
        let actual = evaluated.as_number().expect("finite approximate powers fold");
        let wanted = original.as_number().expect("finite approximate powers fold");
        let tolerance = Mul::mul(Number::from_float(1e-10), Add::add(Number::one(), wanted.abs()));
        prop_assert!(Sub::sub(actual, wanted).abs() < tolerance);
    }

    #[test]
    fn variable_elimination_preserves_equal_names_from_another_context(power in 2_i64..=8) {
        let context = Context::new();
        let other_context = Context::new();
        let x = context.symb("x")?;
        let other_x = other_context.symb("x")?;
        let u = context.symb("u")?;
        let inverse = u.sqrt();
        let input = Add::add(x.pow(power), other_x.pow(power));
        let expected = Add::add(inverse.pow(power), other_x.pow(power));
        let result = Substitution::new(x.into(), inverse).apply_all(&input);
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x));
        prop_assert!(result.contains_symbol(other_x));
        prop_assert!(result.contains_var("x"));
    }

    #[test]
    fn simultaneous_inverse_substitutions_eliminate_both_old_variables(power in 2_i64..=8) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let v = context.symb("v")?;
        let f = context.register_function("f", UserFunction::new(2..=2))
            .expect("the function name is unbound");
        let input = Add::add(Mul::mul(x.pow(power), y.sin()), f.call(vec![x.into(), y.into()]).expect("f is binary"));
        let first = u.sqrt();
        let second = v.ln();
        let expected = Add::add(Mul::mul(first.pow(power), second.sin()), f.call(vec![first.clone(), second.clone()]).expect("f is binary"));
        let substitutions = Substitutions::new([(x.into(), first), (y.into(), second)])
            .expect("the source symbols have distinct identities");
        let result = substitutions.apply_all(&input);
        prop_assert_eq!(&result, &expected);
        prop_assert!(!result.contains_symbol(x));
        prop_assert!(!result.contains_symbol(y));
    }
}

#[test]
fn substituting_after_differentiation_uses_the_evaluated_expression() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let derivative = Expr::derivative(x.pow(2), x, 1);
    let result = Substitution::new(x.into(), u.sqrt()).apply_all(&derivative);
    assert_eq!(result, 2 * u.sqrt());
}
