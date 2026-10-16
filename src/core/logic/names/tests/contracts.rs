//! Registered function ownership, uniqueness, and argument contracts.

use std::{sync::Barrier, thread};

use alloc::{format, sync::Arc, vec, vec::Vec};

use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use crate::{
    core::{Context, Expr, Pattern, UserFunction},
    errors::FunctionError,
};

use super::super::{FunctionIdentity, FunctionName};

proptest! {
    #[test]
    fn registered_metadata_outlives_context_and_reclaims_after_last_expression(
        suffix in "[a-z]{1,24}", value in -100_i64..=100,
    ) {
        let context = Context::new();
        let name = format!("custom_{suffix}");
        let function = context.register_function(&name, UserFunction::new(1..=1))
            .expect("the custom name is unbound");
        let same = context.get_function(&name).expect("the name is registered");
        let FunctionIdentity::Custom(data) = &function.0 else {
            return Err(proptest::test_runner::TestCaseError::fail("a custom registration has owning metadata"));
        };
        let weak = Arc::downgrade(data);
        let name_weak = Arc::downgrade(&data.name);
        let expression = function.call(vec![Expr::number(value)])
            .expect("the registered arity is one");
        prop_assert_eq!(&function, &same);
        drop(context);
        prop_assert_eq!(&function.call(vec![Expr::number(value)]).expect("the handle retains its arity"), &expression);
        drop(function);
        drop(same);
        prop_assert!(weak.upgrade().is_some());
        drop(expression);
        prop_assert!(weak.upgrade().is_none());
        prop_assert!(name_weak.upgrade().is_none());
    }

    #[test]
    fn duplicate_names_fail_and_removal_allows_a_fresh_arity(
        suffix in "[a-z]{1,24}", first_arity in 0_usize..=5, next_arity in 6_usize..=10,
    ) {
        let context = Context::new();
        let name = format!("custom_{suffix}");
        let first = context.register_function(&name, UserFunction::new(first_arity..=first_arity))
            .expect("the custom name is unbound");
        prop_assert_eq!(
            context.register_function(&name, UserFunction::new(next_arity..=next_arity)),
            Err(FunctionError::DuplicateName(name.clone()))
        );
        prop_assert_eq!(context.get_function(&name), Some(first.clone()));
        prop_assert!(context.remove_function(&name));
        let next = context.register_function(&name, UserFunction::new(next_arity..=next_arity))
            .expect("removal releases the name binding");
        prop_assert_ne!(&first, &next);
        prop_assert_eq!(first.arity(), first_arity..=first_arity);
        prop_assert_eq!(next.arity(), next_arity..=next_arity);
        prop_assert!(first.call(vec![Expr::number(1); first_arity]).is_ok());
        prop_assert!(next.call(vec![Expr::number(1); next_arity]).is_ok());
    }

    #[test]
    fn calls_and_patterns_reject_unregistered_arities(
        minimum in 0_usize..=5, width in 0_usize..=5, actual in 0_usize..=12,
    ) {
        let maximum = minimum.checked_add(width).expect("bounded arities fit usize");
        let context = Context::new();
        let function = context.register_function("arity_property", UserFunction::new(minimum..=maximum))
            .expect("the custom name is unbound");
        let call = function.call(vec![Expr::number(1); actual]);
        let pattern = Pattern::apply(function, vec![Pattern::capture("a"); actual]);
        let accepts = (minimum..=maximum).contains(&actual);
        prop_assert_eq!(call.is_ok(), accepts);
        prop_assert_eq!(pattern.is_ok(), accepts);
        if !accepts {
            prop_assert_eq!(call, Err(FunctionError::InvalidArity { minimum, maximum, actual }));
        }
    }

    #[test]
    fn empty_arity_ranges_do_not_publish_function_metadata(
        minimum in 1_usize..=100, difference in 1_usize..=100,
    ) {
        let maximum = minimum.saturating_sub(difference);
        let context = Context::new();
        prop_assert_eq!(
            context.register_function("invalid_range", UserFunction::new(minimum..=maximum)),
            Err(FunctionError::InvalidArityRange { minimum, maximum })
        );
        prop_assert!(context.get_function("invalid_range").is_none());
        prop_assert!(context.is_empty());
    }

    #[test]
    fn builtin_names_and_aliases_are_reserved(
        name in proptest::sample::select(vec!["sin", "cos", "hermite", "bessel_j", "besselj", "lambert_w", "lambertw"]),
        actual in 0_usize..=5,
    ) {
        let context = Context::new();
        prop_assert_eq!(
            context.register_function(name, UserFunction::default()),
            Err(FunctionError::DuplicateName(name.into()))
        );
        prop_assert!(context.get_function(name).is_none());
        let builtin = FunctionName::builtin(name).expect("the sampled builtin name is known");
        prop_assert_eq!(
            builtin.call(vec![Expr::symbol("builtin_arity_argument"); actual]).is_ok(),
            builtin.arity().contains(&actual)
        );
    }
}

#[test]
fn concurrent_final_handles_reclaim_metadata_without_registry_access() {
    let context = Context::new();
    let first = context
        .register_function("concurrent_release", UserFunction::new(0..=0))
        .expect("the custom name is unbound");
    assert!(
        matches!(&first.0, FunctionIdentity::Custom(_)),
        "registration owns custom metadata"
    );
    let FunctionIdentity::Custom(data) = &first.0 else {
        return;
    };
    let weak = Arc::downgrade(data);
    let second = first.clone();
    drop(context);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = [first, second]
        .into_iter()
        .map(|handle| {
            let ready = Arc::clone(&barrier);
            thread::spawn(move || {
                ready.wait();
                drop(handle);
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("the owner release worker completed");
    }
    assert!(
        weak.upgrade().is_none(),
        "the final owners reclaim registered metadata"
    );
}
