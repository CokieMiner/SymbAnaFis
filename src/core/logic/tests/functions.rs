//! Immutable function definitions retained by handles, expressions, and patterns.

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{format, sync::Arc, vec};

use proptest::{prop_assert, prop_assert_eq, prop_assert_ne, proptest};

use crate::core::{Context, Expr, ExprView, Pattern, RewriteRule, UserFunction};

proptest! {
    #[test]
    fn expressions_retain_definitions_through_every_namespace_release(
        suffix in "[a-z]{1,24}",
        value in -1_000_i64..=1_000,
        arity in 1_usize..=8,
        cleanup in 0_u8..=4,
    ) {
        let mut context = Context::new();
        let name = format!("retained_{suffix}");
        let capture = Arc::new(value);
        let weak_capture = Arc::downgrade(&capture);
        let body_capture = Arc::clone(&capture);
        let partial_index = arity.checked_sub(1).expect("the generated arity is positive");
        let definition = UserFunction::new(arity..=arity)
            .body(Arc::new(move |_| Expr::number(*body_capture)))
            .partial(partial_index, Arc::new(move |_| Expr::number(*capture)))
            .expect("the final argument index is valid");
        let function = context.register_function(&name, definition).expect("the name is unbound");
        let expression = function.call(vec![Expr::number(1); arity]).expect("the arity is valid");
        drop(function);
        match cleanup {
            0 => prop_assert!(context.remove_function(&name)),
            1 => context.clear_functions(),
            2 => context.clear_all(),
            3 => context.reset(),
            _ => {}
        }
        drop(context);
        prop_assert!(weak_capture.upgrade().is_some());
        let ExprView::Function { name: retained, args } = expression.view() else {
            return Err(proptest::test_runner::TestCaseError::fail("custom calls stay symbolic"));
        };
        let retained_definition = retained.definition().expect("the expression owns its definition");
        prop_assert_eq!(retained_definition.arity(), arity..=arity);
        let body = retained_definition.body_fn().expect("the body survives namespace release");
        let partial = retained_definition.partial_fn(partial_index).expect("the partial survives namespace release");
        prop_assert_eq!(body(args), Expr::number(value));
        prop_assert_eq!(partial(args), Expr::number(value));
        drop(body);
        drop(partial);
        prop_assert!(weak_capture.upgrade().is_some());
        drop(expression);
        prop_assert!(weak_capture.upgrade().is_none());
    }

    #[test]
    fn rebinding_keeps_previous_callbacks_arities_and_canonical_calls(
        suffix in "[a-z]{1,24}",
        previous_value in -1_000_i64..=1_000,
        current_value in -1_000_i64..=1_000,
        previous_arity in 1_usize..=5,
        current_arity in 1_usize..=5,
    ) {
        let context = Context::new();
        let name = format!("rebound_{suffix}");
        let previous = context.register_function(
            &name,
            UserFunction::new(previous_arity..=previous_arity)
                .body(Arc::new(move |_| Expr::number(previous_value)))
                .partial(0, Arc::new(move |_| Expr::number(previous_value)))
                .expect("argument zero is present"),
        ).expect("the name is unbound");
        let original = previous.call(vec![Expr::number(1); previous_arity]).expect("the previous arity is valid");
        prop_assert!(context.remove_function(&name));
        let current = context.register_function(
            &name,
            UserFunction::new(current_arity..=current_arity)
                .body(Arc::new(move |_| Expr::number(current_value)))
                .partial(0, Arc::new(move |_| Expr::number(current_value)))
                .expect("argument zero is present"),
        ).expect("removal releases the name");
        prop_assert_ne!(&previous, &current);
        let registered = context.get_function(&name).expect("the new identity is registered");
        prop_assert_eq!(&registered, &current);
        drop(context);
        prop_assert_eq!(
            previous.call(vec![Expr::number(1); previous_arity]).expect("the previous contract remains valid"),
            original,
        );
        for (function, expected_arity, expected_value) in [
            (previous, previous_arity, previous_value),
            (current, current_arity, current_value),
        ] {
            let definition = function.definition().expect("custom definition");
            let arguments = vec![Expr::number(1); expected_arity];
            prop_assert_eq!(definition.arity(), expected_arity..=expected_arity);
            prop_assert_eq!(definition.body_fn().expect("registered body")(&arguments), Expr::number(expected_value));
            prop_assert_eq!(definition.partial_fn(0).expect("registered partial")(&arguments), Expr::number(expected_value));
        }
    }

    #[test]
    fn cloned_drafts_do_not_mutate_a_registered_definition(
        original_value in -1_000_i64..=1_000,
        replacement_value in -1_000_i64..=1_000,
    ) {
        let context = Context::new();
        let original = UserFunction::new(1..=1)
            .body(Arc::new(move |_| Expr::number(original_value)))
            .partial(0, Arc::new(move |_| Expr::number(original_value)))
            .expect("argument zero is present");
        let function = context.register_function("immutable_definition", original).expect("the name is unbound");
        let definition = function.definition().expect("custom definition");
        let draft = definition.clone()
            .body(Arc::new(move |_| Expr::number(replacement_value)))
            .partial(0, Arc::new(move |_| Expr::number(replacement_value)))
            .expect("argument zero is present");
        let replacement = context.register_function("independent_definition", draft).expect("the second name is unbound");
        prop_assert!(core::ptr::eq(definition, function.definition().expect("the definition is borrowed")));
        let arguments = [Expr::number(1)];
        prop_assert_eq!(definition.body_fn().expect("the original body")(&arguments), Expr::number(original_value));
        prop_assert_eq!(definition.partial_fn(0).expect("the original partial")(&arguments), Expr::number(original_value));
        let replacement_definition = replacement.definition().expect("the replacement definition");
        prop_assert_eq!(replacement_definition.body_fn().expect("the replacement body")(&arguments), Expr::number(replacement_value));
        prop_assert_eq!(replacement_definition.partial_fn(0).expect("the replacement partial")(&arguments), Expr::number(replacement_value));
    }

    #[test]
    fn retained_patterns_build_calls_without_executing_custom_callbacks(
        value in -1_000_i64..=1_000,
    ) {
        let invocations = Arc::new(AtomicUsize::new(0));
        let weak_invocations = Arc::downgrade(&invocations);
        let rule = {
            let context = Context::new();
            let counter = Arc::clone(&invocations);
            let function = context.register_function(
                "retained_pattern",
                UserFunction::new(1..=1).body(Arc::new(move |args| {
                    counter.fetch_add(1, Ordering::Relaxed);
                    Expr::sum(args.to_vec())
                })),
            ).expect("the name is unbound");
            let argument = Pattern::capture("argument", None);
            RewriteRule::new(argument.clone(), Pattern::apply(function, vec![argument]).expect("unary pattern"))
                .expect("the replacement capture is bound")
        };
        let input = Expr::number(value);
        let rewritten = rule.apply_all(&input, None, None).expect("unrestricted rewriting");
        prop_assert_eq!(invocations.load(Ordering::Relaxed), 0);
        let ExprView::Function { name, args } = rewritten.view() else {
            return Err(proptest::test_runner::TestCaseError::fail("the replacement builds a custom call"));
        };
        let body = name.definition().and_then(UserFunction::body_fn).expect("the pattern retains the body");
        prop_assert_eq!(body(args), input);
        prop_assert_eq!(invocations.load(Ordering::Relaxed), 1);
        drop(body);
        drop(invocations);
        drop(rewritten);
        prop_assert!(weak_invocations.upgrade().is_some());
        drop(rule);
        prop_assert!(weak_invocations.upgrade().is_none());
    }
}
