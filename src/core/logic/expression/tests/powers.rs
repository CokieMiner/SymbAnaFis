//! Canonical roots, builtin aliases, and explicit denominator domains.

use core::ops::{Div, Mul, Neg};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::core::{Context, Expr, ExprView, Function, Pattern, RewriteRule};

proptest! {
    #[test]
    fn exact_square_roots_share_their_power_node(base_kind in 0_usize..7, value in -20_i64..=20) {
        let context = Context::new();
        let x = context.symb("x")?;
        let base = match base_kind {
            0 => x.to_expr(),
            1 => x.sin(),
            2 => Expr::sum(vec![x.into(), Expr::number(value)]),
            3 => Expr::e(),
            4 => Expr::pi(),
            5 => Expr::i(),
            _ => Expr::number(value),
        };
        let half = Number::from_rational(RationalType::new(1, 2));
        prop_assert_eq!(base.sqrt(), base.pow(half));
        let input = Number::from_float(f64::from(i32::try_from(value).expect("the generated value fits i32")));
        let literal = Expr::number(input);
        prop_assert_eq!(literal.sqrt(), literal.pow(Number::from_rational(RationalType::new(1, 2))));
    }

    #[test]
    fn known_builtin_bases_and_spellings_share_construction(offset in -12_i64..=12) {
        let context = Context::new();
        let argument = Expr::sum(vec![context.symb("x")?.into(), Expr::number(offset)]);
        prop_assert_eq!(Expr::e().pow(&argument), argument.exp());
        prop_assert_eq!(argument.exp_neg(), Neg::neg(&argument).exp());
        prop_assert_eq!(argument.log(2), argument.log2());
        prop_assert_eq!(argument.log(10), argument.log10());
        prop_assert_eq!(argument.log(Expr::e()), argument.ln());
        let a = Pattern::capture("a", None);
        let half = Number::from_rational(RationalType::new(1, 2));
        let rule = RewriteRule::new(a.pow(Expr::number(half)), a.exp()).expect("the replacement capture is bound");
        prop_assert_eq!(rule.apply_all(&argument.sqrt(), None, None), Ok(argument.exp()));
        let inverse = RewriteRule::new(Pattern::exact(Expr::e()).pow(&a), a.sqrt()).expect("the replacement capture is bound");
        prop_assert_eq!(inverse.apply_all(&argument.exp(), None, None), Ok(argument.sqrt()));
    }

    #[test]
    fn negative_exact_exponents_use_positive_denominators(numerator in 1_i64..=20, denominator in 1_u64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let positive = Number::from_rational(RationalType::from_parts(numerator, denominator));
        let negative = Number::from_rational(RationalType::from_parts(numerator.saturating_neg(), denominator));
        let result = x.pow(negative);
        prop_assert_eq!(&result, &Div::div(Expr::number(1), x.pow(positive)));
        let explicit = matches!(result.view(), ExprView::Div(unit, _) if unit.is_one_num());
        prop_assert!(explicit);
        let negative_power = result.fold(false, |found, node| found || matches!(node.view(),
            ExprView::Pow(_, exponent) if exponent.as_number().is_some_and(Number::is_negative)));
        prop_assert!(!negative_power);
    }

    #[test]
    fn products_and_integer_powers_retain_all_denominators(power in 2_i64..=8) {
        let context = Context::new();
        let x = context.symb("x")?.to_expr();
        let y = context.symb("y")?.to_expr();
        let z = context.symb("z")?.to_expr();
        prop_assert_eq!(Mul::mul(&x, y.pow(-1)), Div::div(&x, &y));
        prop_assert_eq!(Div::div(Div::div(&x, &y), &z), Div::div(&x, Mul::mul(&y, &z)));
        prop_assert_eq!(Mul::mul(&x, &y).pow(power), Mul::mul(x.pow(power), y.pow(power)));
        prop_assert_eq!(Div::div(&x, &y).pow(power), Div::div(x.pow(power), y.pow(power)));
        prop_assert_eq!(x.sqrt().pow(power), x.pow(Number::from_rational(RationalType::from_parts(power, 2))));
    }

    #[test]
    fn nested_positive_integer_powers_share_a_single_canonical_node(
        inner in 2_i64..=12,
        outer in 2_i64..=12,
        kind in 0_usize..6,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let base = match kind {
            0 => x.to_expr(),
            1 => x.sin(),
            2 => Expr::sum(vec![x.into(), y.into()]),
            3 => Expr::sum(vec![x.pow(2), x.pow(3)]),
            4 => Div::div(Expr::number(1), x),
            _ => Expr::number(0).pow(0),
        };
        let combined = inner.saturating_mul(outer);
        prop_assert_eq!(base.pow(inner).pow(outer), base.pow(combined));
        let a = Pattern::capture("a", None);
        let rule = RewriteRule::new(a.pow(inner).pow(outer), a.sin())?;
        prop_assert_eq!(rule.apply_all(&x.pow(combined), None, None), Ok(x.sin()));
    }

    #[test]
    fn nested_positive_powers_preserve_an_independent_integer_oracle(
        inner in 2_u32..=4,
        outer in 2_u32..=4,
        value in -2_i64..=2,
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let combined = inner.saturating_mul(outer);
        let expression = x.pow(i64::from(inner)).pow(i64::from(outer));
        let expected = value.checked_pow(combined).expect("the magnitude is at most two to the sixteenth power");
        prop_assert_eq!(expression.substitute_symbol(x, &Expr::number(value)), Expr::number(expected));
    }

    #[test]
    fn minimum_signed_exponents_use_positive_blocks(denominator in 1_u64..=19) {
        let context = Context::new();
        let x = context.symb("x")?;
        let result = x.pow(Number::from_rational(RationalType::from_parts(i64::MIN, denominator)));
        let valid = result.fold(true, |valid, expression| valid && !matches!(expression.view(),
            ExprView::Pow(_, exponent) if exponent.as_number().is_some_and(Number::is_negative)));
        prop_assert!(valid);
        prop_assert!(result.node_count() < 20);
    }

    #[test]
    fn literal_aliases_share_exact_and_approximate_folding(value in 1_i32..=20) {
        let literal = Expr::number(value);
        prop_assert_eq!(literal.log(2), literal.log2());
        prop_assert_eq!(literal.log(10), literal.log10());
        let approximate = Expr::number(f64::from(value));
        prop_assert_eq!(Expr::e().pow(&approximate), approximate.exp());
        prop_assert_eq!(approximate.log(2), approximate.log2());
    }

    #[test]
    fn collected_roots_reenter_grouping_only_when_their_base_changes(power in 1_i64..=9, coefficient in -9_i64..=9) {
        let context = Context::new();
        let x = context.symb("x")?;
        let product = Expr::product(vec![Expr::number(coefficient), x.pow(power), x.sqrt(), x.sqrt()]);
        prop_assert_eq!(product, Expr::product(vec![Expr::number(coefficient), x.pow(power.saturating_add(1))]));
        let nonsquare_root = Expr::number(2).sqrt();
        prop_assert_eq!(Expr::product(vec![Expr::number(coefficient), nonsquare_root.clone(), nonsquare_root]), Expr::number(coefficient.saturating_mul(2)));
    }

    #[test]
    fn nested_denominators_preserve_values_under_product_and_integer_powers(depth in 1_usize..=64, power in 2_i64..=6) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let mut left = x.to_expr();
        let mut right = y.to_expr();
        for _ in 0..depth {
            left = Div::div(Expr::number(1), left);
            right = Div::div(Expr::number(1), right);
        }
        let values = crate::collections::HashMap::from([(x, Expr::number(2)), (y, Expr::number(3))]);
        let product = Mul::mul(&left, &right);
        let evaluated = Mul::mul(left.substitute_symbols(&values), right.substitute_symbols(&values));
        prop_assert_eq!(product.substitute_symbols(&values), evaluated);
        prop_assert_eq!(left.pow(power).substitute_symbols(&values), left.substitute_symbols(&values).pow(power));
        prop_assert!(product.max_depth() <= depth.saturating_add(3));
    }
}

