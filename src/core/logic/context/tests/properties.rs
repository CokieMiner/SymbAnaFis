//! Generated context contracts.

use alloc::{string::String, sync::Arc, vec};

use proptest::{
    collection, prop_assert, prop_assert_eq, prop_assert_ne, prop_assume, proptest,
    strategy::Strategy,
};

use crate::{
    collections::HashSet,
    core::{Expr, ExprView, Function, Symbol},
    errors::{FunctionError, SymbolError},
};

use super::super::{Context, SymbolData, UserFunction};

fn local_name_strategy() -> impl Strategy<Value = String> {
    collection::vec(0_u8..26, 1..=8).prop_map(|letters| {
        let mut name = String::from("ctx_");
        for letter in letters {
            name.push(char::from(b'a'.saturating_add(letter)));
        }
        name
    })
}

proptest! {
    #[test]
    fn generated_contexts_stay_isolated_from_each_other_and_global_names(
        name in local_name_strategy()
    ) {
        let global = Symbol::intern("context_global_name");
        let first = Context::new();
        let second = Context::new();
        let first_symbol = first.symb(&name);
        let second_symbol = second.symb(&name);
        prop_assert_ne!(first_symbol, second_symbol);
        prop_assert_ne!(first_symbol, global);
        prop_assert_ne!(first_symbol.to_expr(), second_symbol.to_expr());
        let cloned = first.clone();
        prop_assert_eq!(cloned.symb(&name), first_symbol);
        prop_assert_eq!(first.id(), cloned.id());
    }

    #[test]
    fn generated_registrations_register_and_clear_unique_bindings(
        symbol_names in collection::vec(local_name_strategy(), 1..=16),
        function_names in collection::vec(local_name_strategy(), 1..=16)
    ) {
        let expected_symbols: HashSet<_> = symbol_names.iter().cloned().collect();
        let expected_functions: HashSet<_> = function_names.iter().cloned().collect();
        let context = Context::new();
        for name in symbol_names {
            let registered = context.symb(&name);
            prop_assert_eq!(context.get_symbol(&name), Some(registered));
        }
        for name in &expected_functions {
            context.register_function(name, UserFunction::default())
                .expect("unique custom names register once");
        }
        prop_assert_eq!(context.symbol_count(), expected_symbols.len());
        for name in &expected_symbols {
            prop_assert!(context.contains_symbol(name));
            prop_assert!(context.get_symbol(name).is_some());
        }
        for name in &expected_functions {
            prop_assert!(context.has_function(name));
        }
        prop_assert_eq!(context.symbol_names_set(), expected_symbols);
        prop_assert_eq!(context.function_names(), expected_functions);
        context.clear_all();
        prop_assert!(context.is_empty());
    }

    #[test]
    fn generated_missing_function_queries_do_not_register_names(
        name in local_name_strategy(),
        index in 0_usize..=8
    ) {
        let context = Context::new();
        prop_assert!(!Symbol::exists(&name));
        prop_assert!(context.get_user_fn(&name).is_none());
        prop_assert!(context.get_body(&name).is_none());
        prop_assert!(context.get_partial(&name, index).is_none());
        prop_assert!(!context.has_function(&name));
        prop_assert!(!Symbol::exists(&name));
    }

    #[test]
    fn generated_registered_bodies_use_shared_expression_handles(
        name in local_name_strategy(),
        value in -100_i64..=100,
        offset in -100_i64..=100
    ) {
        let context = Context::new();
        context.register_function(
            &name,
            UserFunction::new(1..=1).body(Arc::new(move |args| {
                Expr::sum(vec![
                    args.first().cloned().unwrap_or_else(|| Expr::number(0)),
                    Expr::number(offset),
                ])
            })),
        ).expect("the custom name is unbound");
        let body = context.get_body(&name).expect("registered body");
        let argument = Expr::number(value);
        let function = context.get_function(&name).expect("registered function");
        prop_assert_eq!(
            body(core::slice::from_ref(&argument)),
            Expr::sum(vec![argument, Expr::number(offset)])
        );
        prop_assert!(context.has_expandable_functions());
        prop_assert_eq!(context.get_function(&name), Some(function));
    }

    #[test]
    fn duplicate_registration_preserves_bodies_partials_and_arities(
        name in local_name_strategy(),
        new_name in local_name_strategy(),
        arity in 1_usize..=8,
        partial_index in 0_usize..=7,
        body_value in -100_i64..=100,
        partial_value in -100_i64..=100
    ) {
        prop_assume!(name != new_name);
        prop_assume!(partial_index < arity);
        let context = Context::new();
        let definition = UserFunction::new(arity..=arity)
            .body(Arc::new(move |_| Expr::number(body_value)))
            .partial(partial_index, Arc::new(move |_| Expr::number(partial_value)))
            .expect("generated partial index is valid");
        let function = context.register_function(&name, definition)
            .expect("the custom name is unbound");
        let original_body = context.get_body_for(&function).expect("registered body");
        let original_partial = context
            .get_partial_for(&function, partial_index)
            .expect("registered partial");
        prop_assert_eq!(
            context.register_function(&name, UserFunction::default()),
            Err(FunctionError::DuplicateName(name.clone()))
        );
        context.register_function(&new_name, UserFunction::default())
            .expect("the second custom name is unbound");
        prop_assert_eq!(context.get_function(&name), Some(function.clone()));
        prop_assert!(Arc::ptr_eq(
            &original_body,
            &context.get_body_for(&function).expect("preserved body")
        ));
        prop_assert!(Arc::ptr_eq(
            &original_partial,
            &context
                .get_partial_for(&function, partial_index)
                .expect("preserved partial")
        ));
        let preserved = context.get_user_fn_for(&function).expect("definition");
        prop_assert!(preserved.accepts_arity(arity));
        prop_assert!(!preserved.accepts_arity(arity.saturating_sub(1)));
        let placeholder = context.get_user_fn(&new_name).expect("new placeholder");
        prop_assert!(!placeholder.has_body());
        prop_assert!(!placeholder.has_partial(0));
    }

    #[test]
    fn generated_context_release_reclaims_unused_metadata_but_expressions_retain_symbols(
        name in local_name_strategy(),
        unused_name in local_name_strategy()
    ) {
        prop_assume!(name != unused_name);
        let context = Context::new();
        let clone = context.clone();
        let symbol = clone.symb(&name);
        let id = symbol.id();
        let expression = symbol.to_expr();
        let metadata = Arc::downgrade(&SymbolData::lookup(id).expect("live local ID"));
        let unused = context.symb(&unused_name).id();
        let unused_metadata = Arc::downgrade(&SymbolData::lookup(unused).expect("unused ID"));
        prop_assert!(!Symbol::exists(&name));
        drop(context);
        prop_assert!(symbol.to_expr().same_node(&expression));
        drop(clone);
        prop_assert!(unused_metadata.upgrade().is_none());
        prop_assert!(metadata.upgrade().is_some());
        prop_assert_eq!(expression.symbol_name(), Some(name.as_str()));
        let borrowed = expression.as_symbol().expect("expression owns symbol");
        prop_assert!(borrowed.to_expr().same_node(&expression));
        drop(expression);
        prop_assert!(metadata.upgrade().is_none());
        prop_assert_eq!(Symbol::from_id(id), Err(SymbolError::InvalidId(id)));
    }

    #[test]
    fn generated_removal_and_rebinding_keep_old_expressions_valid(
        name in local_name_strategy()
    ) {
        let context = Context::new();
        let manager = context.clone();
        let old = context.symb(&name);
        let retained = old.to_expr();
        prop_assert!(context.remove_symbol(&name));
        prop_assert!(old.to_expr().same_node(&retained));
        let replacement = context.symb(&name);
        prop_assert_ne!(old, replacement);
        manager.clear_symbols();
        let replacement_expression = replacement.to_expr();
        prop_assert_eq!(replacement_expression.as_symbol(), Some(replacement));
        let fresh = context.symb(&name);
        prop_assert_ne!(fresh, replacement);
        manager.clear_all();
        prop_assert_eq!(context.symbol_from_id(old.id()), Ok(old));
        prop_assert_eq!(context.symbol_from_id(replacement.id()), Ok(replacement));
        prop_assert!(old.to_expr().same_node(&retained));
    }

    #[test]
    fn generated_reset_detaches_namespaces_and_preserves_live_expressions(
        name in local_name_strategy(),
        unused_name in local_name_strategy()
    ) {
        prop_assume!(name != unused_name);
        let mut context = Context::new();
        let previous = context.id();
        let symbol = context.symb(&name);
        let id = symbol.id();
        let expression = Expr::sum(vec![symbol.to_expr(), Expr::number(1)]);
        let metadata = Arc::downgrade(&SymbolData::lookup(id).expect("used local ID"));
        let unused = context.symb(&unused_name).id();
        let unused_metadata = Arc::downgrade(&SymbolData::lookup(unused).expect("unused ID"));
        context.reset();
        prop_assert_ne!(context.id(), previous);
        prop_assert!(context.is_empty());
        prop_assert_eq!(context.symbol_from_id(id), Err(SymbolError::InvalidId(id)));
        prop_assert!(unused_metadata.upgrade().is_none());
        prop_assert!(metadata.upgrade().is_some());
        prop_assert!(expression.symbols().iter().any(|referenced| referenced.id() == id));
        prop_assert_ne!(context.symb(&name).id(), id);
    }

    #[test]
    fn generated_clone_reset_preserves_the_other_namespace_owner(
        name in local_name_strategy()
    ) {
        let mut context = Context::new();
        let mut manager = context.clone();
        let owner = context.clone();
        let symbol = owner.symb(&name);
        let id = symbol.id();
        let metadata = Arc::downgrade(&SymbolData::lookup(id).expect("local ID"));
        context.reset();
        prop_assert_eq!(owner.id(), manager.id());
        prop_assert_ne!(owner.id(), context.id());
        manager.clear_all();
        manager.reset();
        prop_assert_eq!(owner.symbol_from_id(id), Ok(symbol));
        let symbol_expression = symbol.to_expr();
        prop_assert_eq!(symbol_expression.as_symbol(), Some(symbol));
        prop_assert!(metadata.upgrade().is_some());
        drop(symbol_expression);
        drop(manager);
        drop(owner);
        prop_assert!(metadata.upgrade().is_none());
    }

    #[test]
    fn generated_raw_local_ids_require_their_owner(
        name in local_name_strategy()
    ) {
        let context = Context::new();
        let other = Context::new();
        let symbol = context.symb(&name);
        prop_assert_eq!(context.symbol_from_id(symbol.id()), Ok(symbol));
        prop_assert_eq!(other.symbol_from_id(symbol.id()), Err(SymbolError::InvalidId(symbol.id())));
        prop_assert_eq!(Symbol::from_id(symbol.id()), Err(SymbolError::InvalidId(symbol.id())));
        prop_assert_eq!(context.symbol_from_id(0), Err(SymbolError::InvalidId(0)));
        prop_assert_eq!(core::mem::size_of::<Symbol<'_>>(), core::mem::size_of::<usize>());
    }

    #[test]
    fn generated_local_functions_keep_namespace_identity(
        name in local_name_strategy(),
        marker in -100_i64..=100
    ) {
        let first = Context::new();
        let second = Context::new();
        let function_name = name.as_str();
        let local = first.register_function(
            function_name,
            UserFunction::new(0..=0).body(Arc::new(move |_| Expr::number(marker))),
        ).expect("the custom name is unbound");
        let other = second.register_function(function_name, UserFunction::new(0..=0))
            .expect("independent contexts may bind the same custom name");
        prop_assert_ne!(&local, &other);
        prop_assert!(Function::builtin(function_name).is_none());
        let local_call = local.call(vec![]).expect("the local function accepts zero arguments");
        let is_function = matches!(local_call.view(), ExprView::Function { .. });
        prop_assert!(is_function);
        prop_assert_eq!(first.get_function(function_name), Some(local.clone()));
        prop_assert!(second.get_body_for(&local).is_none());
        prop_assert_eq!(
            first.get_body_for(&local).expect("registered body")(&[]),
            Expr::number(marker)
        );
    }
}
