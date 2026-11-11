//! Local and global symbol metadata: index membership, removal, and raw identities.

use core::ops::{Add, Div, Mul, Sub};

use alloc::{format, string::String, vec, vec::Vec};

use proptest::prelude::{prop, prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Expr, Symbol},
    errors::SymbolError,
};

use super::super::super::symbols::{SymbolData, SymbolRegistry};

proptest! {
    #[test]
    fn generated_local_metadata_stays_out_of_the_global_index(
        suffix in prop::collection::vec(prop::char::range('a', 'z'), 1..=24),
    ) {
        let name = format!("property_local_{}", suffix.into_iter().collect::<String>());
        let data = SymbolData::local(Some(name.into())).expect("bounded local identity");
        let id = data.id();
        let observer = alloc::sync::Arc::downgrade(&data);

        prop_assert_eq!(Symbol::from_id(id), Err(SymbolError::InvalidId(id)));
        let expression = data.symbol().to_expr();
        prop_assert_eq!(expression.as_symbol(), Some(data.symbol()));
        drop(expression);

        drop(data);
        prop_assert!(observer.upgrade().is_none());
    }
}

#[test]
fn unregistering_preserves_replacements_and_local_ownership() {
    let registry = SymbolRegistry::new();
    let first = registry.intern("by_id").expect("global name");
    let anonymous = registry.anonymous().expect("anonymous identity");
    let local = SymbolData::local(Some("by_id".into())).expect("local name");
    assert!(!registry.unregister(&local));
    assert!(registry.unregister(&SymbolData::of(first)));
    let replacement = registry.intern("by_id").expect("replacement name");
    assert!(!registry.unregister(&SymbolData::of(first)));
    assert!(SymbolData::lookup_permanent(replacement.id()).is_ok());
    assert!(registry.unregister(&SymbolData::of(anonymous)));
    assert!(SymbolData::lookup_permanent(anonymous.id()).is_ok());
    drop(registry);
    assert_eq!(local.symbol().to_expr().symbol_name(), Some("by_id"));
}

#[test]
fn global_removal_and_drop_preserve_context_symbols_and_retained_expressions() {
    let context = Context::new().expect("context");
    let local = context
        .symb("registry_independent_local")
        .expect("symbol allocation");
    let registry = SymbolRegistry::new();
    let global = registry
        .intern("registry_independent_local")
        .expect("global registration");
    let global_symbol = Symbol::from_id(global.id()).expect("published global ID");
    let global_expr = global_symbol.to_expr();
    assert!(registry.remove("registry_independent_local"));
    assert!(SymbolData::lookup_permanent(global.id()).is_ok());
    assert_eq!(local.to_expr().as_symbol(), Some(local));
    assert_eq!(
        global_expr.symbol_name(),
        Some("registry_independent_local")
    );
    let another = registry
        .intern("registry_drop_id")
        .expect("global registration");
    drop(registry);
    assert!(SymbolData::lookup_permanent(another.id()).is_ok());
    assert_eq!(local.to_expr().as_symbol(), Some(local));
    assert_eq!(global_symbol.to_expr(), global_expr);
}

#[test]
fn raw_id_reconstruction_rejects_unpublished_identities() {
    assert_eq!(Symbol::from_id(0), Err(SymbolError::InvalidId(0)));
    assert_eq!(
        Symbol::from_id(usize::MAX),
        Err(SymbolError::InvalidId(usize::MAX))
    );
    let registry = SymbolRegistry::new();
    let data = registry.intern("checked_raw_symbol").expect("registration");
    let symbol = Symbol::from_id(data.id()).expect("published identity");
    drop(registry);
    assert_eq!(Symbol::from_id(symbol.id()), Ok(symbol));
    assert!((symbol + 1).contains_symbol(symbol));
    assert_eq!(symbol.as_str(), Some("checked_raw_symbol"));
}

#[test]
fn symbol_as_str_and_display_cover_named_and_anonymous() {
    let named = Symbol::intern("display_named_symbol").expect("global symbol");
    assert_eq!(named.as_str(), Some("display_named_symbol"));
    assert_eq!(format!("{named}"), "display_named_symbol");
    assert_eq!(format!("{named}"), format!("{}", named.to_expr()));
    assert!(Symbol::remove("display_named_symbol"));
    assert_eq!(named.as_str(), Some("display_named_symbol"));
    assert_eq!(format!("{named}"), "display_named_symbol");

    let anon = Symbol::anon().expect("anonymous symbol");
    assert_eq!(anon.as_str(), None);
    let expected_anon = format!("${}", anon.id());
    assert_eq!(format!("{anon}"), expected_anon);
    assert_eq!(format!("{anon}"), format!("{}", anon.to_expr()));

    let context = Context::new().expect("context");
    let local = context.symb("display_local_symbol").expect("local symbol");
    assert_eq!(local.as_str(), Some("display_local_symbol"));
    assert_eq!(format!("{local}"), "display_local_symbol");
    assert!(context.remove_symbol("display_local_symbol"));
    assert_eq!(local.as_str(), Some("display_local_symbol"));
}

