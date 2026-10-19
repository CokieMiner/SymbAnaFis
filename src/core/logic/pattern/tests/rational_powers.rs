//! Counted captured rational powers with unchanged principal bases.

use core::ops::{Add, Div, Mul, Sub};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, Pattern, RewriteRule, UserFunction};

use super::super::{AlgebraicOperands, Group, GroupValues, Selected};

proptest! {
    #[test]
    fn generic_root_rules_consume_explicit_power_groups(copies in 2_i64..=16) {
        let context = Context::new();
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.sqrt(), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(copies, 2)));
        let expected = if copies == 2 { input.clone() } else { output.call(vec![x.into()]).expect("unary call").pow(copies) };
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn fractional_templates_keep_rational_remainders(copies in 3_i64..=12, remainder in 0_i64..=1, inverse in proptest::bool::ANY) {
        let context = Context::new();
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let numerator = if inverse { -2 } else { 2 };
        let rule = RewriteRule::new(capture.pow(Expr::number(Number::from_rational(RationalType::new(numerator, 3)))), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let total = copies.saturating_mul(2).saturating_add(remainder);
        let input = x.pow(Number::from_rational(RationalType::new(if inverse { total.saturating_neg() } else { total }, 3)));
        let replacement = output.call(vec![x.into()]).expect("unary call").pow(copies);
        let residual = x.pow(Number::from_rational(RationalType::new(remainder, 3)));
        let expected = if remainder == 0 { replacement } else if inverse { Div::div(replacement, residual) } else { Mul::mul(replacement, residual) };
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn integral_templates_consume_groups_inside_fractional_powers(exponent in 3_i64..=17) {
        let context = Context::new();
        let x = context.symb("x")?;
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.pow(2), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(exponent, 2)));
        let copies = exponent.checked_div(4).expect("constant divisor");
        let remainder = exponent.checked_rem(4).expect("constant divisor");
        let mut factors = vec![];
        if copies > 0 { factors.push(output.call(vec![x.into()]).expect("unary call").pow(copies)); }
        match remainder {
            0 => {},
            1 => factors.push(x.sqrt()),
            2 => factors.push(x.into()),
            _ => {
                factors.push(output.call(vec![x.sqrt()]).expect("unary call"));
                factors.push(x.sqrt());
            }
        }
        let expected = Expr::product(factors);
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(expected));
    }

    #[test]
    fn generic_rational_identity_preserves_both_complex_half_planes(copies in 3_i64..=9, remainder in 0_i64..=1, real in -3_i32..=3, imaginary in 1_i32..=3, lower in proptest::bool::ANY) {
        let context = Context::new();
        let x = context.symb("x")?;
        let capture = Pattern::capture("a", None);
        let pattern = capture.pow(Expr::number(Number::from_rational(RationalType::new(2, 3))));
        let rule = RewriteRule::new(pattern.clone(), pattern).expect("bound capture");
        let input = x.pow(Number::from_rational(RationalType::new(copies.saturating_mul(2).saturating_add(remainder), 3)));
        let rewritten = rule.apply_all(&input, None, None).expect("unbounded search");
        let value = Expr::number(Number::from_complex(Number::from_float(f64::from(real)), Number::from_float(f64::from(if lower { imaginary.saturating_neg() } else { imaginary }))));
        let before = input.substitute_symbol(x, &value);
        let after = rewritten.substitute_symbol(x, &value);
        let expected = before.as_number().expect("finite approximate powers fold");
        let actual = after.as_number().expect("finite approximate powers fold");
        let tolerance = Mul::mul(Number::from_float(1e-9), Add::add(Number::one(), expected.abs()));
        prop_assert!(Sub::sub(expected, actual).abs() < tolerance);
    }
}

#[test]
fn nested_reciprocal_templates_do_not_introduce_their_zero_exclusion() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let capture = Pattern::capture("a", None);
    let pattern = Div::div(Pattern::exact(1), Div::div(Pattern::exact(1), capture));
    let rule = RewriteRule::new(pattern, Pattern::exact(u)).expect("literal replacement");
    let input = x.pow(2);
    assert_eq!(rule.apply_all(&input, None, None), Ok(input));
}

#[test]
fn the_last_native_power_candidate_advances_instead_of_repeating() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let expression = Div::div(Expr::number(1), x.pow(i64::MAX));
    let operands =
        AlgebraicOperands::new(&expression, false, 0).expect("multiplicative projection");
    let pattern = Div::div(Pattern::exact(1), Pattern::capture("a", None));
    let patterns = [pattern];
    let mut group = Group {
        patterns: &patterns,
        values: GroupValues::Counted(&operands),
        selected: Selected::Counted(vec![]),
        next: 1,
        projection: i64::MAX,
        combination: None,
        sum: false,
        start: 0,
        root: true,
    };
    assert!(group.candidate(&patterns[0]).is_none());
}

#[test]
fn normalized_roots_supply_exact_rational_groups_without_peeling_fractional_bases() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        capture.pow(Expr::number(Number::from_rational(RationalType::new(1, 3)))),
        Pattern::exact(u),
    )
    .expect("literal replacement");
    assert_eq!(
        rule.apply_all(&x.sqrt(), None, None),
        Ok(Mul::mul(
            u,
            x.pow(Number::from_rational(RationalType::new(1, 6)))
        ))
    );
    let branch =
        RewriteRule::new(capture.pow(2).sqrt(), Pattern::exact(u)).expect("literal replacement");
    let input = x.pow(Number::from_rational(RationalType::new(3, 2)));
    assert_eq!(branch.apply_all(&input, None, None), Ok(input));
}

#[test]
fn one_candidate_handles_the_largest_rational_multiplicity() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let output = context
        .register_function("output", UserFunction::new(1..=1))
        .expect("unbound function");
    let capture = Pattern::capture("a", None);
    let rule = RewriteRule::new(
        capture.pow(Expr::number(Number::from_rational(RationalType::new(1, 3)))),
        Pattern::apply(output.clone(), vec![capture]).expect("unary template"),
    )
    .expect("bound capture");
    let input = x.pow(Number::from_rational(RationalType::new(i64::MAX, 3)));
    let expected = output
        .call(vec![x.into()])
        .expect("unary call")
        .pow(i64::MAX);
    assert_eq!(rule.apply_all(&input, Some(1), None), Ok(expected));
}
