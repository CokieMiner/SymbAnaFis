//! Accepted callback destinations across shared and counted logical matches.

use core::{
    ops::{Add, Div, Mul},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{sync::Arc, vec};

use num_anafis::{Number, RationalType};
use proptest::{prop_assert_eq, proptest};

use crate::{
    collections::HashMap,
    core::{Context, Expr, Pattern, Replacement, RewriteRule, UserFunction},
    errors::RewriteError,
};

proptest! {
    #[test]
    fn shared_inputs_reuse_one_callback_result_without_traversing_it(edges in 2_usize..=16) {
        let context = Context::new();
        let x = context.symb("x")?;
        let pack = context.register_function("pack", UserFunction::new(0..=16))?;
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.sin(), Replacement::Callback(Arc::new(move |captures| {
            observed.fetch_add(1, Ordering::Relaxed);
            captures.get("a").map_or_else(|| Expr::number(0), |value| Add::add(value.sin(), 1))
        })))?;
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
        let input = pack.call(vec![x.sin(); edges])?;
        let expected = pack.call(vec![Add::add(x.sin(), 1); edges])?;
        prop_assert_eq!(rule.apply_all(&input, None, None)?, expected);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn predicates_and_rejected_guards_do_not_construct_destinations(repetitions in 2_i64..=30) {
        let context = Context::new();
        let x = context.symb("x")?;
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.pow(2), Replacement::Callback(Arc::new(move |captures| {
            observed.fetch_add(1, Ordering::Relaxed);
            captures.get("a").map_or_else(|| Expr::number(0), Expr::cos)
        })))?;
        let input = x.pow(repetitions.saturating_mul(2));
        prop_assert_eq!(&rule.apply_all(&input, None, Some(&mut |_| false))?, &input);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
        prop_assert_eq!(rule.apply_all(&input, Some(0), None), Err(RewriteError::MatchLimitExceeded { limit: 0 }));
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
        let rejected = Pattern::capture("a", Some(Arc::new(|_| false)));
        let rejected_calls = Arc::clone(&calls);
        let filtered = RewriteRule::new(rejected.pow(2), Replacement::Callback(Arc::new(move |_| {
            rejected_calls.fetch_add(1, Ordering::Relaxed);
            Expr::number(0)
        })))?;
        prop_assert_eq!(filtered.apply_all(&input, None, None)?, input);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn captured_rational_groups_construct_one_destination(copies in 3_i64..=100_000) {
        let context = Context::new();
        let x = context.symb("x")?;
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let capture = Pattern::capture("a", None);
        let rule = RewriteRule::new(capture.sqrt(), Replacement::Callback(Arc::new(move |captures| {
            observed.fetch_add(1, Ordering::Relaxed);
            captures.get("a").map_or_else(|| Expr::number(0), Expr::exp)
        })))?;
        let input = x.pow(Number::from_rational(RationalType::new(copies, 2)));
        prop_assert_eq!(rule.apply_all(&input, Some(1), None)?, x.exp().pow(copies));
        prop_assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn callbacks_follow_accepted_commutative_bindings_and_keep_the_tail(extra in 1_i64..=20) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let a = Pattern::capture("a", None);
        let b = Pattern::capture("b", None);
        let template = Add::add(a.sin(), b.cos());
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let callback = RewriteRule::new(template.clone(), Replacement::Callback(Arc::new(move |captures| {
            observed.fetch_add(1, Ordering::Relaxed);
            match (captures.get("a"), captures.get("b")) {
                (Some(left), Some(right)) => Add::add(left.exp(), right.abs()),
                _ => Expr::number(0),
            }
        })))?;
        let compiled = RewriteRule::new(template, Add::add(a.exp(), b.abs()))?;
        let input = Expr::sum(vec![x.sin(), y.sin(), x.cos(), y.cos(), Expr::number(extra)]);
        let wanted = y.to_expr();
        let mut accepts = |captures: &HashMap<&str, Expr>| {
            captures.get("a") == Some(&wanted) && captures.get("b") == Some(&wanted)
        };
        let expected = compiled.apply_all(&input, None, Some(&mut accepts))?;
        prop_assert_eq!(callback.apply_all(&input, None, Some(&mut accepts))?, expected);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn capture_free_callbacks_preserve_concrete_group_semantics(kind in 0_u8..5, count in 3_i64..=12) {
        let context = Context::new();
        let x = context.symb("x")?;
        let y = context.symb("y")?;
        let u = context.symb("u")?.to_expr();
        let factor = y.sin();
        let (source, input) = match kind {
            0 => (Add::add(x, y), Add::add(Mul::mul(factor.clone(), x), Mul::mul(factor, y))),
            1 => (Mul::mul(2, x), Mul::mul(Expr::number(count), x)),
            2 => (x.pow(2), x.pow(count)),
            3 => (x.sqrt(), x.pow(Number::from_rational(RationalType::new(count, 2)))),
            _ => (Div::div(Expr::number(1), x), Div::div(Expr::number(1), x.pow(count))),
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let destination = u.clone();
        let dynamic = RewriteRule::new(Pattern::exact(&source), Replacement::Callback(Arc::new(move |captures| {
            assert!(captures.is_empty(), "capture-free sources supply no bindings");
            observed.fetch_add(1, Ordering::Relaxed);
            destination.clone()
        })))?;
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
        let compiled = RewriteRule::new(Pattern::exact(source), Pattern::exact(u))?;
        prop_assert_eq!(&dynamic.apply_all(&input, None, Some(&mut |_| false))?, &input);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 0);
        prop_assert_eq!(dynamic.apply_all(&input, None, None)?, compiled.apply_all(&input, None, None)?);
        prop_assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}

#[test]
fn capture_free_callbacks_stay_lazy_until_a_bounded_match_is_accepted() {
    let context = Context::new();
    let x = context.symb("x").expect("symbol allocation");
    let u = context.symb("u").expect("symbol allocation").to_expr();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let destination = u.clone();
    let rule = RewriteRule::new(
        Pattern::exact(x.pow(2)),
        Replacement::Callback(Arc::new(move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
            destination.clone()
        })),
    )
    .expect("callback destination has no template captures");
    assert_eq!(rule.apply_all(&x.sin(), None, None), Ok(x.sin()));
    assert_eq!(
        rule.apply_all(&x.pow(5), Some(0), None),
        Err(RewriteError::MatchLimitExceeded { limit: 0 })
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(
        rule.apply_all(&x.pow(5), Some(1), None),
        Ok(Mul::mul(u.pow(2), x))
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(rule.apply_all(&x.pow(2), Some(0), None), Ok(u));
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
