//! Signed integer groups, reciprocal forms, and zero-domain boundaries.

use core::ops::Div;

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{arbitrary::any, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, ExprView, Pattern, RewriteRule, Substitution, Substitutions},
};

proptest! {
    #[test]
    fn negative_powers_expose_all_positive_integer_groups(
        magnitude in 2_i64..=12,
        requested in 2_i64..=5,
        reciprocal in any::<bool>(),
        value in 1_i64..=3,
        negative_value in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let source = x.pow(requested);
        let input = if reciprocal {
            Div::div(Expr::number(1), x.pow(magnitude))
        } else { x.pow(magnitude.saturating_neg()) };
        let copies = magnitude.checked_div(requested).expect("positive requested power");
        let remainder = magnitude.checked_rem(requested).expect("positive requested power");
        let expected = if copies == 0 { input.clone() } else {
            let mut factors = vec![u.pow(copies.saturating_neg())];
            if remainder != 0 { factors.push(x.pow(remainder.saturating_neg())); }
            Expr::product(factors)
        };
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &expected);
        let rule = RewriteRule::new(Pattern::exact(source.clone()), Pattern::exact(u)).expect("literal rule");
        let rewritten = rule.apply_all(&input, None, None).expect("unlimited literal rule");
        let signed_value = if negative_value { value.saturating_neg() } else { value };
        let mut values = HashMap::from([(x, Expr::number(signed_value))]);
        values.insert(u, source.substitute_symbols(&values));
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
        prop_assert_eq!(rewritten.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn negative_sources_accept_reciprocal_groups_without_introducing_poles(
        requested in 2_i64..=5,
        magnitude in 2_i64..=12,
        reciprocal_source in any::<bool>(),
        reciprocal_input in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let source = if reciprocal_source {
            Div::div(Expr::number(1), x.pow(requested))
        } else { x.pow(requested.saturating_neg()) };
        let input = if reciprocal_input {
            Div::div(Expr::number(1), x.pow(magnitude))
        } else { x.pow(magnitude.saturating_neg()) };
        let substitution = Substitution::new(source, u.into());
        let copies = magnitude.checked_div(requested).expect("positive requested power");
        let remainder = magnitude.checked_rem(requested).expect("positive requested power");
        let expected = if copies == 0 { input.clone() } else {
            let mut factors = vec![u.pow(copies)];
            if remainder != 0 { factors.push(x.pow(remainder.saturating_neg())); }
            Expr::product(factors)
        };
        prop_assert_eq!(substitution.apply_all(&input), expected);
        let positive = x.pow(magnitude);
        prop_assert_eq!(substitution.apply_all(&positive), positive);
    }

    #[test]
    fn nested_integer_powers_match_without_expanding_the_dag(
        inner in 2_i64..=6,
        outer in 2_i64..=6,
        requested in 2_i64..=5,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let input = x.pow(inner).pow(outer);
        let total = inner.checked_mul(outer).expect("bounded integer powers");
        let copies = total.checked_div(requested).expect("positive requested power");
        let remainder = total.checked_rem(requested).expect("positive requested power");
        let expected = if copies == 0 { input.clone() } else {
            let mut factors = vec![u.pow(copies)];
            if remainder != 0 { factors.push(x.pow(remainder)); }
            Expr::product(factors)
        };
        prop_assert_eq!(Substitution::new(x.pow(requested), u.into()).apply_all(&input), expected);
    }

    #[test]
    fn signed_groups_follow_entry_order_without_revisiting_new_exponents(magnitude in 8_i64..=24) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?;
        let v = context.symb("v")?;
        let input = x.pow(magnitude.saturating_neg());
        let first = magnitude.checked_div(3).expect("three is nonzero");
        let residual = magnitude.checked_rem(3).expect("three is nonzero");
        let mut factors = vec![u.pow(first.saturating_neg())];
        if residual == 2 { factors.push(v.pow(-1)); }
        if residual == 1 { factors.push(y.pow(-1)); }
        let substitutions = Substitutions::new([
            (x.pow(3), u.into()), (x.pow(2), v.into()),
            (x.into(), y.into()), (u.into(), Expr::number(99)),
            (Expr::number(-1), Expr::number(42)),
        ]).expect("distinct sources");
        prop_assert_eq!(substitutions.apply_all(&input), Expr::product(factors));
    }

    #[test]
    fn generated_numeric_powers_promote_before_native_capacity(repetitions in 60_i32..=66) {
        let context = Context::new();
        let x = context.symb("x")?;
        let native_exponent = u32::try_from(repetitions).expect("the generated count is positive");
        let exact = 2_i64.checked_pow(native_exponent);
        let positive_expected = exact.map_or_else(
            || Expr::number(2.0).pow(repetitions), Expr::number,
        );
        let positive = Substitution::new(x.pow(-1), Expr::number(2))
            .apply_all(&x.pow(repetitions.saturating_neg()));
        prop_assert_eq!(positive, positive_expected);
        let negative_expected = 1_u64.checked_shl(native_exponent).map_or_else(
            || Expr::number(2.0).pow(repetitions.saturating_neg()),
            |denominator| Expr::number(Number::from_rational(RationalType::from_parts(1, denominator))),
        );
        let negative = Substitution::new(x.pow(2), Expr::number(2))
            .apply_all(&x.pow(repetitions.saturating_mul(-2)));
        prop_assert_eq!(negative, negative_expected);
    }
}

