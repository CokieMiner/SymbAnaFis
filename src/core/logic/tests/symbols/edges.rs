//! Local and global symbol metadata: index membership, removal, and raw identities.

use alloc::{format, string::String};

use proptest::prelude::{prop, prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Symbol},
    errors::SymbolError,
};

use super::super::super::symbols::{SymbolData, SymbolRegistry};

proptest! {
    #[test]
    fn generated_local_metadata_stays_out_of_the_global_index(
        suffix in prop::collection::vec(prop::char::range('a', 'z'), 1..=24),
    ) {
        let name = format!("property_local_{}", suffix.into_iter().collect::<String>());
        let data = SymbolData::local(name.into()).expect("bounded local identity");
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
    let local = SymbolData::local("by_id".into()).expect("local name");
    assert!(!registry.unregister(&local));
    assert!(registry.unregister(first));
    let replacement = registry.intern("by_id").expect("replacement name");
    assert!(!registry.unregister(first));
    assert!(SymbolData::lookup_permanent(replacement.id()).is_ok());
    assert!(registry.unregister(anonymous));
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
    assert_eq!(symbol.name().as_deref(), Some("checked_raw_symbol"));
}
