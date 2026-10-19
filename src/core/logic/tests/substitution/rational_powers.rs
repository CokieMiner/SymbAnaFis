//! Principal rational power groups, retained poles, and simultaneous sources.

use core::ops::{Add, Div, Mul, Sub};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{
        Context, Expr, ExprView, Pattern, RewriteRule, Substitution, Substitutions, UserFunction,
    },
};

proptest! {
    #[test]
    fn root_sources_consume_integral_counts_without_extracting_new_branches(
        denominator in 2_u64..=9,
        copies in 1_i64..=12,
        reciprocal in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let exponent = if reciprocal { copies.saturating_neg() } else { copies };
        let source = x.pow(Number::from_rational(RationalType::from_parts(1, denominator)));
        let input = x.pow(Number::from_rational(RationalType::from_parts(exponent, denominator)));
        let result = Substitution::new(source, u.into()).apply_all(&input);
        prop_assert_eq!(result, u.pow(exponent));
    }

    #[test]
    fn proper_rational_groups_preserve_the_unmatched_fraction(
        denominator in 3_i64..=9,
        copies in 1_i64..=6,
        raw_remainder in 0_i64..=8,
        reciprocal in any::<bool>(),
        real in -3_i32..=3,
        imaginary in 1_i32..=3,
        lower_half_plane in any::<bool>(),
    ) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let numerator = denominator.saturating_sub(1);
        let remainder = raw_remainder.checked_rem(numerator).expect("the numerator is positive");
        let total = numerator.saturating_mul(copies).saturating_add(remainder);
        let signed_total = if reciprocal { total.saturating_neg() } else { total };
        let source = x.pow(Number::from_rational(RationalType::new(numerator, denominator)));
        let input = x.pow(Number::from_rational(RationalType::new(signed_total, denominator)));
        let mut factors = vec![u.pow(if reciprocal { copies.saturating_neg() } else { copies })];
        if remainder != 0 {
            let base = if input == x.to_expr() { x.to_expr() } else {
                Expr::product(vec![u.into(), x.pow(Number::from_rational(RationalType::new(1, denominator)))])
            };
            factors.push(base.pow(Number::from_rational(RationalType::new(
                if reciprocal { remainder.saturating_neg() } else { remainder },
                denominator,
            ))));
        }
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::product(factors));

        let imaginary_value = if lower_half_plane { imaginary.saturating_neg() } else { imaginary };
        let value = Expr::number(Number::from_complex(
            Number::from_float(f64::from(real)),
            Number::from_float(f64::from(imaginary_value)),
        ));
        let destination = source.substitute_symbol(x, &value);
        let values = HashMap::from([(x, value), (u, destination)]);
        let expected_value = input.substitute_symbols(&values);
        let actual_value = result.substitute_symbols(&values);
        let expected_number = expected_value.as_number().expect("finite approximate powers fold");
        let actual_number = actual_value.as_number().expect("finite approximate arithmetic folds");
        let error = Sub::sub(expected_number, actual_number).abs();
        let tolerance = Mul::mul(Number::from_float(1e-9), Add::add(Number::one(), expected_number.abs()));
        prop_assert!(error < tolerance, "the same principal base must retain its numerical value");
    }

    #[test]
    fn root_groups_keep_generated_exponent_counts_out_of_the_pass(copies in 2_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let substitutions = Substitutions::new([
            (x.sqrt(), u.into()),
            (u.into(), Expr::number(99)),
            (Expr::number(copies), Expr::number(77)),
        ]).expect("the sources are distinct");
        let input = x.pow(Number::from_rational(RationalType::new(copies, 2)));
        prop_assert_eq!(substitutions.apply_all(&input), u.pow(copies));
    }

    #[test]
    fn reciprocal_root_sources_do_not_introduce_poles(copies in 1_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let u = context.symb("u")?;
        let substitution = Substitution::new(x.pow(Number::from_rational(RationalType::new(-1, 2))), u.into());
        let positive = x.pow(Number::from_rational(RationalType::new(copies, 2)));
        prop_assert_eq!(substitution.apply_all(&positive), positive);
        let negative = x.pow(Number::from_rational(RationalType::new(copies.saturating_neg(), 2)));
        prop_assert_eq!(substitution.apply_all(&negative), u.pow(copies));
    }

    #[test]
    fn residual_composite_bases_visit_original_children_once(base_kind in 0_usize..5) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let z = context.symb("z")?;
        let u = context.symb("u")?;
        let base = match base_kind {
            0 => x.sin(),
            1 => Expr::sum(vec![x.into(), y.into()]),
            2 => Expr::sum(vec![x.pow(2), x.pow(3)]),
            3 => Expr::product(vec![x.into(), y.into()]),
            _ => Div::div(x.to_expr(), y),
        };
        let source = base.pow(Number::from_rational(RationalType::new(3, 4)));
        let substitutions = Substitutions::new([(source, u.into()), (x.into(), z.into())]).expect("distinct sources");
        let expected = Expr::product(vec![u.into(), base.substitute_symbol(x, &z.into()).pow(
            Number::from_rational(RationalType::new(1, 4)),
        )]);
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the name is unbound");
        let input = pair.call(vec![base.clone(), base]).expect("binary pair");
        prop_assert_eq!(substitutions.apply_all(&input), pair.call(vec![expected.clone(), expected]).expect("binary pair"));
    }
}

#[test]
fn fractional_outer_powers_do_not_expose_an_inner_base() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation");
    let substitution = Substitution::new(x.pow(2), u.into());
    assert_eq!(substitution.apply_all(&x.sqrt()), x.sqrt());
    assert_eq!(substitution.apply_all(&x.pow(2).sqrt()), u.sqrt());
    let fractional_source = x.pow(Number::from_rational(RationalType::new(3, 4)));
    assert_eq!(
        Substitution::new(fractional_source, u.into()).apply_all(&x.into()),
        Expr::product(vec![
            u.into(),
            x.pow(Number::from_rational(RationalType::new(1, 4)))
        ])
    );
    assert_eq!(
        Substitution::new(x.sqrt(), u.into()).apply_all(&x.into()),
        u.pow(2)
    );
    assert_eq!(
        Substitution::new(x.sqrt(), u.into())
            .apply_all(&x.pow(Number::from_rational(RationalType::new(3, 2)))),
        u.pow(3)
    );
    let rule = RewriteRule::new(Pattern::exact(x.sqrt()), Pattern::exact(u)).expect("literal rule");
    let input = x.pow(Number::from_rational(RationalType::new(3, 2)));
    assert_eq!(
        rule.apply_all(&input, None, None),
        Ok(u.pow(2).pow(Number::from_rational(RationalType::new(3, 2))))
    );
    let zero_denominator = Div::div(Expr::number(1), Expr::number(0));
    assert!(matches!(
        Substitution::new(x.sqrt(), u.into())
            .apply_all(&x.pow(Number::from_rational(RationalType::new(-1, 2))))
            .substitute_symbol(u, &Expr::number(0))
            .view(),
        ExprView::Div(..)
    ));
    assert!(!zero_denominator.is_zero_num());
}