#[test]
fn replacing_an_even_power_preserves_the_sign_of_an_odd_power() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let input = x.pow(5);
    let expected = Expr::product(vec![u.pow(2), x.into()]);
    let result = Substitution::new(x.pow(2), u.into()).apply_all(&input);
    assert_eq!(result, expected);
    let rule = RewriteRule::new(Pattern::exact(x.pow(2)), Pattern::exact(u))
        .expect("the literal replacement has no captures");
    assert_eq!(rule.apply_all(&input, None, None), Ok(result.clone()));
    let values = HashMap::from([(x, Expr::number(-1)), (u, Expr::number(1))]);
    assert_eq!(result.substitute_symbols(&values), Expr::number(-1));
}

#[test]
fn nested_reciprocals_keep_their_zero_exclusion() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let input = x.pow(-2).pow(-3);
    let result = Substitution::new(x.pow(2), u.into()).apply_all(&input);
    assert_eq!(
        result,
        u.pow(-1).pow(-3),
        "the nested pole remains explicit"
    );
    assert!(
        !result.contains_symbol(x),
        "the original base was eliminated"
    );
    assert!(
        matches!(
            result.substitute_symbol(u, &Expr::number(0)).view(),
            ExprView::Div(..)
        ),
        "zero remains undefined"
    );
    let fractional = x.pow(2).pow(Expr::number(0.5));
    assert_eq!(
        Substitution::new(x.pow(2), u.into()).apply_all(&fractional),
        u.pow(Expr::number(0.5)),
        "the principal square root is not replaced by the old base"
    );
}

#[test]
fn minimum_native_exponent_and_wide_nested_counts_do_not_wrap() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let inverse = Substitution::new(x.pow(-1), u.into()).apply_all(&x.pow(i64::MIN));
    assert_eq!(
        inverse,
        u.pow(2).pow(1_i64 << 62),
        "the positive magnitude exceeds i64 without wrapping"
    );
    let input = x.pow(i64::MAX).pow(i64::MAX);
    let result = Substitution::new(x.pow(2), u.into()).apply_all(&input);
    let mut nodes = 0_usize;
    result
        .map(|_| {
            nodes = nodes.checked_add(1).expect("the projected graph is small");
            Ok::<_, ()>(None)
        })
        .expect("counting cannot fail");
    assert!(nodes < 160, "wide powers use logarithmic storage");
    assert!(
        result.contains_symbol(u),
        "the wide exponent exposes the replacement"
    );
}
