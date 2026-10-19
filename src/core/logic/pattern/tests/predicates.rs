//! Local capture pruning, repeated constraints, and transactional rejection.

use core::{
    ops::{Add, Mul},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{sync::Arc, vec};

use proptest::{prop_assert_eq, proptest};

use crate::core::{Context, Expr, Pattern, Property, RewriteRule, Truth, UserFunction};

proptest! {
    #[test]
    fn capture_predicates_filter_function_arguments(value in -30_i64..=30) {
        let context = Context::new();
        let host = context.register_function("host", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", Some(Arc::new(|node| node.properties().get(Property::Positive) == Truth::True)));
        let rule = RewriteRule::new(Pattern::apply(host.clone(), vec![capture.clone()]).expect("unary template"), capture).expect("bound capture");
        let input = host.call(vec![Expr::number(value)]).expect("unary call");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(if value > 0 { Expr::number(value) } else { input }));
    }

    #[test]
    fn collected_named_captures_keep_all_their_conditions(value in -20_i64..=20) {
        let context = Context::new();
        let host = context.register_function("host", UserFunction::new(1..=1)).expect("unbound function");
        let positive = Pattern::capture("a", Some(Arc::new(|node| node.properties().get(Property::Positive) == Truth::True)));
        let integer = Pattern::capture("a", Some(Arc::new(|node| node.properties().get(Property::Integer) == Truth::True)));
        let input_pattern = Pattern::apply(host.clone(), vec![Pattern::sum(vec![positive, integer])]).expect("unary template");
        let rule = RewriteRule::new(input_pattern, Pattern::capture("a", None)).expect("bound capture");
        let input = host.call(vec![Expr::number(value.saturating_mul(2))]).expect("unary call");
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(if value > 0 { Expr::number(value) } else { input }));
        let irrational = host.call(vec![Mul::mul(2, Expr::pi())]).expect("unary call");
        prop_assert_eq!(rule.apply_all(&irrational, None, None), Ok(irrational));
    }

    #[test]
    fn failed_local_conditions_do_not_reach_the_rule_guard(extra in 1_i64..=20) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let a = Pattern::capture("a", Some(Arc::new(|_| false)));
        let b = Pattern::capture("b", None);
        let rule = RewriteRule::new(Add::add(a, b), Pattern::exact(x)).expect("literal destination");
        let input = Add::add(Add::add(x, y), extra);
        let mut guard_calls = 0_usize;
        prop_assert_eq!(rule.apply_all(&input, None, Some(&mut |_| { guard_calls = guard_calls.saturating_add(1); true })), Ok(input));
        prop_assert_eq!(guard_calls, 0);
    }

    #[test]
    fn realness_proofs_guard_a_square_root_identity(offset in 1_i64..=15) {
        let context = Context::new();
        let base = Add::add(context.symb("x")?.abs(), offset);
        let capture = Pattern::capture("a", Some(Arc::new(|node| node.properties().get(Property::Real) == Truth::True)));
        let rule = RewriteRule::new(capture.pow(2).sqrt(), capture.abs()).expect("bound capture");
        prop_assert_eq!(rule.apply_all(&base.pow(2).sqrt(), None, None), Ok(base.abs()));
    }

    #[test]
    fn rejected_whole_bases_backtrack_to_fractional_blocks(whole in 2_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let root = x.sqrt();
        let wanted = root.clone();
        let output = context.register_function("output", UserFunction::new(1..=1)).expect("unbound function");
        let capture = Pattern::capture("a", Some(Arc::new(move |node| *node == wanted)));
        let rule = RewriteRule::new(capture.pow(2), Pattern::apply(output.clone(), vec![capture]).expect("unary template")).expect("bound capture");
        let input = x.pow(Expr::number(num_anafis::Number::from_rational(num_anafis::RationalType::new(whole.saturating_mul(2).saturating_add(1), 2))));
        let replacement = output.call(vec![root.clone()]).expect("unary call").pow(whole);
        prop_assert_eq!(rule.apply_all(&input, None, None), Ok(Mul::mul(replacement, root)));
    }
}

#[test]
fn predicate_callbacks_are_not_run_during_template_construction() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let capture = Pattern::capture(
        "a",
        Some(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
            true
        })),
    );
    let normalized = Add::add(capture.clone(), capture);
    let _rule = RewriteRule::new(normalized, Pattern::capture("a", None)).expect("bound capture");
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}
