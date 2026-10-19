//! Compressed factor counts, collected addends, and simultaneous remainder ownership.

use core::ops::{Div, Mul};

use alloc::vec;

use num_anafis::{Complex, Number};

use proptest::{prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, Substitution, Substitutions},
};

proptest! {
    #[test]
    fn positive_power_factors_replace_all_disjoint_copies_without_expansion(
        exponent in 1_i64..=14,
        requested in 2_i64..=5,
        coefficient in 1_i32..=6,
        x_value in -2_i64..=2,
        y_value in -2_i64..=2,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let source = x.pow(requested);
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(exponent), y.into()]);
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        let copies = exponent.checked_div(requested).expect("requested power is positive");
        let remainder = exponent.checked_rem(requested).expect("requested power is positive");
        let expected = if copies == 0 { input.clone() } else {
            let mut factors = vec![Expr::number(coefficient), u.pow(copies), y.into()];
            if remainder > 0 { factors.push(x.pow(remainder)); }
            Expr::product(factors)
        };
        prop_assert_eq!(&result, &expected);
        let mut values = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value))]);
        let source_value = source.substitute_symbols(&values);
        values.insert(u, source_value);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn mixed_power_groups_preserve_numeric_coefficients_and_other_factors(
        x_power in 3_i64..=11,
        y_power in 2_i64..=7,
        coefficient in 1_i32..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let source = Expr::product(vec![x.pow(2), y.into()]);
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(x_power), y.pow(y_power), z.into()]);
        let copies = x_power.checked_div(2).expect("two is nonzero").min(y_power);
        let x_remaining = x_power.saturating_sub(copies.saturating_mul(2));
        let y_remaining = y_power.saturating_sub(copies);
        let mut factors = vec![Expr::number(coefficient), u.pow(copies), z.into()];
        if x_remaining > 0 { factors.push(x.pow(x_remaining)); }
        if y_remaining > 0 { factors.push(y.pow(y_remaining)); }
        prop_assert_eq!(Substitution::new(source, u.into()).apply_all(&input), Expr::product(factors));
    }

    #[test]
    fn collected_positive_addends_match_repeated_sum_groups(
        x_count in 1_i32..=12,
        y_count in 1_i32..=12,
        x_value in -3_i64..=3,
        y_value in -3_i64..=3,
        z_value in -3_i64..=3,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let source = Expr::sum(vec![x.into(), y.into()]);
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(x_count), x.into()]), Expr::product(vec![Expr::number(y_count), y.into()]), z.into()]);
        let copies = x_count.min(y_count);
        let mut terms = vec![Expr::product(vec![Expr::number(copies), u.into()]), z.into()];
        if x_count > copies { terms.push(Expr::product(vec![Expr::number(x_count.saturating_sub(copies)), x.into()])); }
        if y_count > copies { terms.push(Expr::product(vec![Expr::number(y_count.saturating_sub(copies)), y.into()])); }
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::sum(terms));
        let mut values = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value)), (z, Expr::number(z_value))]);
        let source_value = source.substitute_symbols(&values);
        values.insert(u, source_value);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn collected_subtractions_keep_signed_units_and_unmatched_terms(count in 2_i32..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let source = Expr::sum(vec![x.into(), Expr::product(vec![Expr::number(-1), y.into()])]);
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(count), x.into()]), Expr::product(vec![Expr::number(count.saturating_add(1).saturating_neg()), y.into()])]);
        let expected = Expr::sum(vec![Expr::product(vec![Expr::number(count), u.into()]), Expr::product(vec![Expr::number(-1), y.into()])]);
        prop_assert_eq!(Substitution::new(source, u.into()).apply_all(&input), expected);
    }

    #[test]
    fn coefficients_signs_and_zero_factors_preserve_the_exact_remainder(
        coefficient in -12_i32..=-1,
        exponent in 3_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(exponent)]);
        let copies = exponent.checked_div(2).expect("two is nonzero");
        let remainder = exponent.checked_rem(2).expect("two is nonzero");
        let mut factors = vec![Expr::number(coefficient), u.pow(copies)];
        if remainder > 0 { factors.push(x.into()); }
        prop_assert_eq!(Substitution::new(x.pow(2), u.into()).apply_all(&input), Expr::product(factors));
        let zero = Expr::product(vec![Expr::number(0), x.pow(exponent)]);
        let weighted_source = Expr::product(vec![Expr::number(2), x.pow(2)]);
        let mut zero_factors = vec![Expr::number(0), u.pow(copies)];
        if remainder > 0 { zero_factors.push(x.into()); }
        prop_assert_eq!(Substitution::new(weighted_source, u.into()).apply_all(&zero), Expr::product(zero_factors));
    }

    #[test]
    fn fractional_powers_expose_integral_groups_and_zero_powers_remain_opaque(numerator in -12_i32..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let substitution = Substitution::new(x.pow(2), u.into());
        let odd = numerator.saturating_mul(2).saturating_add(1);
        let fractional = Expr::number(Div::div(Number::from(odd), Number::from(2)));
        let input = x.pow(fractional);
        let copies = odd.checked_div(4).expect("four is nonzero");
        let remainder = odd.checked_rem(4).expect("four is nonzero");
        let expected = if copies == 0 { input.clone() } else {
            Expr::product(vec![
                u.pow(copies),
                x.pow(Expr::number(Div::div(Number::from(remainder), Number::from(2)))),
            ])
        };
        prop_assert_eq!(substitution.apply_all(&input), expected);
        let zero_power = x.pow(0);
        prop_assert_eq!(substitution.apply_all(&zero_power), zero_power);
    }

    #[test]
    fn coefficient_quotients_allow_every_disjoint_positive_power_group(
        coefficient in 1_i32..=64,
        exponent in 3_i64..=9,
        x_value in -2_i64..=2,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(exponent)]);
        let source = Expr::product(vec![Expr::number(2), x.pow(2)]);
        let copies = exponent.checked_div(2).expect("two is nonzero");
        let denominator = Number::from(2).pow(&Number::from(copies));
        let quotient = Div::div(Number::from(coefficient), denominator);
        let mut factors = vec![Expr::number(quotient), u.pow(copies)];
        let power = exponent.saturating_sub(copies.saturating_mul(2));
        if power > 0 { factors.push(x.pow(power)); }
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::product(factors));
        let mut values = HashMap::from([(x, Expr::number(x_value))]);
        values.insert(u, source.substitute_symbols(&values));
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn simultaneous_factor_groups_consume_only_disjoint_original_units(exponent in 3_i64..=15) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let v = context.symb("v")?;
        let substitutions = Substitutions::new([
            (x.pow(3), u.into()), (x.pow(2), v.into()),
            (x.into(), y.into()), (u.into(), Expr::number(99)),
        ]).expect("distinct sources have no conflict");
        let copies = exponent.checked_div(3).expect("three is nonzero");
        let remainder = exponent.checked_rem(3).expect("three is nonzero");
        let mut factors = vec![u.pow(copies)];
        if remainder == 2 { factors.push(v.into()); }
        if remainder == 1 { factors.push(y.into()); }
        prop_assert_eq!(substitutions.apply_all(&x.pow(exponent)), Expr::product(factors));
    }

    #[test]
    fn virtual_exponent_literals_and_numeric_quotients_are_not_revisited(exponent in 5_i64..=14) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let v = context.symb("v")?;
        let substitutions = Substitutions::new([
            (Expr::product(vec![Expr::number(2), x.pow(3)]), u.into()), (Expr::number(3), v.into()),
            (u.into(), Expr::number(99)),
        ]).expect("the sources are distinct");
        let input = Expr::product(vec![Expr::number(6), x.pow(exponent)]);
        let copies = exponent.checked_div(3).expect("three is nonzero");
        let quotient = Div::div(Number::from(6), Number::from(2).pow(&Number::from(copies)));
        let mut factors = vec![Expr::number(quotient), u.pow(copies)];
        let power = exponent.saturating_sub(copies.saturating_mul(3));
        if power > 0 { factors.push(x.pow(power)); }
        prop_assert_eq!(substitutions.apply_all(&input), Expr::product(factors));
    }

    #[test]
    fn exact_scalar_quotients_preserve_rational_and_complex_backend_values(
        numerator in 1_i64..=9,
        denominator in 11_i64..=17,
        imaginary in -3_i64..=3,
        requested in 2_i64..=5,
        x_value in -2_i64..=2,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let rational = Div::div(Number::from(numerator), Number::from(denominator));
        let coefficient = Number::from(Complex::new(rational, imaginary).expect("both components are real scalars"));
        let input = Expr::product(vec![Expr::number(coefficient.clone()), x.into()]);
        let requested_rational = Div::div(Number::from(requested), Number::from(7));
        let requested_scalar = Number::from(Complex::new(requested_rational, imaginary.saturating_neg()).expect("both components are real scalars"));
        let source = Expr::product(vec![Expr::number(requested_scalar.clone()), x.into()]);
        let quotient = Div::div(coefficient, requested_scalar);
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::product(vec![Expr::number(quotient), u.into()]));
        let mut values = HashMap::from([(x, Expr::number(x_value))]);
        values.insert(u, source.substitute_symbols(&values));
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn floating_coefficients_use_backend_float_division(numerator in 1_i32..=99, requested in 2_i32..=9) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let coefficient = core::ops::Add::add(Mul::mul(f64::from(numerator), 0.25), 0.125);
        let source = Expr::product(vec![Expr::number(requested), x.into()]);
        let input = Expr::product(vec![Expr::number(coefficient), x.into()]);
        let quotient = Div::div(Number::from(coefficient), Number::from(requested));
        let expected = Expr::product(vec![Expr::number(quotient), u.into()]);
        prop_assert_eq!(Substitution::new(source, u.into()).apply_all(&input), expected);
    }

    #[test]
    fn identity_implicit_groups_preserve_canonical_nodes_and_prune_consumed_children(count in 2_i32..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(count), x.into()]), Expr::product(vec![Expr::number(count), y.into()])]);
        let source = Expr::sum(vec![x.into(), y.into()]);
        let substitutions = Substitutions::new([(source.clone(), source), (x.into(), Expr::number(99))])
            .expect("the sources are distinct");
        prop_assert_eq!(substitutions.apply_all(&input), input);
        let power = x.pow(count);
        prop_assert_eq!(Substitution::new(x.pow(2), x.pow(2)).apply_all(&power), power);
    }
}

#[test]
fn largest_integer_power_uses_a_bounded_replacement_graph() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let result = Substitution::new(x.pow(2), u.into()).apply_all(&x.pow(i64::MAX));
    assert_eq!(
        result,
        Expr::product(vec![
            u.pow(i64::MAX.checked_div(2).expect("two is nonzero")),
            x.into()
        ])
    );
    assert!(
        result.node_count() < 12,
        "large multiplicities stay compressed"
    );
}

#[test]
fn unrepresentable_integer_quotients_promote_without_overflowing() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let source = Expr::product(vec![Expr::number(-1), x.pow(2)]);
    let input = Expr::product(vec![Expr::number(i64::MIN), x.pow(3)]);
    let result = Substitution::new(source, context.symb("u").expect("symbol allocation").into())
        .apply_all(&input);
    let expected = Expr::product(vec![
        Expr::number(Number::from(i64::MIN).to_float().negate()),
        context.symb("u").expect("symbol allocation").into(),
        x.into(),
    ]);
    assert_eq!(result, expected, "MIN/-1 requires the approximate tier");
}
