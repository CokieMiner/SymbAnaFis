//! Implicit positive powers, repeated coefficients, captures, and search limits.

use alloc::vec;

use num_anafis::Number;

use proptest::{prop_assert, prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, UserFunction},
};

use super::super::{Pattern, RewriteError, RewriteRule};

proptest! {
    #[test]
    fn scalar_patterns_capture_the_whole_symbolic_factor_before_splitting(
        coefficient in 2_i32..=12,
        exponent in 2_i64..=9,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the name is unbound");
        let a = Pattern::capture("a");
        let pattern = Pattern::product(vec![Pattern::exact(Expr::number(2)), a.clone()]);
        let replacement = Pattern::apply(f.clone(), vec![a]).expect("f is unary");
        let rule = RewriteRule::new(pattern, replacement).expect("a is bound");
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(exponent)]);
        let quotient = core::ops::Div::div(Number::from(coefficient), Number::from(2));
        let expected = Expr::product(vec![Expr::number(quotient), f.call(vec![x.pow(exponent)]).expect("f is unary")]);
        let unweighted = x.pow(exponent);
        let mut explicit = |captures: &HashMap<&str, Expr>| captures.get("a") == Some(&unweighted);
        prop_assert_eq!(rule.apply_all(&input, None, Some(&mut explicit)), Ok(expected));
        let implicit = Expr::product(vec![core::ops::Div::div(Expr::number(1),Expr::number(2)), unweighted.clone()]);
        let mut inferred = |captures: &HashMap<&str, Expr>| captures.get("a") == Some(&implicit);
        prop_assert_eq!(rule.apply_all(&unweighted, None, Some(&mut inferred)), Ok(f.call(vec![implicit.clone()]).expect("f is unary")));
    }

    #[test]
    fn whole_capture_backtracking_can_split_one_power_for_two_names(exponent in 2_i64..=12) {
        let context = Context::new();
        let x = context.symb("x");
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the name is unbound");
        let a = Pattern::capture("a");
        let b = Pattern::capture("b");
        let rule = RewriteRule::new(
            Pattern::product(vec![a.clone(), b.clone()]),
            Pattern::apply(pair.clone(), vec![a, b]).expect("pair is binary"),
        ).expect("both captures are bound");
        let expected = pair.call(vec![x.into(), x.pow(exponent.saturating_sub(1))]).expect("pair is binary");
        prop_assert_eq!(rule.apply_all(&x.pow(exponent), None, None), Ok(expected));
        let x_expression = x.to_expr();
        let mut units_only = |captures: &HashMap<&str, Expr>| captures.values().all(|value| value.same_node(&x_expression));
        let copies = exponent.checked_div(2).expect("two is nonzero");
        let mut factors = vec![pair.call(vec![x.into(), x.into()]).expect("pair is binary").pow(copies)];
        if exponent & 1 != 0 { factors.push(x.into()); }
        prop_assert_eq!(rule.apply_all(&x.pow(exponent), None, Some(&mut units_only)), Ok(Expr::product(factors)));
    }

    #[test]
    fn exact_power_rules_consume_counted_factors_and_preserve_remainders(
        exponent in 2_i64..=15,
        requested in 2_i64..=5,
        coefficient in 1_i32..=7,
        x_value in -2_i64..=2,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let u = context.symb("u");
        let source = x.pow(requested);
        let rule = RewriteRule::new(Pattern::exact(source.clone()), Pattern::exact(u))
            .expect("literal replacements have no captures");
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(exponent)]);
        let copies = exponent.checked_div(requested).expect("requested power is positive");
        let remainder = exponent.checked_rem(requested).expect("requested power is positive");
        let expected = if copies == 0 { input.clone() } else {
            let mut factors = vec![Expr::number(coefficient), u.pow(copies)];
            if remainder > 0 { factors.push(x.pow(remainder)); }
            Expr::product(factors)
        };
        let result = rule.apply_all(&input, None, None).expect("unlimited search cannot exhaust");
        prop_assert_eq!(&result, &expected);
        let mut values = HashMap::from([(x, Expr::number(x_value))]);
        let source_value = source.substitute_symbols(&values);
        values.insert(u, source_value);
        prop_assert_eq!(result.substitute_symbols(&values), input.substitute_symbols(&values));
    }

    #[test]
    fn captured_power_factors_enforce_shared_function_arguments(
        x_power in 3_i64..=11,
        function_power in 1_i64..=7,
        coefficient in 1_i32..=6,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let u = context.symb("u");
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the name is unbound");
        let a = Pattern::capture("a");
        let call = Pattern::apply(f.clone(), vec![a.clone()]).expect("f is unary");
        let rule = RewriteRule::new(Pattern::product(vec![a.pow(2), call]), Pattern::exact(u)).expect("the RHS has no captures");
        let fx = f.call(vec![x.into()]).expect("f is unary");
        let input = Expr::product(vec![Expr::number(coefficient), x.pow(x_power), fx.pow(function_power)]);
        let copies = x_power.checked_div(2).expect("two is nonzero").min(function_power);
        let x_remaining = x_power.saturating_sub(copies.saturating_mul(2));
        let function_remaining = function_power.saturating_sub(copies);
        let mut factors = vec![Expr::number(coefficient), u.pow(copies)];
        if x_remaining > 0 { factors.push(x.pow(x_remaining)); }
        if function_remaining > 0 { factors.push(fx.pow(function_remaining)); }
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Expr::product(factors)));
        let wrong = Expr::product(vec![Expr::number(coefficient), x.pow(x_power), f.call(vec![y.into()]).expect("f is unary").pow(function_power)]);
        prop_assert!(rule.apply_all(&wrong, None, None).expect("unlimited search").same_node(&wrong));
    }

    #[test]
    fn weighted_sum_patterns_replace_collected_repetitions(
        x_count in 2_i32..=10,
        y_count in 2_i32..=10,
    ) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let z = context.symb("z");
        let u = context.symb("u");
        let rule = RewriteRule::new(Pattern::sum(vec![Pattern::exact(x), Pattern::exact(y)]), Pattern::exact(u))
            .expect("literal replacements have no captures");
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(x_count), x.into()]), Expr::product(vec![Expr::number(y_count), y.into()]), z.into()]);
        let copies = x_count.min(y_count);
        let mut terms = vec![Expr::product(vec![Expr::number(copies), u.into()]), z.into()];
        if x_count > copies { terms.push(Expr::product(vec![Expr::number(x_count.saturating_sub(copies)), x.into()])); }
        if y_count > copies { terms.push(Expr::product(vec![Expr::number(y_count.saturating_sub(copies)), y.into()])); }
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Expr::sum(terms)));
    }

    #[test]
    fn signed_collected_addends_match_patterns_without_erasing_the_sign(count in 2_i32..=10) {
        let context = Context::new();
        let x = context.symb("x");
        let y = context.symb("y");
        let u = context.symb("u");
        let negative_y = Expr::product(vec![Expr::number(-1), y.into()]);
        let rule = RewriteRule::new(Pattern::sum(vec![Pattern::exact(x), Pattern::exact(negative_y)]), Pattern::exact(u))
            .expect("there are no RHS captures");
        let input = Expr::sum(vec![Expr::product(vec![Expr::number(count), x.into()]), Expr::product(vec![Expr::number(count.saturating_neg()), y.into()])]);
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Expr::product(vec![Expr::number(count), u.into()])));
    }

    #[test]
    fn nested_function_templates_do_not_extract_a_factor_from_an_argument(exponent in 3_i64..=12) {
        let context = Context::new();
        let x = context.symb("x");
        let f = context.register_function("f", UserFunction::new(1..=1)).expect("the name is unbound");
        let rule = RewriteRule::new(Pattern::apply(f.clone(), vec![Pattern::exact(x.pow(2))]).expect("f is unary"), Pattern::exact(context.symb("u")))
            .expect("there are no RHS captures");
        let input = f.call(vec![x.pow(exponent)]).expect("f is unary");
        prop_assert!(rule.apply_all(&input, None, None).expect("unlimited search").same_node(&input));
    }

    #[test]
    fn distinct_captures_may_consume_disjoint_units_of_one_power(exponent in 4_i64..=13) {
        let context = Context::new();
        let x = context.symb("x");
        let pair = context.register_function("pair", UserFunction::new(2..=2)).expect("the name is unbound");
        let a = Pattern::capture("a");
        let b = Pattern::capture("b");
        let replacement = Pattern::apply(pair.clone(), vec![a.clone(), b.clone()]).expect("pair is binary");
        let rule = RewriteRule::new(Pattern::product(vec![a.pow(2), b.pow(2)]), replacement).expect("both captures are bound");
        let copies = exponent.checked_div(4).expect("four is nonzero");
        let remainder = exponent.checked_rem(4).expect("four is nonzero");
        let mut factors = vec![pair.call(vec![x.into(), x.into()]).expect("pair is binary").pow(copies)];
        if remainder > 0 { factors.push(x.pow(remainder)); }
        prop_assert_eq!(rule.apply_all(&x.pow(exponent), None, None), Ok(Expr::product(factors)));
    }

    #[test]
    fn guard_rejection_preserves_original_powers_and_rolls_back_captures(exponent in 3_i64..=15) {
        let context = Context::new();
        let x = context.symb("x");
        let a = Pattern::capture("a");
        let rule = RewriteRule::new(a.pow(2), Pattern::exact(context.symb("u"))).expect("no RHS captures");
        let input = x.pow(exponent);
        let mut reject = |_: &HashMap<&str, Expr>| false;
        prop_assert!(rule.apply_all(&input, None, Some(&mut reject)).expect("unlimited search").same_node(&input));
        prop_assert_eq!(rule.apply_all(&input, Some(0), None), Err(RewriteError::MatchLimitExceeded { limit: 0 }));
    }
}

#[test]
fn a_single_virtual_candidate_handles_the_largest_exponent() {
    let context = Context::new();
    let x = context.symb("x");
    let u = context.symb("u");
    let a = Pattern::capture("a");
    let rule = RewriteRule::new(a.pow(2), Pattern::exact(u)).expect("no RHS captures");
    let expected = Expr::product(vec![
        u.pow(i64::MAX.checked_div(2).expect("two is nonzero")),
        x.into(),
    ]);
    let result = rule
        .apply_all(&x.pow(i64::MAX), Some(1), None)
        .expect("one distinct candidate");
    assert_eq!(result, expected);
    assert!(result.node_count() < 12, "the result stays compressed");
}
