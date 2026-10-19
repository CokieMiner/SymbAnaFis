use alloc::{format, string::String};

use proptest::prelude::{prop, prop_assert, prop_assert_eq, proptest};

use crate::{
    core::{Context, Symbol},
    errors::SymbolError,
};

use super::super::{SymbolData, SymbolIndex, SymbolRegistry};

#[test]
fn removing_by_id_preserves_replacements_and_local_ownership() {
    let registry = SymbolRegistry::new();
    let first = registry.intern("by_id").expect("global name");
    let anonymous = registry.anonymous().expect("anonymous identity");
    let local = SymbolData::local("by_id".into()).expect("local name");
    assert!(!registry.remove_id(local.id()));
    assert!(registry.remove_id(first.id()));
    let replacement = registry.intern("by_id").expect("replacement name");
    assert!(!registry.remove_id(first.id()));
    assert!(SymbolData::lookup(replacement.id()).is_ok());
    assert!(registry.remove_id(anonymous.id()));
    assert!(SymbolData::lookup(anonymous.id()).is_ok());
    drop(registry);
    assert!(SymbolData::lookup(local.id()).is_ok());
}

#[test]
fn global_removal_and_drop_preserve_context_symbols_and_retained_expressions() {
    let context = Context::new();
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
    assert!(SymbolData::lookup(global.id()).is_ok());
    assert_eq!(local.to_expr().as_symbol(), Some(local));
    assert_eq!(
        global_expr.symbol_name(),
        Some("registry_independent_local")
    );
    let another = registry
        .intern("registry_drop_id")
        .expect("global registration");
    drop(registry);
    assert!(SymbolData::lookup(another.id()).is_ok());
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
    drop(data);
    drop(registry);
    assert_eq!(Symbol::from_id(symbol.id()), Ok(symbol));
    assert!((symbol + 1).contains_symbol(symbol));
    assert_eq!(symbol.name().as_deref(), Some("checked_raw_symbol"));
}

proptest! {
    #[test]
    fn generated_local_metadata_has_weak_directory_lifetime(
        suffix in prop::collection::vec(prop::char::range('a', 'z'), 1..=24),
    ) {
        let name = format!("property_local_{}", suffix.into_iter().collect::<String>());
        let directory = SymbolIndex::new();
        let data = SymbolData::local(name.into()).expect("bounded local identity");
        let id = data.id();
        directory.publish_local(&data);

        prop_assert!(directory.lookup(id).is_some());
        prop_assert!(directory.lookup_permanent(id).is_none());
        prop_assert_eq!(Symbol::from_id(id), Err(SymbolError::InvalidId(id)));

        drop(data);
        prop_assert!(directory.lookup(id).is_none());
        prop_assert!(SymbolData::lookup(id).is_err());
    }
}