#[test]
fn deeply_nested_quotients_normalize_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(|| {
            let context = Context::new();
            let x = context.symb("x").expect("symbol allocation");
            let y = context.symb("y").expect("symbol allocation");
            let mut left = x.to_expr();
            let mut right = y.to_expr();
            for _ in 0..4096 {
                left = Div::div(Expr::number(1), left);
                right = Div::div(Expr::number(1), right);
            }
            let product = Mul::mul(&left, &right);
            let powered = left.pow(2);
            assert!(product.contains_symbol(x));
            assert!(product.contains_symbol(y));
            assert!(powered.contains_symbol(x));
            assert!(product.node_count() < 4110);
            assert!(powered.node_count() < 4110);
        })
        .expect("the small-stack thread starts")
        .join()
        .expect("normalization and destruction remain iterative");
}

#[test]
fn root_branches_approximate_exponents_and_unknown_signs_remain_distinct() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let n = context.symb("n").expect("symbol allocation");
    assert_ne!(x.sqrt(), x.pow(0.5));
    assert_ne!(
        Expr::number(-8).cbrt(),
        Expr::number(-8).pow(Number::from_rational(RationalType::new(1, 3)))
    );
    assert_eq!(Expr::i().pow(2).sqrt(), Expr::i());
    assert_ne!(Expr::i().pow(2).sqrt(), Expr::i().abs());
    assert!(matches!(x.pow(n).view(), ExprView::Pow(..)));
    assert_eq!(Expr::number(1).exp(), Expr::e());
    assert_eq!(Expr::e().ln(), Expr::number(1));
    assert_ne!(Expr::number(1.0).exp(), Expr::e());
    assert!(matches!(
        Div::div(Expr::number(1), x.pow(-1)).view(),
        ExprView::Div(..)
    ));
    assert_eq!(
        Function::builtin("sqrt")
            .expect("known builtin")
            .call(vec![x.into()])
            .expect("unary call"),
        x.sqrt()
    );
}

#[test]
fn nested_power_capacity_zero_exclusions_and_fractional_branches_stay_explicit() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let maximum = x.pow(i64::MAX);
    let overflow = maximum.pow(2);
    assert!(matches!(overflow.view(), ExprView::Pow(inner, exponent)
        if inner == &maximum && exponent == &Expr::number(2)));
    let zero_power = x.pow(0);
    assert!(
        matches!(zero_power.pow(2).view(), ExprView::Pow(inner, exponent)
        if inner == &zero_power && exponent == &Expr::number(2))
    );
    let cube = x.pow(3);
    let third = Number::from_rational(RationalType::new(1, 3));
    assert!(
        matches!(cube.pow(third.clone()).view(), ExprView::Pow(inner, _)
        if inner == &cube)
    );
    let fractional = x.pow(third);
    assert!(matches!(fractional.pow(3).view(), ExprView::Pow(inner, _)
        if inner == &fractional));
    let approximate = x.pow(2.0);
    assert!(matches!(approximate.pow(3).view(), ExprView::Pow(inner, _)
        if inner == &approximate));
    let pole = Div::div(Expr::number(1), x);
    let powered_pole = pole.pow(2).pow(3);
    assert_eq!(powered_pole, pole.pow(6));
    assert!(matches!(
        powered_pole.substitute_symbol(x, &Expr::number(0)).view(),
        ExprView::Div(..)
    ));
}
