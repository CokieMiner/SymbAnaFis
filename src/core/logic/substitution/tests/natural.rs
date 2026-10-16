//! Scalar multiples, quotient factors, and bounded coefficient reconstruction.

use core::{convert::Infallible, ops::Div};

use alloc::vec;

use num_anafis::{Number, RationalType};
use proptest::{arbitrary::any, prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, ExprView, Pattern, RewriteRule},
};

use super::super::{Substitution, Substitutions};

proptest! {
    #[test]
    fn implicit_and_explicit_groups_follow_the_same_entry_order(
        sum in any::<bool>(), reverse in any::<bool>(), count in 2_i32..=9,
    ) {
        let ctx=Context::new(); let x=ctx.symb("x"); let y=ctx.symb("y"); let z=ctx.symb("z"); let u=ctx.symb("u"); let v=ctx.symb("v");
        let (first,second,input,forward,backward)=if sum {
            let scale=Expr::number(Div::div(Number::from(count.saturating_sub(1)),Number::from(count)));
            let remainder=Expr::number(Div::div(Number::from(1),Number::from(count)));
            let weighted=Expr::product(vec![Expr::number(count),x.into()]);
            (Expr::sum(vec![x.into(),y.into()]),Expr::sum(vec![weighted.clone(),z.into()]),Expr::sum(vec![weighted,y.into(),z.into()]),Expr::sum(vec![u.into(),Expr::product(vec![scale,v.into()]),Expr::product(vec![remainder,z.into()])]),Expr::sum(vec![v.into(),y.into()]))
        } else {
            (Expr::product(vec![x.pow(count.saturating_sub(1)),y.into()]),Expr::product(vec![x.pow(count),z.into()]),Expr::product(vec![x.pow(count),y.into(),z.into()]),Expr::product(vec![u.into(),x.into(),z.into()]),Expr::product(vec![v.into(),y.into()]))
        };
        let entries=if reverse {[(second,v.into()),(first,u.into())]} else {[(first,u.into()),(second,v.into())]};
        let expected=if reverse {backward} else {forward};
        prop_assert_eq!(Substitutions::new(entries).expect("distinct sources").apply_all(&input),expected);
    }

    #[test]
    fn signed_rational_sum_groups_preserve_the_unmatched_coefficients(
        left in 1_i32..=9, right in 1_i32..=9, denominator in 1_i32..=7,
        negative in any::<bool>(), x_value in -3_i64..=3, y_value in -3_i64..=3,
    ) {
        let ctx = Context::new();
        let x = ctx.symb("x"); let y = ctx.symb("y"); let z = ctx.symb("z"); let u = ctx.symb("u");
        let sign = if negative {-1_i64} else {1};
        let left_value = Div::div(Number::from(i64::from(left).saturating_mul(sign)), Number::from(denominator));
        let right_value = Div::div(Number::from(i64::from(right).saturating_mul(sign)), Number::from(denominator));
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(left_value), x.into()]), Expr::product(vec![Expr::number(right_value), y.into()]), z.into()]);
        let common = left.min(right);
        let scale = Div::div(Number::from(i64::from(common).saturating_mul(sign)), Number::from(denominator));
        let mut expected = vec![Expr::product(vec![Expr::number(scale), u.into()]), z.into()];
        if left > common { expected.push(Expr::product(vec![Expr::number(Div::div(Number::from(i64::from(left.saturating_sub(common)).saturating_mul(sign)), Number::from(denominator))), x.into()])); }
        if right > common { expected.push(Expr::product(vec![Expr::number(Div::div(Number::from(i64::from(right.saturating_sub(common)).saturating_mul(sign)), Number::from(denominator))), y.into()])); }
        let source = Expr::sum(vec![x.into(), y.into()]);
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::sum(expected));
        let values = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value)), (z, Expr::number(2)), (u, Expr::number(x_value.saturating_add(y_value)))]);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
        let rule = RewriteRule::new(Pattern::exact(source), Pattern::exact(u)).expect("no captures");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(result.clone()));
        let group_rule=RewriteRule::new(Pattern::sum(vec![Pattern::exact(x),Pattern::exact(y)]),Pattern::exact(u)).expect("literal pattern");
        prop_assert_eq!(group_rule.apply_all(&input,None,None),Ok(result));
    }

    #[test]
    fn scaled_group_sources_match_expanded_and_unscaled_addition(coefficient in 2_i64..=9, divide in any::<bool>()) {
        let ctx=Context::new(); let x=ctx.symb("x"); let y=ctx.symb("y"); let u=ctx.symb("u");
        let base=Expr::sum(vec![x.into(),y.into()]);
        let source=if divide {Div::div(&base,Expr::number(coefficient))} else {Expr::product(vec![Expr::number(coefficient),base.clone()])};
        let scale=if divide {Number::from(coefficient)} else {Div::div(Number::from(1),Number::from(coefficient))};
        let expected=Expr::product(vec![Expr::number(scale),u.into()]);
        prop_assert_eq!(&Substitution::new(source.clone(),u.into()).apply_all(&base),&expected);
        let rule=RewriteRule::new(Pattern::exact(source),Pattern::exact(u)).expect("literal pattern");
        prop_assert_eq!(rule.apply_all(&base,None,None),Ok(expected));
    }

    #[test]
    fn partially_consumed_constants_are_not_new_substitution_sources(extra in 1_i32..=9) {
        let ctx=Context::new(); let x=ctx.symb("x"); let u=ctx.symb("u"); let v=ctx.symb("v");
        let source=Expr::sum(vec![x.into(),Expr::number(1)]);
        let input=Expr::sum(vec![Expr::product(vec![Expr::number(2),x.into()]),Expr::number(extra.saturating_add(2))]);
        let substitutions=Substitutions::new([(source,u.into()),(Expr::number(extra),v.into())]).expect("distinct sources");
        prop_assert_eq!(substitutions.apply_all(&input),Expr::sum(vec![Expr::product(vec![Expr::number(2),u.into()]),Expr::number(extra)]));
    }

    #[test]
    fn denominator_quotients_are_not_matched_again_in_the_same_pass(divisor in 2_i64..=9) {
        let ctx=Context::new(); let x=ctx.symb("x"); let u=ctx.symb("u"); let v=ctx.symb("v");
        let source=Div::div(x.to_expr(),Expr::number(divisor));
        let quotient=Div::div(Expr::number(1),Expr::number(divisor));
        let substitutions=Substitutions::new([(source,u.into()),(quotient,v.into())]).expect("distinct sources");
        prop_assert_eq!(substitutions.apply_all(&x.to_expr()),Expr::product(vec![Expr::number(divisor),u.into()]));
    }

    #[test]
    fn absent_numeric_prefixes_supply_unit_coefficients(exponent in 1_i64..=15, x_value in -2_i64..=2) {
        let ctx = Context::new(); let x = ctx.symb("x"); let u = ctx.symb("u");
        let source = Expr::product(vec![Expr::number(2), x.into()]);
        let input = x.pow(exponent);
        let coefficient = Div::div(Number::from(1), Number::from(2).pow(&Number::from(exponent)));
        let result = Substitution::new(source, u.into()).apply_all(&input);
        prop_assert_eq!(&result, &Expr::product(vec![Expr::number(coefficient), u.pow(exponent)]));
        let values = HashMap::from([(x, Expr::number(x_value)), (u, Expr::number(x_value.saturating_mul(2)))]);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn imaginary_unit_factors_use_their_exact_period(exponent in 1_i64..=i64::MAX) {
        let ctx=Context::new(); let x=ctx.symb("x"); let u=ctx.symb("u");
        let source=Expr::product(vec![Expr::i(),x.into()]);
        let expected=Expr::product(vec![Expr::i().pow(exponent.saturating_neg()),u.pow(exponent)]);
        prop_assert_eq!(Substitution::new(source,u.into()).apply_all(&x.pow(exponent)),expected);
    }

    #[test]
    fn even_sign_cycles_keep_the_minimum_integer_exact(half_exponent in 1_i64..=100) {
        let ctx=Context::new(); let x=ctx.symb("x"); let u=ctx.symb("u");
        let exponent=half_exponent.saturating_mul(2);
        let source=Expr::product(vec![Expr::number(-1),x.into()]);
        let input=Expr::product(vec![Expr::number(i64::MIN),x.pow(exponent)]);
        let expected=Expr::product(vec![Expr::number(i64::MIN),u.pow(exponent)]);
        prop_assert_eq!(Substitution::new(source,u.into()).apply_all(&input),expected);
    }

    #[test]
    fn quotient_factors_retain_every_unmatched_denominator(
        numerator_power in 1_i64..=8, denominator_power in 1_i64..=8,
        x_value in -2_i64..=2, y_value in 1_i64..=3,
    ) {
        let ctx = Context::new(); let x = ctx.symb("x"); let y = ctx.symb("y"); let z = ctx.symb("z"); let u = ctx.symb("u");
        let source = Div::div(x.to_expr(), y.to_expr());
        let input = Div::div(Expr::product(vec![x.pow(numerator_power), z.into()]), y.pow(denominator_power));
        let common = numerator_power.min(denominator_power);
        let mut numerator = vec![u.pow(common), z.into()];
        if numerator_power > common { numerator.push(x.pow(numerator_power.saturating_sub(common))); }
        let mut expected = Expr::product(numerator);
        if denominator_power > common { expected = Div::div(expected, y.pow(denominator_power.saturating_sub(common))); }
        let result = Substitution::new(source.clone(), u.into()).apply_all(&input);
        prop_assert_eq!(&result, &expected);
        let mut values = HashMap::from([(x, Expr::number(x_value)), (y, Expr::number(y_value)), (z, Expr::number(3))]);
        values.insert(u, source.substitute_symbols(&values));
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn polynomial_groups_accept_fractional_and_negative_scales(
        lower in 1_i64..=4, higher in 6_i64..=9, numerator in -7_i64..=-1, denominator in 2_i64..=7,
    ) {
        let ctx = Context::new(); let x = ctx.symb("x"); let u = ctx.symb("u");
        let coefficient = Expr::number(Div::div(Number::from(numerator), Number::from(denominator)));
        let source = Expr::sum(vec![x.pow(lower), x.pow(higher)]);
        let input = Expr::sum(vec![Expr::product(vec![coefficient.clone(), x.pow(lower)]), Expr::product(vec![coefficient.clone(), x.pow(higher)]), Expr::number(5)]);
        prop_assert_eq!(Substitution::new(source, u.into()).apply_all(&input), Expr::sum(vec![Expr::product(vec![coefficient, u.into()]), Expr::number(5)]));
    }

    #[test]
    fn exact_coefficients_reduce_before_crossing_integer_power_capacity(numerator_power in 0_u32..=10, denominator_power in 1_u32..=63) {
        let ctx = Context::new(); let x = ctx.symb("x"); let u = ctx.symb("u");
        let groups = i64::from(numerator_power.saturating_add(denominator_power));
        let coefficient = 1_i64.checked_shl(numerator_power).expect("at most ten bits");
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(groups.saturating_mul(2))]);
        let expected_coefficient = Number::from_rational(RationalType::from_parts(1, 1_u64.checked_shl(denominator_power).expect("at most 63 bits")));
        let result = Substitution::new(Expr::product(vec![Expr::number(2), x.pow(2)]), u.into()).apply_all(&input);
        prop_assert_eq!(result, Expr::product(vec![Expr::number(expected_coefficient), u.pow(groups)]));
    }

    #[test]
    fn repeated_replacement_coefficients_promote_before_integer_power_overflow(groups in 60_i64..=66, literal in any::<bool>()) {
        let ctx=Context::new(); let x=ctx.symb("x"); let u=ctx.symb("u");
        let replacement=if literal {Expr::number(2)} else {Expr::product(vec![Expr::number(2),u.into()])};
        let coefficient=if groups<=62 {Number::from(2).pow(&Number::from(groups))} else {Number::from(2).to_float().pow(&Number::from(groups))};
        let expected=if literal {Expr::number(coefficient)} else {Expr::product(vec![Expr::number(coefficient),u.pow(groups)])};
        let result=Substitution::new(x.pow(2),replacement).apply_all(&x.pow(groups.saturating_mul(2)));
        prop_assert_eq!(result,expected);
    }

    #[test]
    fn approximate_replacements_use_the_backends_direct_power(coefficient in 1_i32..=31, groups in 2_i64..=12) {
        let ctx=Context::new(); let x=ctx.symb("x");
        let value=core::ops::Mul::mul(f64::from(coefficient),0.125);
        let result=Substitution::new(x.pow(2),Expr::number(value)).apply_all(&x.pow(groups.saturating_mul(2)));
        prop_assert_eq!(result,Expr::number(Number::from(value).pow(&Number::from(groups))));
    }

    #[test]
    fn scalar_remainders_keep_simultaneous_source_identity(denominator in 2_i64..=9) {
        let ctx = Context::new(); let x = ctx.symb("x"); let y = ctx.symb("y"); let u = ctx.symb("u"); let v = ctx.symb("v");
        let scale = Expr::number(Div::div(Number::from(1), Number::from(denominator)));
        let left = Expr::number(Div::div(Number::from(2), Number::from(denominator)));
        let input = Expr::sum(vec![Expr::product(vec![left, x.into()]), Expr::product(vec![scale.clone(), y.into()])]);
        let substitutions = Substitutions::new([(Expr::sum(vec![x.into(),y.into()]), u.into()), (x.into(),v.into()), (u.into(), Expr::number(99)), (scale.clone(), Expr::number(777))]).expect("distinct sources");
        prop_assert_eq!(substitutions.apply_all(&input), Expr::sum(vec![Expr::product(vec![scale.clone(),u.into()]),Expr::product(vec![scale,v.into()])]));
    }

    #[test]
    fn nested_denominator_domains_remain_explicit(exponent in 2_i64..=8) {
        let ctx = Context::new(); let x = ctx.symb("x"); let y = ctx.symb("y"); let z = ctx.symb("z"); let u = ctx.symb("u");
        let input = Div::div(x.pow(exponent), Div::div(y.to_expr(),z.to_expr()));
        prop_assert!(Substitution::new(Div::div(x.to_expr(),y.to_expr()), u.into()).apply_all(&input).same_node(&input));
    }
}

#[test]
fn maximum_factor_counts_preserve_nonzero_scalars_without_expanding_them() {
    let ctx = Context::new();
    let x = ctx.symb("x");
    let u = ctx.symb("u");
    let result = Substitution::new(Expr::product(vec![Expr::number(2), x.pow(2)]), u.into())
        .apply_all(&Expr::product(vec![Expr::number(8), x.pow(i64::MAX)]));
    let checked = result
        .map(|node| {
            if let ExprView::Div(_, denominator) = node.view() {
                assert!(
                    !denominator.is_zero_num(),
                    "generated scalar denominators stay nonzero"
                );
            }
            Ok::<_, Infallible>(None)
        })
        .expect("the visitor cannot fail");
    assert_eq!(result, checked, "inspection preserves the canonical graph");
    assert!(result.node_count() < 24, "large counts remain compressed");
}
