//! Generated user-function contracts.

use alloc::{sync::Arc, vec::Vec};

use proptest::{collection, prelude::any, prop_assert, prop_assert_eq, proptest};

use crate::{core::Context, errors::FunctionError};

use super::super::{BodyFn, Expr, UserFunction};

proptest! {
    #[test]
    fn replacing_a_draft_partial_preserves_registered_definitions(
        index in 0_usize..8,
        original_value in -100_i64..=100,
        replacement_value in -100_i64..=100,
    ) {
        let context = Context::new();
        let arity = index.saturating_add(1);
        let draft = UserFunction::new(arity..=arity)
            .partial(index, Arc::new(move |_| Expr::number(original_value)))?;
        let original = context.register_function("original", draft.clone())?;
        let replacement = draft.partial(index, Arc::new(move |_| Expr::number(replacement_value)))?;
        let revised = context.register_function("revised", replacement)?;
        let original_partial = original.definition().and_then(|definition| definition.partial_fn(index))
            .expect("the original registration owns its partial");
        let revised_partial = revised.definition().and_then(|definition| definition.partial_fn(index))
            .expect("the revised registration owns its partial");
        prop_assert_eq!(original_partial(&[]), Expr::number(original_value));
        prop_assert_eq!(revised_partial(&[]), Expr::number(replacement_value));
        prop_assert!(!Arc::ptr_eq(&original_partial, &revised_partial));
    }

    #[test]
    fn callback_installation_preserves_shared_allocation_and_clone_ownership(
        index in 0_usize..8,
        trailing_arguments in 1_usize..=8,
        value in -100_i64..=100,
    ) {
        let arity = index.saturating_add(trailing_arguments);
        let callback: BodyFn = Arc::new(move |_| Expr::number(value));
        let function = UserFunction::new(arity..=arity)
            .body(Arc::clone(&callback))
            .partial(index, Arc::clone(&callback))
            .expect("the generated index is within the accepted arity");
        prop_assert!(Arc::ptr_eq(&function.body_fn().expect("installed body"), &callback));
        prop_assert!(Arc::ptr_eq(&function.partial_fn(index).expect("installed partial"), &callback));
        prop_assert_eq!(Arc::strong_count(&callback), 3);
        let copy = function.clone();
        prop_assert_eq!(Arc::strong_count(&callback), 5);
        drop(function);
        prop_assert_eq!(Arc::strong_count(&callback), 3);
        prop_assert_eq!(copy.partial_fn(index).expect("retained partial")(&[]), Expr::number(value));
        drop(copy);
        prop_assert_eq!(Arc::strong_count(&callback), 1);
    }

    #[test]
    fn arbitrary_partial_indices_respect_exclusive_argument_bounds(
        maximum in any::<usize>(), index in any::<usize>(),
    ) {
        let callback = Arc::new(|_: &[Expr]| Expr::number(0));
        let result = UserFunction::new(maximum..=maximum).partial(index, callback);
        if index < maximum {
            let function = result.expect("the index belongs to an accepted argument list");
            prop_assert!(function.has_partial(index));
            prop_assert_eq!(function.partial_fn(index).expect("registered partial")(&[]), Expr::number(0));
        } else {
            let error = result.expect_err("an argument list ends before this index");
            prop_assert_eq!(error, FunctionError::InvalidPartialIndex { index, max_arity: maximum });
        }
    }

    #[test]
    fn generated_bodies_preserve_arity_and_shared_expression_handles(
        arity in 0_usize..=8,
        argument_values in collection::vec(-100_i64..=100, 0..=8)
    ) {
        let function = UserFunction::new(arity..=arity)
            .body(Arc::new(|args| args.first().cloned().unwrap_or_else(|| Expr::number(0))));
        let arguments = argument_values.into_iter().map(Expr::number).collect::<Vec<_>>();
        let argument_count = arguments.len();
        prop_assert!(function.arity().contains(&arity));
        prop_assert_eq!(function.arity().contains(&argument_count), arity == argument_count);
        let body = function.body_fn().expect("a registered body must be retrievable");
        prop_assert_eq!(
            body(&arguments),
            arguments.first().cloned().unwrap_or_else(|| Expr::number(0))
        );
    }

    #[test]
    fn generated_partial_indices_remain_sparse_sorted_unique_and_replaceable(
        maximum in 1_usize..=8,
        indices in collection::vec(0_usize..=12, 0..=24),
        replacement_index in 0_usize..=12,
        replacement_value in -100_i64..=100
    ) {
        let mut function = UserFunction::new(maximum..=maximum);
        for index in indices.iter().copied().filter(|index| *index < maximum) {
            let index_value = i64::try_from(index).expect("generated index fits in i64");
            let next = function
                .clone()
                .partial(index, Arc::new(move |_| Expr::number(index_value)));
            prop_assert!(next.is_ok(), "a valid partial index must be accepted");
            if let Ok(updated) = next {
                function = updated;
            }
        }
        if replacement_index < maximum {
            function = function
                .partial(replacement_index, Arc::new(move |_| Expr::number(replacement_value)))
                .expect("the replacement index is valid");
        }

        let mut expected = indices
            .iter()
            .copied()
            .filter(|index| *index < maximum)
            .collect::<Vec<_>>();
        if replacement_index < maximum {
            expected.push(replacement_index);
        }
        expected.sort_unstable();
        expected.dedup();
        let stored = function
            .partials
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>();
        prop_assert_eq!(&stored, &expected);
        for index in indices.iter().copied().filter(|index| *index >= maximum) {
            prop_assert!(
                function
                    .clone()
                    .partial(index, Arc::new(|_args| Expr::number(0)))
                    .is_err(),
                "an index outside the maximum arity must be rejected"
            );
        }
        if replacement_index < maximum {
            prop_assert_eq!(
                function.partial_fn(replacement_index).expect("replacement exists")(&[]),
                Expr::number(replacement_value)
            );
        }
        for index in expected {
            prop_assert!(function.has_partial(index));
            prop_assert!(function.partial_fn(index).is_some());
        }
    }
}
