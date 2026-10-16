use core::panic::AssertUnwindSafe;
use std::panic::catch_unwind;

use alloc::format;

use crate::core::Symbol;

use super::super::{SymbolData, SymbolIndex, SymbolRegistry};

#[test]
#[expect(
    clippy::panic,
    reason = "the isolated directory test deliberately unwinds under its guard to verify unconditional unlock"
)]
fn directory_unlocks_after_unwinding_without_losing_published_metadata() {
    let directory = SymbolIndex::new();
    let data = SymbolData::local("unwound_identity_directory".into()).expect("identity");
    directory.publish_local(&data);
    let worker = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let _guard = directory.shard(data.id()).lock();
                panic!("unwind under this isolated directory guard");
            })
            .join()
    });
    assert!(worker.is_err(), "the isolated directory scope unwound");
    let recovered = directory.lookup(data.id()).expect("published metadata");
    assert!(alloc::sync::Arc::ptr_eq(&data, &recovered));
    directory.publish_local(&data);
    assert!(alloc::sync::Arc::ptr_eq(
        &data,
        &directory.lookup(data.id()).expect("recovered write lock")
    ));
    let another = SymbolData::local("recovered_identity_publication".into()).expect("identity");
    directory.publish_local(&another);
    assert!(directory.lookup(another.id()).is_some());
    assert!(directory.lookup(0).is_none());
}

#[test]
#[expect(
    clippy::panic,
    reason = "the isolated registry deliberately unwinds under its guards to verify mutation and lookup remain available"
)]
fn unwound_registry_queries_and_cleanup_preserve_issued_handles() {
    let registry = SymbolRegistry::new();
    let named = registry
        .intern("registry_unwind_name")
        .expect("named identity");
    let anonymous = registry.anonymous().expect("anonymous identity");
    for shard in &registry.shards {
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _guard = shard.lock();
                panic!("unwind under this isolated registry guard");
            }))
            .is_err(),
            "the isolated guarded scope unwound"
        );
    }
    assert_eq!(registry.len(), 2);
    assert_eq!(registry.names(), ["registry_unwind_name"]);
    assert_eq!(
        registry
            .intern("registry_unwind_name")
            .expect("usable lock")
            .id(),
        named.id()
    );
    let subsequent = registry.anonymous().expect("usable publication lock");
    assert!(registry.remove_id(subsequent.id()));
    assert!(registry.remove("registry_unwind_name"));
    assert!(registry.remove_id(anonymous.id()));
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.names().len(), 0);
    assert!(Symbol::from_id(named.id()).is_ok());
    assert!(Symbol::from_id(anonymous.id()).is_ok());
}

#[test]
fn partial_registry_removal_releases_peak_binding_capacity() {
    let registry = SymbolRegistry::new();
    let identities: alloc::vec::Vec<_> = (0..2048)
        .map(|index| {
            registry
                .intern(&format!("registry_capacity_{index}"))
                .expect("burst identity")
        })
        .collect();
    let anchor = identities
        .iter()
        .max_by_key(|data| {
            registry
                .name_shard(data.name().expect("named identity"))
                .lock()
                .names
                .capacity()
        })
        .expect("one retained identity");
    let name = anchor.name().expect("named anchor");
    let initial = registry.name_shard(name).lock().names.capacity();
    assert!(
        initial > 64,
        "the burst grew at least one of sixteen shards"
    );
    for data in &identities {
        if data.id() != anchor.id() {
            assert!(registry.remove(data.name().expect("named identity")));
        }
    }
    assert_eq!(registry.len(), 1);
    assert_eq!(
        registry.get(name).expect("retained binding").id(),
        anchor.id()
    );
    let remaining = registry.name_shard(name).lock().names.capacity();
    assert!(
        remaining <= 64,
        "partial deletion releases excess binding storage"
    );
}