#[test]
fn symbol_operators_with_i64_rhs_and_i32_lhs_match_expr_arithmetic() {
    let context = Context::new().expect("context");
    let x = context.symb("i64_op_x").expect("local symbol");
    let x_expr = x.to_expr();
    for value in [
        i64::MIN,
        -5_000_000_000_i64,
        -1,
        0,
        1,
        5_000_000_000_i64,
        i64::MAX,
    ] {
        let num = Expr::number(value);
        assert_eq!(Add::add(x, value), Add::add(&x_expr, &num));
        assert_eq!(Add::add(&x, value), Add::add(&x_expr, &num));
        assert_eq!(Sub::sub(x, value), Sub::sub(&x_expr, &num));
        assert_eq!(Sub::sub(&x, value), Sub::sub(&x_expr, &num));
        assert_eq!(Mul::mul(x, value), Mul::mul(&x_expr, &num));
        assert_eq!(Mul::mul(&x, value), Mul::mul(&x_expr, &num));
        assert_eq!(Div::div(x, value), Div::div(&x_expr, &num));
        assert_eq!(Div::div(&x, value), Div::div(&x_expr, &num));
    }
    for value in [i32::MIN, -7, 0, 1, 7, i32::MAX] {
        let num = Expr::number(value);
        assert_eq!(Add::add(value, x), Add::add(&num, &x_expr));
        assert_eq!(Add::add(value, &x), Add::add(&num, &x_expr));
        assert_eq!(Sub::sub(value, x), Sub::sub(&num, &x_expr));
        assert_eq!(Sub::sub(value, &x), Sub::sub(&num, &x_expr));
        assert_eq!(Mul::mul(value, x), Mul::mul(&num, &x_expr));
        assert_eq!(Mul::mul(value, &x), Mul::mul(&num, &x_expr));
        assert_eq!(Div::div(value, x), Div::div(&num, &x_expr));
        assert_eq!(Div::div(value, &x), Div::div(&num, &x_expr));
    }
}

#[test]
fn registry_names_and_multi_shard_distribution_preserve_every_binding() {
    let registry = SymbolRegistry::new();
    let mut expected: Vec<String> = (0..64)
        .map(|index| format!("shard_probe_{index}"))
        .collect();
    for name in &expected {
        let symbol = registry.register(name).expect("unique name");
        assert_eq!(registry.get(name), Some(symbol));
    }
    let _anon_a = registry.anonymous().expect("anonymous a");
    let _anon_b = registry.anonymous().expect("anonymous b");

    assert_eq!(registry.len(), 66);
    let mut expected_sorted = expected.clone();
    expected_sorted.sort_unstable();
    assert_eq!(registry.names(), expected_sorted);

    for name in expected.iter().take(32) {
        assert!(registry.remove(name));
        assert_eq!(registry.get(name), None);
    }
    assert_eq!(registry.len(), 34);
    let mut expected_remaining: Vec<String> = expected.drain(32..).collect();
    expected_remaining.sort_unstable();
    assert_eq!(registry.names(), expected_remaining);
    for name in &expected_remaining {
        assert!(registry.get(name).is_some());
    }
}

#[test]
fn symbol_calculus_shortcuts_delegate_to_expression_methods() {
    let context = Context::new().expect("context");
    let x = context.symb("calc_x").expect("local symbol");
    let y = context.symb("calc_y").expect("local symbol");
    assert_eq!(x.derivative(x, 1), Expr::number(1));
    assert_eq!(x.derivative(y, 1), Expr::number(0));
    assert_eq!(x.gradient(&[x, y]), vec![Expr::number(1), Expr::number(0)]);
    assert_eq!(
        x.hessian(&[x, y]),
        vec![
            vec![Expr::number(0), Expr::number(0)],
            vec![Expr::number(0), Expr::number(0)],
        ]
    );
}

#[test]
fn context_anon_stays_local_and_isolates_from_global_index() {
    let mut context = Context::new().expect("context");
    let first = context.anon().expect("first anonymous");
    let second = context.anon().expect("second anonymous");
    assert_ne!(first, second);
    assert_eq!(first.as_str(), None);
    assert_eq!(format!("{first}"), format!("${}", first.id()));
    assert_eq!(context.symbol_from_id(first.id()), Ok(first));
    assert_eq!(
        Symbol::from_id(first.id()),
        Err(SymbolError::InvalidId(first.id()))
    );
    assert_eq!(context.symbol_names(), Vec::<String>::new());
    assert_eq!(context.symbol_count(), 0);

    let first_id = first.id();
    let retained = Add::add(first, 1);
    context.reset().expect("reset");
    assert_eq!(
        context.symbol_from_id(first_id),
        Err(SymbolError::InvalidId(first_id))
    );
    assert_eq!(format!("{retained}"), format!("1 + ${first_id}"));
}
